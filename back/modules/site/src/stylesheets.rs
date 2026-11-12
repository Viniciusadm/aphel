use std::collections::VecDeque;
use std::sync::{LazyLock, Mutex};

use aphel_sdk::platform::api::state::AppState;
use aphel_sdk::platform::tenancy::TenantLocal;
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use sha2::{Digest, Sha256};

pub(crate) const STYLES_PREFIX: &str = "/styles/";
pub(crate) const FRAME_PATH: &str = "/frame.css";
pub(crate) const BLOCKS_PATH: &str = "/blocks.css";
pub(crate) const PLUGINS_PREFIX: &str = "/plugins/";
const PUBLIC_BASE: &str = "/_aphel/site";
const FRAME_CSS: &str = include_str!("../theme/frame.css");
const BLOCKS_CSS: &str = include_str!("../theme/blocks/blocks.css");
const KEPT: usize = 32;
const IMMUTABLE: &str = "public, max-age=31536000, immutable";
const SHORT: &str = "public, max-age=60";
const CSS: &str = "text/css; charset=utf-8";

static CACHE: TenantLocal<Mutex<VecDeque<(String, String)>>> = TenantLocal::new(Mutex::default);
static FRAME_VERSION: LazyLock<String> = LazyLock::new(|| digest(FRAME_CSS));
static BLOCKS_VERSION: LazyLock<String> = LazyLock::new(|| digest(BLOCKS_CSS));

fn digest(text: &str) -> String {
    let hash = Sha256::digest(text.as_bytes());
    hash.iter()
        .take(8)
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

pub(crate) fn frame_url() -> String {
    format!("{PUBLIC_BASE}{FRAME_PATH}?v={}", *FRAME_VERSION)
}

pub(crate) fn blocks_url() -> String {
    format!("{PUBLIC_BASE}{BLOCKS_PATH}?v={}", *BLOCKS_VERSION)
}

pub(crate) fn plugin_url(owner: &str, css: &str) -> String {
    format!("{PUBLIC_BASE}{PLUGINS_PREFIX}{owner}.css?v={}", digest(css))
}

pub(crate) fn plugin(css: Option<&String>) -> Response {
    match css {
        Some(css) => stylesheet(css.clone(), IMMUTABLE),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

pub(crate) fn register(css: String) -> String {
    let hash = digest(&css);
    if let Some(cache) = CACHE.get() {
        let mut cache = cache
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if !cache.iter().any(|(key, _)| *key == hash) {
            cache.push_back((hash.clone(), css));
            while cache.len() > KEPT {
                cache.pop_front();
            }
        }
    }
    format!("{PUBLIC_BASE}{STYLES_PREFIX}{hash}.css")
}

fn stylesheet(body: String, cache_control: &'static str) -> Response {
    let mut response = (StatusCode::OK, body).into_response();
    let headers = response.headers_mut();
    headers.insert(header::CONTENT_TYPE, HeaderValue::from_static(CSS));
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

pub(crate) fn frame() -> Response {
    stylesheet(FRAME_CSS.to_owned(), IMMUTABLE)
}

pub(crate) fn blocks() -> Response {
    stylesheet(BLOCKS_CSS.to_owned(), IMMUTABLE)
}

pub(crate) async fn generated(state: &AppState, name: &str) -> Response {
    let hash = name.strip_suffix(".css").unwrap_or(name);
    let cached = CACHE.get().and_then(|cache| {
        let cache = cache
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        cache
            .iter()
            .find(|(key, _)| key == hash)
            .map(|(_, css)| css.clone())
    });
    if let Some(css) = cached {
        return stylesheet(css, IMMUTABLE);
    }
    match crate::appearance::resolved(state, None).await {
        Ok(appearance) => stylesheet(
            aphel_contract_site::styles::compile_css(&appearance["styles"]),
            SHORT,
        ),
        Err(error) => {
            tracing::warn!(%error, "the site stylesheet could not be built");
            StatusCode::SERVICE_UNAVAILABLE.into_response()
        }
    }
}
