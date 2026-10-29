use std::sync::LazyLock;
use std::time::Duration;

use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{Html, IntoResponse, Response};
use serde_json::{Map, Value};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::model::{HONEYPOT_FIELD, LeadFormPublicDefinition, LeadFormSubmitResult, public_field};
use crate::render::{self, PageState};
use crate::store::{self, StoredForm, public_path};
use crate::submit::{self, INVALID_SUBMISSION, SubmissionContext};
use crate::validate::normalize_origin;
use crate::{TAG, error_fields, error_status, form_not_found};
use aphel_kernel::Ctx;
use aphel_kernel::api::dto::common::{Data, ErrorEnvelope};
use aphel_kernel::api::error::ApiError;
use aphel_kernel::api::extractors::ClientInfo;
use aphel_kernel::api::rate_limit::SharedRateLimiter;
use aphel_kernel::domain::access::AccessScope;
use aphel_kernel::errors::AppError;
use aphel_kernel::state::AppState;

pub(crate) const ORIGIN_NOT_ALLOWED: &str = "lead_form_origin_not_allowed";
const VIEW_LIMIT: u32 = 120;
const SUBMIT_PER_ADDRESS_LIMIT: u32 = 5;
const SUBMIT_PER_FORM_LIMIT: u32 = 300;
const MINUTE: Duration = Duration::from_secs(60);
const HOUR: Duration = Duration::from_secs(3600);

struct Limits {
    views: SharedRateLimiter,
    submits_per_address: SharedRateLimiter,
    submits_per_form: SharedRateLimiter,
}

static LIMITS: LazyLock<Limits> = LazyLock::new(|| Limits {
    views: SharedRateLimiter::new("lead_forms.views", VIEW_LIMIT, MINUTE),
    submits_per_address: SharedRateLimiter::new(
        "lead_forms.submits_per_address",
        SUBMIT_PER_ADDRESS_LIMIT,
        MINUTE,
    ),
    submits_per_form: SharedRateLimiter::new(
        "lead_forms.submits_per_form",
        SUBMIT_PER_FORM_LIMIT,
        HOUR,
    ),
});

pub(crate) fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(definition))
        .routes(routes!(submit))
        .routes(routes!(hosted_page))
        .routes(routes!(embed_script))
}

async fn limit(limiter: &SharedRateLimiter, key: &str) -> Result<(), AppError> {
    limiter
        .check(
            key,
            "muitas tentativas em sequência; aguarde um pouco e tente novamente",
        )
        .await
}

fn address(ip: &Option<String>) -> &str {
    ip.as_deref().unwrap_or("unknown")
}

async fn resolve(ctx: &Ctx<'_>, public_id: &str) -> Result<StoredForm, AppError> {
    store::find_by_public_id(ctx.pool(), public_id)
        .await?
        .filter(|form| form.is_active)
        .ok_or_else(form_not_found)
}

fn header_text(headers: &HeaderMap, name: header::HeaderName) -> Option<&str> {
    headers.get(name).and_then(|value| value.to_str().ok())
}

fn origin_of(url: &str) -> Option<String> {
    let (scheme, rest) = url.split_once("://")?;
    let authority = rest.split(['/', '?', '#']).next()?;
    normalize_origin(&format!("{scheme}://{authority}"))
}

fn origin_allowed(ctx: &Ctx<'_>, form: &StoredForm, headers: &HeaderMap) -> bool {
    let allowed = &form.definition.allowed_origins;
    if allowed.is_empty() {
        return true;
    }
    let Some(origin) = header_text(headers, header::ORIGIN) else {
        return true;
    };
    let Some(origin) = normalize_origin(origin) else {
        return false;
    };
    if allowed.contains(&origin)
        || origin_of(&ctx.state().auth_settings.app_base_url.get()).as_ref() == Some(&origin)
    {
        return true;
    }
    header_text(headers, header::HOST).is_some_and(|authority| {
        origin
            .split_once("://")
            .is_some_and(|(_, rest)| rest.eq_ignore_ascii_case(authority))
    })
}

fn require_origin(ctx: &Ctx<'_>, form: &StoredForm, headers: &HeaderMap) -> Result<(), AppError> {
    if origin_allowed(ctx, form, headers) {
        Ok(())
    } else {
        Err(AppError::forbidden_code(
            ORIGIN_NOT_ALLOWED,
            "este formulário não aceita envios a partir deste site",
        ))
    }
}

fn frame_ancestors(form: &StoredForm) -> String {
    if form.definition.allowed_origins.is_empty() {
        "*".to_owned()
    } else {
        format!("'self' {}", form.definition.allowed_origins.join(" "))
    }
}

fn html_response(status: StatusCode, body: String, ancestors: &str) -> Response {
    let mut response = (status, Html(body)).into_response();
    let headers = response.headers_mut();
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("strict-origin-when-cross-origin"),
    );
    let policy = format!(
        "default-src 'none'; style-src 'unsafe-inline'; base-uri 'none'; frame-ancestors {ancestors}"
    );
    if let Ok(value) = HeaderValue::from_str(&policy) {
        headers.insert(header::CONTENT_SECURITY_POLICY, value);
    }
    response
}

#[utoipa::path(
    get,
    path = "/public/lead-forms/forms/{public_id}",
    tag = TAG,
    operation_id = "lead_forms_public_definition",
    params(("public_id" = String, Path, description = "Public id of the form")),
    responses(
        (status = 200, description = "Public definition of an active form, used by `embed.js` to render it. No authentication", body = Data<LeadFormPublicDefinition>),
        (status = 403, description = "The request `Origin` is not in the form allowed origins (`lead_form_origin_not_allowed`)", body = ErrorEnvelope),
        (status = 404, description = "Unknown or inactive form, or the module is disabled (`lead_form_not_found`)", body = ErrorEnvelope),
        (status = 429, description = "Too many requests from this address (`rate_limited`)", body = ErrorEnvelope),
    )
)]
async fn definition(
    State(state): State<AppState>,
    ClientInfo(client): ClientInfo,
    Path(public_id): Path<String>,
    headers: HeaderMap,
) -> Result<axum::Json<Data<LeadFormPublicDefinition>>, ApiError> {
    limit(&LIMITS.views, &format!("view:{}", address(&client.ip))).await?;
    let scope = AccessScope::system();
    let ctx = Ctx::from_state(&state, &scope).await?;
    let form = resolve(&ctx, &public_id).await?;
    require_origin(&ctx, &form, &headers)?;
    let definition = &form.definition;
    Ok(axum::Json(Data {
        data: LeadFormPublicDefinition {
            submit_path: public_path(&form.public_id, "/submissions"),
            public_id: form.public_id.clone(),
            name: form.name.clone(),
            fields: definition.fields.iter().map(public_field).collect(),
            submit_label: definition.submit_label.clone(),
            success_message: definition.success_message.clone(),
            redirect_url: definition.redirect_url.clone(),
            honeypot_field: HONEYPOT_FIELD.to_owned(),
        },
    }))
}

#[utoipa::path(
    get,
    path = "/public/lead-forms/forms/{public_id}/page",
    tag = TAG,
    operation_id = "lead_forms_public_page",
    params(("public_id" = String, Path, description = "Public id of the form")),
    responses(
        (status = 200, description = "Self-contained HTML page with the form, suitable for an iframe. `Content-Security-Policy: frame-ancestors` follows the form allowed origins", content_type = "text/html", body = String),
        (status = 404, description = "Unknown or inactive form, or the module is disabled", content_type = "text/html", body = String),
        (status = 429, description = "Too many requests from this address", content_type = "text/html", body = String),
    )
)]
async fn hosted_page(
    State(state): State<AppState>,
    ClientInfo(client): ClientInfo,
    Path(public_id): Path<String>,
) -> Response {
    if let Err(error) = limit(&LIMITS.views, &format!("view:{}", address(&client.ip))).await {
        return html_response(error_status(&error), render::unavailable_page(), "*");
    }
    let scope = AccessScope::system();
    let resolved = match Ctx::from_state(&state, &scope).await {
        Ok(ctx) => resolve(&ctx, &public_id).await,
        Err(error) => Err(error),
    };
    match resolved {
        Ok(form) => html_response(
            StatusCode::OK,
            render::page(&form, &PageState::blank()),
            &frame_ancestors(&form),
        ),
        Err(error) => html_response(error_status(&error), render::unavailable_page(), "*"),
    }
}

#[utoipa::path(
    get,
    path = "/public/lead-forms/forms/{public_id}/embed.js",
    tag = TAG,
    operation_id = "lead_forms_public_embed",
    params(("public_id" = String, Path, description = "Public id of the form")),
    responses(
        (status = 200, description = "Script that renders the form inside `<div data-aphel-form=\"<public_id>\">` and submits it with `fetch`", content_type = "application/javascript", body = String),
        (status = 404, description = "Unknown or inactive form, or the module is disabled (`lead_form_not_found`)", body = ErrorEnvelope),
        (status = 429, description = "Too many requests from this address (`rate_limited`)", body = ErrorEnvelope),
    )
)]
async fn embed_script(
    State(state): State<AppState>,
    ClientInfo(client): ClientInfo,
    Path(public_id): Path<String>,
) -> Result<Response, ApiError> {
    limit(&LIMITS.views, &format!("view:{}", address(&client.ip))).await?;
    let scope = AccessScope::system();
    let ctx = Ctx::from_state(&state, &scope).await?;
    let form = resolve(&ctx, &public_id).await?;
    let script = render::embed_script(&form.public_id)?;
    let mut response = script.into_response();
    let headers = response.headers_mut();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/javascript; charset=utf-8"),
    );
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=300"),
    );
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    Ok(response)
}

struct Failure {
    form: Option<StoredForm>,
    payload: Map<String, Value>,
    error: AppError,
}

impl Failure {
    fn early(error: AppError) -> Self {
        Self {
            form: None,
            payload: Map::new(),
            error,
        }
    }
}

async fn process(
    ctx: &Ctx<'_>,
    ip: &Option<String>,
    public_id: &str,
    headers: &HeaderMap,
    body: &[u8],
) -> Result<StoredForm, Failure> {
    limit(
        &LIMITS.submits_per_address,
        &format!("submit:{public_id}:{}", address(ip)),
    )
    .await
    .map_err(Failure::early)?;
    let form = resolve(ctx, public_id).await.map_err(Failure::early)?;
    let with_form = |error: AppError, payload: Map<String, Value>| Failure {
        form: Some(form.clone()),
        payload,
        error,
    };
    limit(
        &LIMITS.submits_per_form,
        &format!("submit_form:{public_id}"),
    )
    .await
    .map_err(|error| with_form(error, Map::new()))?;
    require_origin(ctx, &form, headers).map_err(|error| with_form(error, Map::new()))?;
    let content_type = header_text(headers, header::CONTENT_TYPE).unwrap_or_default();
    let payload =
        submit::parse_payload(content_type, body).map_err(|error| with_form(error, Map::new()))?;
    let context = SubmissionContext {
        ip_address: ip.clone(),
        origin: header_text(headers, header::ORIGIN)
            .map(|origin| origin.chars().take(255).collect()),
    };
    if let Some(trap) = submit::honeypot(&payload) {
        submit::record_spam(ctx, &form, &payload, trap, &context)
            .await
            .map_err(|error| with_form(error, Map::new()))?;
        return Ok(form);
    }
    let values = match submit::validate_values(&form.definition.fields, &payload) {
        Ok(values) => values,
        Err(violations) => {
            return Err(with_form(
                AppError::invalid_fields_code(
                    INVALID_SUBMISSION,
                    "revise os campos destacados",
                    violations,
                ),
                payload,
            ));
        }
    };
    submit::accept(ctx, &form, &payload, &values, &context)
        .await
        .map_err(|error| with_form(error, payload.clone()))?;
    Ok(form)
}

fn wants_html(headers: &HeaderMap) -> bool {
    let form_post = header_text(headers, header::CONTENT_TYPE).is_some_and(|value| {
        value
            .to_ascii_lowercase()
            .starts_with("application/x-www-form-urlencoded")
    });
    let accepts_html =
        header_text(headers, header::ACCEPT).is_some_and(|value| value.contains("text/html"));
    form_post && accepts_html
}

fn html_message(error: &AppError) -> String {
    let status = error_status(error);
    if status.is_server_error() && status != StatusCode::SERVICE_UNAVAILABLE {
        "Não foi possível enviar agora. Tente novamente em instantes.".to_owned()
    } else {
        error.client_message().to_owned()
    }
}

#[utoipa::path(
    post,
    path = "/public/lead-forms/forms/{public_id}/submissions",
    tag = TAG,
    operation_id = "lead_forms_public_submit",
    params(("public_id" = String, Path, description = "Public id of the form")),
    request_body(
        description = "Values by form field key (up to 64 KB). The honeypot field `_aphel_hp` must stay empty",
        content(
            (Object = "application/json"),
            (Object = "application/x-www-form-urlencoded")
        )
    ),
    responses(
        (status = 200, description = "Submission accepted: creates the contact (and the deal), publishes `lead_form.submitted`. A filled honeypot also answers 200 without creating anything (recorded as spam). An HTML form post (`Accept: text/html`) gets the hosted page with the success message or a 303 to the redirect URL", body = Data<LeadFormSubmitResult>),
        (status = 303, description = "HTML form post with a redirect URL configured"),
        (status = 403, description = "The request `Origin` is not allowed (`lead_form_origin_not_allowed`)", body = ErrorEnvelope),
        (status = 404, description = "Unknown or inactive form, or the module is disabled (`lead_form_not_found`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid values (`lead_form_invalid_submission` with `details.fields` by field key), malformed body (`lead_form_invalid_payload`) or body above 64 KB (`lead_form_payload_too_large`)", body = ErrorEnvelope),
        (status = 429, description = "Too many submissions from this address or to this form (`rate_limited`)", body = ErrorEnvelope),
        (status = 503, description = "The acting user is no longer able to create the records (`lead_form_owner_unavailable`) or the CRM refused the contact (`lead_form_submission_failed`); recorded as a failed submission", body = ErrorEnvelope),
    )
)]
async fn submit(
    State(state): State<AppState>,
    ClientInfo(client): ClientInfo,
    Path(public_id): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let html = wants_html(&headers);
    let scope = AccessScope::system();
    let ctx = match Ctx::from_state(&state, &scope).await {
        Ok(ctx) => ctx,
        Err(error) => return ApiError(error).into_response(),
    };
    match process(&ctx, &client.ip, &public_id, &headers, &body).await {
        Ok(form) if html => match &form.definition.redirect_url {
            Some(url) => match HeaderValue::from_str(url) {
                Ok(location) => {
                    let mut response = StatusCode::SEE_OTHER.into_response();
                    response.headers_mut().insert(header::LOCATION, location);
                    response
                }
                Err(_) => success_page(&form),
            },
            None => success_page(&form),
        },
        Ok(form) => axum::Json(Data {
            data: LeadFormSubmitResult {
                status: "accepted".to_owned(),
                success_message: form.definition.success_message.clone(),
                redirect_url: form.definition.redirect_url.clone(),
            },
        })
        .into_response(),
        Err(failure) if html => {
            let status = error_status(&failure.error);
            match &failure.form {
                Some(form) => {
                    let message = html_message(&failure.error);
                    html_response(
                        status,
                        render::page(
                            form,
                            &PageState {
                                values: &failure.payload,
                                errors: error_fields(&failure.error),
                                message: Some(&message),
                                success: false,
                            },
                        ),
                        &frame_ancestors(form),
                    )
                }
                None => html_response(status, render::unavailable_page(), "*"),
            }
        }
        Err(failure) => ApiError(failure.error).into_response(),
    }
}

fn success_page(form: &StoredForm) -> Response {
    html_response(
        StatusCode::OK,
        render::page(
            form,
            &PageState {
                success: true,
                ..PageState::blank()
            },
        ),
        &frame_ancestors(form),
    )
}
