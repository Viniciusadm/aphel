use aphel_contract_site::{SITE_API_HEADER, SiteApiHandler, SiteApiRequest, SiteContentSource};
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::api::error::app_error;
use aphel_sdk::platform::api::extractors::ClientInfo;
use aphel_sdk::platform::api::state::AppState;
use axum::body::Bytes;
use axum::extract::{FromRequestParts, Request};
use axum::http::{HeaderMap, HeaderValue, Method, StatusCode, header};
use axum::response::{IntoResponse, Response};
use serde_json::Value;

pub(crate) const API_SEGMENT: &str = "api";
pub(crate) const ISLANDS_LOADER_PATH: &str = "/islands.js";
pub(crate) const PREVIEW_SCRIPT_PATH: &str = "/preview.js";
const ISLAND_EXTENSION: &str = ".js";
const MAX_BODY_BYTES: usize = 1024 * 1024;
const MAX_DATA_BYTES: usize = 2 * 1024 * 1024;
const LOADER_CACHE_CONTROL: &str = "public, max-age=3600";
const ISLAND_CACHE_CONTROL: &str = "public, max-age=300";
const PREVIEW_CACHE_CONTROL: &str = "no-cache";
const JAVASCRIPT: &str = "text/javascript; charset=utf-8";
const SITE_HEADER_VALUE: &str = "1";

fn javascript(body: Vec<u8>, cache_control: &'static str) -> Response {
    let mut response = (StatusCode::OK, body).into_response();
    let headers = response.headers_mut();
    headers.insert(header::CONTENT_TYPE, HeaderValue::from_static(JAVASCRIPT));
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static(cache_control),
    );
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    response
}

pub(crate) fn islands_loader() -> Response {
    javascript(
        include_str!("../theme/islands.js").as_bytes().to_vec(),
        LOADER_CACHE_CONTROL,
    )
}

pub(crate) fn preview_script() -> Response {
    javascript(
        include_str!("../theme/preview.js").as_bytes().to_vec(),
        PREVIEW_CACHE_CONTROL,
    )
}

pub(crate) async fn island(state: &AppState, rest: &str) -> Response {
    let Some((owner, file)) = rest.split_once('/') else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let Some(name) = file.strip_suffix(ISLAND_EXTENSION) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    for source in state.app.contributions::<dyn SiteContentSource>() {
        match source.island(owner, name).await {
            Ok(Some(content)) => return javascript(content, ISLAND_CACHE_CONTROL),
            Ok(None) => {}
            Err(error) => {
                tracing::warn!(%error, owner, name, "could not serve a site island");
                return StatusCode::NOT_FOUND.into_response();
            }
        }
    }
    StatusCode::NOT_FOUND.into_response()
}

fn changes_state(method: &Method) -> bool {
    !matches!(*method, Method::GET | Method::HEAD | Method::OPTIONS)
}

fn route_not_found() -> AppError {
    AppError::not_found_code("site_route_not_found", "this site route does not exist")
}

async fn forward(state: &AppState, request: SiteApiRequest) -> Result<Response, AppError> {
    for handler in state.app.contributions::<dyn SiteApiHandler>() {
        if let Some(response) = handler.handle(state, request.clone()).await? {
            return Ok(response);
        }
    }
    Err(route_not_found())
}

pub(crate) async fn api(state: &AppState, rest: &str, request: Request) -> Response {
    let Some((owner, path)) = rest.split_once('/') else {
        return app_error(route_not_found());
    };
    let (mut parts, body) = request.into_parts();
    if changes_state(&parts.method)
        && parts
            .headers
            .get(SITE_API_HEADER)
            .and_then(|value| value.to_str().ok())
            != Some(SITE_HEADER_VALUE)
    {
        return app_error(AppError::forbidden_code(
            "site_request_forbidden",
            "site requests that change data must be sent by the site scripts",
        ));
    }
    let body = match axum::body::to_bytes(body, MAX_BODY_BYTES).await {
        Ok(body) => body,
        Err(_) => {
            return (
                StatusCode::PAYLOAD_TOO_LARGE,
                "the request body is too large",
            )
                .into_response();
        }
    };
    let client = ClientInfo::from_request_parts(&mut parts, state)
        .await
        .ok()
        .and_then(|client| client.0.ip);
    let customer = crate::session::customer(state, &parts.headers).await;
    let request = SiteApiRequest {
        owner: owner.to_owned(),
        method: parts.method.clone(),
        path: format!("/{path}"),
        query: parts.uri.query().map(str::to_owned),
        headers: parts.headers.clone(),
        body,
        client,
        customer,
    };
    match forward(state, request).await {
        Ok(response) => response,
        Err(error) => app_error(error),
    }
}

pub(crate) async fn page_data(
    state: &AppState,
    owner: &str,
    path: &str,
    customer: Option<aphel_contract_site::SiteCustomer>,
) -> Result<Option<Value>, AppError> {
    let request = SiteApiRequest {
        owner: owner.to_owned(),
        method: Method::GET,
        path: path.to_owned(),
        query: None,
        headers: HeaderMap::new(),
        body: Bytes::new(),
        client: None,
        customer,
    };
    let response = match forward(state, request).await {
        Ok(response) => response,
        Err(error) => app_error(error),
    };
    let status = response.status();
    if status == StatusCode::NOT_FOUND {
        return Ok(None);
    }
    if !status.is_success() {
        return Err(AppError::Internal(format!(
            "the data route {path} of {owner} answered {status}"
        )));
    }
    let bytes = axum::body::to_bytes(response.into_body(), MAX_DATA_BYTES)
        .await
        .map_err(|_| AppError::Internal(format!("the data route {path} answered too much")))?;
    serde_json::from_slice(&bytes)
        .map(Some)
        .map_err(|_| AppError::Internal(format!("the data route {path} did not answer JSON")))
}

pub(crate) fn interactions() -> Response {
    javascript(
        include_str!("../theme/interactions.js").as_bytes().to_vec(),
        LOADER_CACHE_CONTROL,
    )
}
