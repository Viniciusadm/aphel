use axum::extract::{FromRequest, Request};
use axum::http::{StatusCode, header};
use bytes::Bytes;
use serde::de::DeserializeOwned;

use crate::api::error::ApiError;
use crate::api::payload_limit::payload_too_large;
use crate::errors::{AppError, FieldViolation};

#[derive(Debug, Clone, Copy, Default)]
pub struct JsonBody<T>(pub T);

impl<S, T> FromRequest<S> for JsonBody<T>
where
    S: Send + Sync,
    T: DeserializeOwned,
{
    type Rejection = ApiError;

    async fn from_request(request: Request, state: &S) -> Result<Self, Self::Rejection> {
        if !has_json_content_type(&request) {
            return Err(AppError::Validation(
                "Expected request with `Content-Type: application/json`".to_owned(),
            )
            .into());
        }
        let body =
            Bytes::from_request(request, state)
                .await
                .map_err(|rejection| match rejection.status() {
                    StatusCode::PAYLOAD_TOO_LARGE => payload_too_large(),
                    _ => AppError::Validation(rejection.body_text()),
                })?;
        Ok(Self(decode(&body)?))
    }
}

fn has_json_content_type(request: &Request) -> bool {
    request
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(';').next())
        .map(|essence| essence.trim().to_ascii_lowercase())
        .is_some_and(|essence| {
            essence
                .strip_prefix("application/")
                .is_some_and(|subtype| subtype == "json" || subtype.ends_with("+json"))
        })
}

fn decode<T: DeserializeOwned>(body: &[u8]) -> Result<T, AppError> {
    let mut deserializer = serde_json::Deserializer::from_slice(body);
    serde_path_to_error::deserialize(&mut deserializer).map_err(|error| {
        let path = field_path(&error.path().to_string());
        let inner = error.into_inner();
        if inner.is_data() && path != "." {
            AppError::invalid_fields(
                "the request body has invalid fields",
                vec![FieldViolation::new(path, inner.to_string())],
            )
        } else {
            AppError::Validation(format!("invalid JSON body: {inner}"))
        }
    })
}

fn field_path(path: &str) -> String {
    path.replace(']', "")
        .split('[')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>()
        .join(".")
        .replace("..", ".")
}
