use aphel_kernel::errors::AppError;
use aphel_sdk::platform::api::state::AppState;
use aphel_sdk::platform::application::security::SecureTokens;
use aphel_sdk::platform::infrastructure::auth::Sha256SecureTokens;
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use sqlx::MySqlPool;

pub(crate) const PREVIEW_SEGMENT: &str = "preview";
const PREVIEW_COOKIE: &str = "aphel_site_preview";
pub(crate) const PREVIEW_MINUTES: i64 = 30;
const END_SEGMENT: &str = "end";
const MAX_TOKEN_LENGTH: usize = 128;

pub(crate) struct Preview {
    pub theme: Option<String>,
}

fn cookie_token(headers: &HeaderMap) -> Option<String> {
    headers
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(';'))
        .filter_map(|pair| pair.trim().split_once('='))
        .find(|(name, _)| *name == PREVIEW_COOKIE)
        .map(|(_, value)| value.trim().to_owned())
        .filter(|value| !value.is_empty() && value.len() <= MAX_TOKEN_LENGTH)
}

pub(crate) async fn start(pool: &MySqlPool, theme: Option<&str>) -> Result<String, AppError> {
    sqlx::query("DELETE FROM site_previews WHERE expires_at < NOW(6)")
        .execute(pool)
        .await?;
    let issued = Sha256SecureTokens.issue()?;
    sqlx::query(
        "INSERT INTO site_previews (token_hash, theme, expires_at) VALUES (?, ?, NOW(6) + INTERVAL ? MINUTE)",
    )
    .bind(&issued.hash)
    .bind(theme)
    .bind(PREVIEW_MINUTES)
    .execute(pool)
    .await?;
    Ok(issued.token.expose().to_owned())
}

async fn find(pool: &MySqlPool, token: &str) -> Result<Option<Preview>, AppError> {
    let row: Option<(Option<String>,)> = sqlx::query_as(
        "SELECT theme FROM site_previews WHERE token_hash = ? AND expires_at > NOW(6)",
    )
    .bind(Sha256SecureTokens.digest(token))
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|(theme,)| Preview { theme }))
}

pub(crate) async fn active(state: &AppState, headers: &HeaderMap) -> Option<Preview> {
    let token = cookie_token(headers)?;
    let pool = state.db().ok()?;
    match find(&pool, &token).await {
        Ok(preview) => preview,
        Err(error) => {
            tracing::warn!(%error, "could not read the site preview");
            None
        }
    }
}

pub(crate) fn link(base_url: &str, token: &str) -> String {
    format!(
        "{}/_aphel/site/{PREVIEW_SEGMENT}/{token}",
        crate::handler::origin_of(base_url.trim_end_matches('/'))
    )
}

fn redirect_home(state: &AppState, cookie: HeaderValue) -> Response {
    let home = format!(
        "{}/",
        state
            .organization_config
            .snapshot()
            .site_base_url()
            .trim_end_matches('/')
    );
    let mut response = StatusCode::SEE_OTHER.into_response();
    let headers = response.headers_mut();
    if let Ok(location) = HeaderValue::from_str(&home) {
        headers.insert(header::LOCATION, location);
    }
    headers.insert(header::SET_COOKIE, cookie);
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

fn clear_cookie() -> HeaderValue {
    HeaderValue::from_static("aphel_site_preview=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0")
}

pub(crate) async fn serve(state: &AppState, rest: &str) -> Response {
    if rest == END_SEGMENT {
        return redirect_home(state, clear_cookie());
    }
    let Ok(pool) = state.db() else {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    };
    if rest.is_empty() || rest.len() > MAX_TOKEN_LENGTH {
        return StatusCode::NOT_FOUND.into_response();
    }
    match find(&pool, rest).await {
        Ok(Some(_)) => {
            let secure = match state.organization_config.snapshot().secure_cookies() {
                true => "; Secure",
                false => "",
            };
            match HeaderValue::from_str(&format!(
                "{PREVIEW_COOKIE}={rest}; Path=/; HttpOnly; SameSite=Lax; Max-Age={}{secure}",
                PREVIEW_MINUTES * 60
            )) {
                Ok(cookie) => redirect_home(state, cookie),
                Err(_) => StatusCode::NOT_FOUND.into_response(),
            }
        }
        Ok(None) => redirect_home(state, clear_cookie()),
        Err(error) => {
            tracing::warn!(%error, "could not read the site preview");
            StatusCode::SERVICE_UNAVAILABLE.into_response()
        }
    }
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

pub(crate) fn with_banner(html: String, theme_name: &str, locale: &str) -> String {
    let (label, leave) = match locale.starts_with("pt") {
        true => ("Pré-visualização do tema", "Sair"),
        false => ("Previewing the theme", "Leave"),
    };
    let banner = format!(
        "<div role=\"status\" style=\"position:fixed;left:50%;bottom:16px;transform:translateX(-50%);z-index:2147483647;background:#111827;color:#fff;padding:10px 18px;border-radius:999px;font:14px/1.4 system-ui,sans-serif;box-shadow:0 4px 16px rgba(0,0,0,.25)\">{label} <strong>{}</strong> · <a href=\"/_aphel/site/{PREVIEW_SEGMENT}/{END_SEGMENT}\" style=\"color:#fff;text-decoration:underline\">{leave}</a></div>",
        escape_html(theme_name)
    );
    match html.rfind("</body>") {
        Some(index) => {
            let mut html = html;
            html.insert_str(index, &banner);
            html
        }
        None => html + &banner,
    }
}
