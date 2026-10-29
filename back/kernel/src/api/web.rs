use axum::http::{StatusCode, Uri, header};
use axum::response::{IntoResponse, Response};
use rust_embed::Embed;

use crate::api::error::app_error;
use crate::errors::AppError;

#[derive(Embed)]
#[folder = "../../web/dist"]
#[allow_missing = true]
struct WebAssets;

const INDEX_FILE: &str = "index.html";
const HASHED_ASSETS_PREFIX: &str = "assets/";
const SERVICE_WORKER_FILE: &str = "sw.js";
const MANIFEST_FILE: &str = "manifest.webmanifest";

pub async fn serve_web(uri: Uri) -> Response {
    let path = uri.path().trim_start_matches('/');

    if path == "api" || path.starts_with("api/") {
        return app_error(AppError::NotFound("route was not found".to_owned()));
    }

    let requested_file = if path.is_empty() { INDEX_FILE } else { path };
    if let Some(response) = embedded_file(requested_file) {
        return response;
    }

    if looks_like_file(path) {
        return StatusCode::NOT_FOUND.into_response();
    }

    embedded_file(INDEX_FILE).unwrap_or_else(|| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            "frontend build not found; run `npm run build` in web/",
        )
            .into_response()
    })
}

fn embedded_file(path: &str) -> Option<Response> {
    let file = WebAssets::get(path)?;
    let content_type = match path {
        MANIFEST_FILE => "application/manifest+json".to_owned(),
        SERVICE_WORKER_FILE => "text/javascript; charset=utf-8".to_owned(),
        _ => file.metadata.mimetype().to_owned(),
    };
    let cache_control = if path.starts_with(HASHED_ASSETS_PREFIX) {
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
            header::HeaderValue::from_static("/"),
        );
    }
    Some(response)
}

fn looks_like_file(path: &str) -> bool {
    path.rsplit('/')
        .next()
        .is_some_and(|segment| segment.contains('.'))
}
