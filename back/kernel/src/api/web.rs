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
use crate::modules::{SiteMountPrefix, SiteRequestHandler};

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
    Site(String),
    Redirect(String),
    NotFound,
}

struct SiteClaim<'a> {
    prefix: &'a str,
    host: Option<&'a str>,
}

fn host_matches(request_host: Option<&str>, configured: &str) -> bool {
    request_host.is_some_and(|host| {
        let host = host.to_ascii_lowercase();
        host == configured || host.split(':').next() == Some(configured)
    })
}

fn mounted_under<'a>(path: &'a str, prefix: &str) -> Option<&'a str> {
    let rest = path.strip_prefix(prefix)?;
    (rest.is_empty() || rest.starts_with('/')).then_some(rest)
}

fn resolve(
    path: &str,
    request_host: Option<&str>,
    app_prefix: &str,
    site: Option<SiteClaim<'_>>,
) -> Target {
    if mounted_under(path, RECOVERY_APP_PATH).is_some() {
        return Target::App(RECOVERY_APP_PATH.to_owned());
    }
    if let Some(rest) = mounted_under(path, RESERVED_PREFIX) {
        return Target::Reserved(rest.trim_start_matches('/').to_owned());
    }
    let app = (app_prefix.is_empty() || mounted_under(path, app_prefix).is_some())
        .then_some(app_prefix.len());
    let site = site
        .filter(|site| {
            site.host
                .is_none_or(|host| host_matches(request_host, host))
        })
        .filter(|site| site.prefix.is_empty() || mounted_under(path, site.prefix).is_some());
    match (app, site) {
        (Some(app_length), Some(site))
            if site.prefix.len() > app_length
                || (site.prefix.len() == app_length && site.host.is_some()) =>
        {
            Target::Site(site.prefix.to_owned())
        }
        (Some(_), _) => Target::App(app_prefix.to_owned()),
        (None, Some(site)) => Target::Site(site.prefix.to_owned()),
        (None, None) if path == "/" => Target::Redirect(format!("{app_prefix}/")),
        (None, None) => Target::NotFound,
    }
}

pub async fn dispatch(state: AppState, mounted: Router, request: Request) -> Response {
    let path = request.uri().path().to_owned();
    if path == "/api" || path.starts_with("/api/") {
        return app_error(AppError::NotFound("route was not found".to_owned()));
    }
    let config = state.organization_config.snapshot();
    let handler = state
        .app
        .contributions::<dyn SiteRequestHandler>()
        .into_iter()
        .next();
    let request_host = request
        .headers()
        .get(header::HOST)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    let site = handler.as_ref().map(|_| SiteClaim {
        prefix: config.mounts.site.prefix(),
        host: config.mounts.site.host.as_deref(),
    });
    match resolve(&path, request_host.as_deref(), config.app_prefix(), site) {
        Target::Reserved(rest) => serve_reserved(&rest, config.app_prefix()),
        Target::App(prefix) => serve_app(mounted, prefix, request).await,
        Target::Site(prefix) => match handler {
            Some(handler) => serve_site(&state, handler.as_ref(), prefix, request).await,
            None => StatusCode::NOT_FOUND.into_response(),
        },
        Target::Redirect(location) => Redirect::temporary(&location).into_response(),
        Target::NotFound => StatusCode::NOT_FOUND.into_response(),
    }
}

fn strip_mount(prefix: &str, request: &mut Request) -> Result<(), StatusCode> {
    let uri = request.uri();
    let rest = uri
        .path()
        .strip_prefix(prefix)
        .filter(|rest| !rest.is_empty())
        .unwrap_or("/");
    let rewritten = match uri.query() {
        Some(query) => format!("{rest}?{query}"),
        None => rest.to_owned(),
    };
    let rewritten = rewritten
        .parse::<Uri>()
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    *request.uri_mut() = rewritten;
    Ok(())
}

async fn serve_site(
    state: &AppState,
    handler: &dyn SiteRequestHandler,
    prefix: String,
    mut request: Request,
) -> Response {
    if let Err(status) = strip_mount(&prefix, &mut request) {
        return status.into_response();
    }
    request.extensions_mut().insert(SiteMountPrefix(prefix));
    handler.serve(state, request).await
}

async fn serve_app(mounted: Router, prefix: String, mut request: Request) -> Response {
    if let Err(status) = strip_mount(&prefix, &mut request) {
        return status.into_response();
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
