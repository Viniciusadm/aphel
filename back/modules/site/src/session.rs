use aphel_contract_site::SiteCustomer;
use aphel_sdk::platform::api::state::AppState;
use axum::http::{HeaderMap, HeaderValue, header};

use crate::customers::{self, SESSION_COOKIE, SESSION_DAYS};

pub(crate) fn token(headers: &HeaderMap) -> Option<String> {
    headers
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(';'))
        .filter_map(|pair| pair.trim().split_once('='))
        .find(|(name, _)| *name == SESSION_COOKIE)
        .map(|(_, value)| value.trim().to_owned())
        .filter(|value| !value.is_empty() && value.len() <= 128)
}

pub(crate) async fn customer(state: &AppState, headers: &HeaderMap) -> Option<SiteCustomer> {
    let token = token(headers)?;
    let pool = state.db().ok()?;
    match customers::current(&pool, &token).await {
        Ok(customer) => customer,
        Err(error) => {
            tracing::warn!(%error, "could not read the site session");
            None
        }
    }
}

pub(crate) fn set_cookie(state: &AppState, token: &str) -> Option<HeaderValue> {
    let secure = if state.organization_config.snapshot().secure_cookies() {
        "; Secure"
    } else {
        ""
    };
    HeaderValue::from_str(&format!(
        "{SESSION_COOKIE}={token}; Path=/; HttpOnly; SameSite=Lax; Max-Age={}{secure}",
        SESSION_DAYS * 24 * 60 * 60
    ))
    .ok()
}

pub(crate) fn clear_cookie() -> HeaderValue {
    HeaderValue::from_static("aphel_site_session=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0")
}
