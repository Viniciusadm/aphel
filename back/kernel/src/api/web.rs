use axum::Router;
use axum::extract::Request;
use axum::http::{HeaderValue, StatusCode, Uri, header};
use axum::response::{IntoResponse, Redirect, Response};
use rust_embed::Embed;
use serde_json::Value;
use tower::ServiceExt;

use crate::api::error::app_error;
use crate::api::state::AppState;
use crate::application::organization_profile::{RECOVERY_APP_PATH, RESERVED_PREFIX};
use crate::errors::AppError;

#[derive(Embed)]
#[folder = "../../web/dist"]
#[allow_missing = true]
struct WebAssets;

const INDEX_FILE: &str = "index.html";
const HASHED_ASSETS_PREFIX: &str = "assets/";
const DOCS_PREFIX: &str = "docs";
const DOCS_HASHED_ASSETS_PREFIX: &str = "docs/_astro/";
const DOCS_NOT_FOUND_FILE: &str = "docs/404.html";
const LLMS_INDEX_PATH: &str = "llms.txt";
const LLMS_FULL_PATH: &str = "llms-full.txt";
const SERVICE_WORKER_FILE: &str = "sw.js";
const MANIFEST_FILE: &str = "manifest.webmanifest";
const BASE_META_PLACEHOLDER: &str = "<head>";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppMountPrefix(pub String);

#[derive(Debug, PartialEq, Eq)]
enum Target {
    Reserved(String),
    App(String),
    Redirect(String),
    NotFound,
}

fn mounted_under<'a>(path: &'a str, prefix: &str) -> Option<&'a str> {
    let rest = path.strip_prefix(prefix)?;
    (rest.is_empty() || rest.starts_with('/')).then_some(rest)
}

fn resolve(path: &str, app_prefix: &str) -> Target {
    if mounted_under(path, RECOVERY_APP_PATH).is_some() {
        return Target::App(RECOVERY_APP_PATH.to_owned());
    }
    if let Some(rest) = mounted_under(path, RESERVED_PREFIX) {
        return Target::Reserved(rest.trim_start_matches('/').to_owned());
    }
    if app_prefix.is_empty() || mounted_under(path, app_prefix).is_some() {
        return Target::App(app_prefix.to_owned());
    }
    if path == "/" {
        return Target::Redirect(format!("{app_prefix}/"));
    }
    Target::NotFound
}

pub async fn dispatch(state: AppState, mounted: Router, request: Request) -> Response {
    let path = request.uri().path().to_owned();
    if path == "/api" || path.starts_with("/api/") {
        return app_error(AppError::NotFound("route was not found".to_owned()));
    }
    let config = state.organization_config.snapshot();
    match resolve(&path, config.app_prefix()) {
        Target::Reserved(rest) => serve_reserved(&rest, config.app_prefix()),
        Target::App(prefix) => serve_app(mounted, prefix, request).await,
        Target::Redirect(location) => Redirect::temporary(&location).into_response(),
        Target::NotFound => StatusCode::NOT_FOUND.into_response(),
    }
}

async fn serve_app(mounted: Router, prefix: String, mut request: Request) -> Response {
    let uri = request.uri();
    let rest = uri
        .path()
        .strip_prefix(prefix.as_str())
        .filter(|rest| !rest.is_empty())
        .unwrap_or("/");
    let rewritten = match uri.query() {
        Some(query) => format!("{rest}?{query}"),
        None => rest.to_owned(),
    };
    match rewritten.parse::<Uri>() {
        Ok(rewritten) => *request.uri_mut() = rewritten,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    }
    request.extensions_mut().insert(AppMountPrefix(prefix));
    match mounted.oneshot(request).await {
        Ok(response) => response,
        Err(never) => match never {},
    }
}

pub async fn serve_app_shell(request: Request) -> Response {
    let prefix = request
        .extensions()
        .get::<AppMountPrefix>()
        .map(|prefix| prefix.0.clone())
        .unwrap_or_default();
    if looks_like_file(request.uri().path()) {
        return StatusCode::NOT_FOUND.into_response();
    }
    let Some(index) = WebAssets::get(INDEX_FILE) else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            "frontend build not found; run `npm run build` in web/",
        )
            .into_response();
    };
    let html = String::from_utf8_lossy(&index.data).replacen(
        BASE_META_PLACEHOLDER,
        &format!("{BASE_META_PLACEHOLDER}<meta name=\"aphel-base\" content=\"{prefix}\" />"),
        1,
    );
    (
        [
            (header::CONTENT_TYPE, "text/html; charset=utf-8"),
            (header::CACHE_CONTROL, "no-cache"),
        ],
        html,
    )
        .into_response()
}

fn serve_reserved(path: &str, app_prefix: &str) -> Response {
    let requested = match path {
        "" => return StatusCode::NOT_FOUND.into_response(),
        LLMS_INDEX_PATH => "docs/llms.txt",
        LLMS_FULL_PATH => "docs/llms-full.txt",
        MANIFEST_FILE => return manifest(app_prefix),
        other => other,
    };
    if let Some(response) = embedded_file(requested) {
        return response;
    }
    if is_docs_path(path) {
        return docs_page(path);
    }
    StatusCode::NOT_FOUND.into_response()
}

fn manifest(app_prefix: &str) -> Response {
    let Some(file) = WebAssets::get(MANIFEST_FILE) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let Ok(mut manifest) = serde_json::from_slice::<Value>(&file.data) else {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };
    let scope = format!("{app_prefix}/");
    if let Some(fields) = manifest.as_object_mut() {
        for field in ["id", "start_url", "scope"] {
            fields.insert(field.to_owned(), Value::String(scope.clone()));
        }
    }
    (
        [
            (header::CONTENT_TYPE, "application/manifest+json"),
            (header::CACHE_CONTROL, "no-cache"),
        ],
        manifest.to_string(),
    )
        .into_response()
}

fn embedded_file(path: &str) -> Option<Response> {
    let file = WebAssets::get(path)?;
    let content_type = match path {
        SERVICE_WORKER_FILE => "text/javascript; charset=utf-8".to_owned(),
        _ => file.metadata.mimetype().to_owned(),
    };
    let cache_control =
        if path.starts_with(HASHED_ASSETS_PREFIX) || path.starts_with(DOCS_HASHED_ASSETS_PREFIX) {
            "public, max-age=31536000, immutable"
        } else {
            "no-cache"
        };

    let mut response = (
        [
            (header::CONTENT_TYPE, content_type),
            (header::CACHE_CONTROL, cache_control.to_owned()),
        ],
        file.data,
    )
        .into_response();
    if path == SERVICE_WORKER_FILE {
        response.headers_mut().insert(
            header::HeaderName::from_static("service-worker-allowed"),
            HeaderValue::from_static("/"),
        );
    }
    Some(response)
}

fn is_docs_path(path: &str) -> bool {
    path == DOCS_PREFIX || path.starts_with("docs/")
}

fn docs_page(path: &str) -> Response {
    let directory = path.trim_end_matches('/');
    if let Some(response) = embedded_file(&format!("{directory}/index.html")) {
        return response;
    }
    let mut response =
        embedded_file(DOCS_NOT_FOUND_FILE).unwrap_or_else(|| StatusCode::NOT_FOUND.into_response());
    *response.status_mut() = StatusCode::NOT_FOUND;
    response
}

fn looks_like_file(path: &str) -> bool {
    path.rsplit('/')
        .next()
        .is_some_and(|segment| segment.contains('.'))
}
