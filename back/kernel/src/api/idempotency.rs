use axum::body::{Body, Bytes};
use axum::extract::{FromRequestParts, Request, State};
use axum::http::{HeaderName, HeaderValue, Method, StatusCode, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use chrono::Utc;
use futures::StreamExt;
use futures::stream;
use utoipa::openapi::path::{ParameterBuilder, ParameterIn};
use utoipa::openapi::schema::{ObjectBuilder, Type};
use utoipa::openapi::{ContentBuilder, Ref, RefOr, Required, ResponseBuilder};

use crate::api::csrf::PUBLIC_HOOKS_PREFIX;
use crate::api::error::app_error;
use crate::api::extractors::auth::{AuthContext, Credential};
use crate::api::state::AppState;
use crate::application::idempotency::{
    IDEMPOTENCY_KEY_IN_PROGRESS, IDEMPOTENCY_KEY_REUSED, IdempotencyDecision, IdempotencyKey,
    IdempotencyUseCases, IdempotentRequest, MAX_IDEMPOTENCY_KEY_LENGTH, MAX_STORED_RESPONSE_BYTES,
    StoredResponse,
};
use crate::application::security::SecureTokens;
use crate::database::Db;
use crate::errors::AppError;
use crate::infrastructure::auth::Sha256SecureTokens;
use crate::infrastructure::idempotency::MySqlIdempotencyStore;

pub const IDEMPOTENCY_KEY_HEADER: HeaderName = HeaderName::from_static("idempotency-key");
pub const IDEMPOTENT_REPLAYED_HEADER: HeaderName = HeaderName::from_static("idempotent-replayed");

const API_PREFIX: &str = "/api/v1/";
const EXCLUDED_PREFIXES: [&str; 3] = ["/api/v1/auth/", "/api/v1/installer", PUBLIC_HOOKS_PREFIX];
const MIN_REQUEST_BUFFER_BYTES: u64 = 2 * 1024 * 1024;
const MULTIPART_OVERHEAD_BYTES: u64 = 1024 * 1024;

pub async fn guard(State(state): State<AppState>, request: Request, next: Next) -> Response {
    let Some(raw_key) = request.headers().get(IDEMPOTENCY_KEY_HEADER).cloned() else {
        return next.run(request).await;
    };
    if request.method() != Method::POST || !applies_to(request.uri().path()) {
        return next.run(request).await;
    }
    let Some(pool) = state.db.clone() else {
        return next.run(request).await;
    };
    let (mut parts, body) = request.into_parts();
    let scope_hash = match AuthContext::from_request_parts(&mut parts, &state).await {
        Ok(context) => credential_scope(&context),
        Err(_) => return next.run(Request::from_parts(parts, body)).await,
    };
    let request = Request::from_parts(parts, body);
    let key = match raw_key
        .to_str()
        .map_err(|_| ())
        .and_then(|value| IdempotencyKey::parse(value).map_err(|_| ()))
    {
        Ok(key) => key,
        Err(()) => return app_error(invalid_key_error(raw_key.as_bytes())),
    };

    let limit = request_buffer_limit(state.max_upload_bytes());
    let (parts, body) = request.into_parts();
    let body = match buffer(body, limit).await {
        Ok(Buffered::Complete(bytes)) => bytes,
        Ok(Buffered::Overflow(body)) => {
            return next.run(Request::from_parts(parts, body)).await;
        }
        Err(error) => return app_error(error),
    };
    let path = parts
        .uri
        .path_and_query()
        .map(|value| value.as_str().to_owned())
        .unwrap_or_else(|| parts.uri.path().to_owned());
    let idempotent = IdempotentRequest::new(scope_hash, key, parts.method.as_str(), &path, &body);

    let use_cases = IdempotencyUseCases::new(MySqlIdempotencyStore::new(pool.clone()));
    match use_cases.begin(&idempotent, Utc::now()).await {
        Ok(IdempotencyDecision::Proceed) => {}
        Ok(IdempotencyDecision::Replay(stored)) => return replay(stored),
        Err(error) => return app_error(error),
    }

    let mut pending = PendingKey::new(pool, idempotent.clone());
    let response = next.run(Request::from_parts(parts, Body::from(body))).await;
    let status = response.status().as_u16();
    let (response_parts, response_body) = response.into_parts();

    if !StoredResponse::is_storable(status, 0) {
        pending.release_now(&use_cases).await;
        return Response::from_parts(response_parts, response_body);
    }

    let body = match buffer(response_body, MAX_STORED_RESPONSE_BYTES as u64).await {
        Ok(Buffered::Complete(bytes)) => bytes,
        Ok(Buffered::Overflow(body)) => {
            pending.release_now(&use_cases).await;
            return Response::from_parts(response_parts, body);
        }
        Err(error) => {
            pending.release_now(&use_cases).await;
            return app_error(error);
        }
    };
    let stored = StoredResponse {
        status,
        content_type: response_parts
            .headers
            .get(header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned),
        body: body.to_vec(),
    };
    pending.disarm();
    if let Err(error) = use_cases
        .finish(&idempotent, Some(stored), Utc::now())
        .await
    {
        tracing::warn!(error = %error, "could not store the idempotent response");
        let _ = use_cases.release(&idempotent).await;
    }
    Response::from_parts(response_parts, Body::from(body))
}

fn applies_to(path: &str) -> bool {
    path.starts_with(API_PREFIX)
        && !EXCLUDED_PREFIXES
            .iter()
            .any(|prefix| path.starts_with(prefix))
}

fn credential_scope(context: &AuthContext) -> String {
    let owner = match &context.credential {
        Credential::Session(_) => format!("user:{}", context.user.id.as_str()),
        Credential::ApiToken(token_id) => format!("token:{token_id}"),
    };
    Sha256SecureTokens.digest(&format!("idempotency:{owner}"))
}

fn invalid_key_error(raw: &[u8]) -> AppError {
    IdempotencyKey::parse(&String::from_utf8_lossy(raw))
        .err()
        .unwrap_or_else(|| {
            AppError::Validation(format!(
                "the Idempotency-Key header must have at most {MAX_IDEMPOTENCY_KEY_LENGTH} printable ASCII characters"
            ))
        })
}

fn request_buffer_limit(max_upload_bytes: u64) -> u64 {
    max_upload_bytes
        .saturating_add(MULTIPART_OVERHEAD_BYTES)
        .max(MIN_REQUEST_BUFFER_BYTES)
}

enum Buffered {
    Complete(Bytes),
    Overflow(Body),
}

async fn buffer(body: Body, limit: u64) -> Result<Buffered, AppError> {
    let mut stream = body.into_data_stream();
    let mut chunks: Vec<Bytes> = Vec::new();
    let mut size: u64 = 0;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk
            .map_err(|error| AppError::Validation(format!("could not read the body: {error}")))?;
        size = size.saturating_add(chunk.len() as u64);
        chunks.push(chunk);
        if size > limit {
            let prefix = stream::iter(chunks.into_iter().map(Ok::<Bytes, axum::Error>));
            return Ok(Buffered::Overflow(Body::from_stream(prefix.chain(stream))));
        }
    }
    let mut bytes = Vec::with_capacity(usize::try_from(size).unwrap_or_default());
    for chunk in chunks {
        bytes.extend_from_slice(&chunk);
    }
    Ok(Buffered::Complete(Bytes::from(bytes)))
}

fn replay(stored: StoredResponse) -> Response {
    let status = StatusCode::from_u16(stored.status).unwrap_or(StatusCode::OK);
    let mut response = (status, stored.body).into_response();
    let headers = response.headers_mut();
    headers.remove(header::CONTENT_TYPE);
    if let Some(value) = stored
        .content_type
        .and_then(|content_type| HeaderValue::from_str(&content_type).ok())
    {
        headers.insert(header::CONTENT_TYPE, value);
    }
    headers.insert(IDEMPOTENT_REPLAYED_HEADER, HeaderValue::from_static("true"));
    response
}

struct PendingKey {
    pool: Db,
    request: IdempotentRequest,
    armed: bool,
}

impl PendingKey {
    fn new(pool: Db, request: IdempotentRequest) -> Self {
        Self {
            pool,
            request,
            armed: true,
        }
    }

    fn disarm(&mut self) {
        self.armed = false;
    }

    async fn release_now(&mut self, use_cases: &IdempotencyUseCases<MySqlIdempotencyStore>) {
        self.disarm();
        if let Err(error) = use_cases.release(&self.request).await {
            tracing::warn!(error = %error, "could not release the idempotency key");
        }
    }
}

impl Drop for PendingKey {
    fn drop(&mut self) {
        if !self.armed {
            return;
        }
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        let store = MySqlIdempotencyStore::new(self.pool.clone());
        let request = self.request.clone();
        crate::tenancy::spawn(async move {
            if let Err(error) = IdempotencyUseCases::new(store).release(&request).await {
                tracing::warn!(error = %error, "could not release the idempotency key");
            }
        });
    }
}

pub fn document(openapi: &mut utoipa::openapi::OpenApi) {
    for (path, item) in openapi.paths.paths.iter_mut() {
        if !applies_to(path) {
            continue;
        }
        let Some(operation) = item.post.as_mut() else {
            continue;
        };
        operation
            .parameters
            .get_or_insert_with(Vec::new)
            .push(
                ParameterBuilder::new()
                    .name("Idempotency-Key")
                    .parameter_in(ParameterIn::Header)
                    .required(Required::False)
                    .description(Some(format!(
                        "Up to {MAX_IDEMPOTENCY_KEY_LENGTH} printable ASCII characters. The same key with the same request (method, path and body) within 24 hours answers the stored response with the same status and body plus `Idempotent-Replayed: true`. Scoped to the credential (session cookie or API token); ignored without a credential. 5xx and 429 responses are not stored"
                    )))
                    .schema(Some(
                        ObjectBuilder::new()
                            .schema_type(Type::String)
                            .max_length(Some(MAX_IDEMPOTENCY_KEY_LENGTH)),
                    ))
                    .build(),
            );
        let responses = &mut operation.responses.responses;
        append_response(
            responses,
            "409",
            format!(
                "The first request with this `Idempotency-Key` is still being processed (`{IDEMPOTENCY_KEY_IN_PROGRESS}`)"
            ),
        );
        append_response(
            responses,
            "422",
            format!(
                "`Idempotency-Key` already used with a different request (`{IDEMPOTENCY_KEY_REUSED}`) or longer than {MAX_IDEMPOTENCY_KEY_LENGTH} characters (`validation_error`)"
            ),
        );
    }
}

fn append_response(
    responses: &mut std::collections::BTreeMap<String, RefOr<utoipa::openapi::Response>>,
    status: &str,
    description: String,
) {
    match responses.get_mut(status) {
        Some(RefOr::T(response)) => {
            response.description = format!("{}. {description}", response.description);
        }
        Some(RefOr::Ref(_)) => {}
        None => {
            responses.insert(
                status.to_owned(),
                ResponseBuilder::new()
                    .description(description)
                    .content(
                        "application/json",
                        ContentBuilder::new()
                            .schema(Some(Ref::from_schema_name("ErrorEnvelope")))
                            .build(),
                    )
                    .build()
                    .into(),
            );
        }
    }
}
