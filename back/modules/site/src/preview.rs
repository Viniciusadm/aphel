use aphel_kernel::errors::AppError;
use aphel_sdk::platform::api::state::AppState;
use aphel_sdk::platform::application::security::SecureTokens;
use aphel_sdk::platform::infrastructure::auth::Sha256SecureTokens;
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use sqlx::MySqlPool;
use std::collections::BTreeMap;

pub(crate) const PREVIEW_SEGMENT: &str = "preview";
const PREVIEW_COOKIE: &str = "aphel_site_preview";
pub(crate) const PREVIEW_MINUTES: i64 = 30;
const END_SEGMENT: &str = "end";
const MAX_TOKEN_LENGTH: usize = 128;

const MAX_PATH_LENGTH: usize = 500;

pub(crate) struct Preview {
    pub theme: Option<String>,
    pub appearance: Option<crate::appearance::AppearancePreview>,
    pub record: Option<PreviewRecord>,
}

#[derive(Debug, Clone)]
pub(crate) struct PreviewRecord {
    pub record_type: String,
    pub record_id: String,
    pub fields: BTreeMap<String, String>,
    pub values: BTreeMap<String, serde_json::Value>,
}

impl Preview {
    pub fn draft_for(&self, record_type: &str) -> Option<crate::data::Draft<'_>> {
        self.record
            .as_ref()
            .filter(|record| record.record_type == record_type)
            .map(|record| crate::data::Draft {
                record_id: &record.record_id,
                fields: &record.fields,
                values: &record.values,
            })
    }

    pub fn shows_record(&self, record_type: &str, record_id: &str) -> bool {
        self.record.as_ref().is_some_and(|record| {
            record.record_type == record_type && record.record_id == record_id
        })
    }
}

pub(crate) fn local_path(path: &str) -> Option<&str> {
    let path = path.trim();
    (path.starts_with('/')
        && !path.starts_with("//")
        && path.len() <= MAX_PATH_LENGTH
        && !path
            .chars()
            .any(|character| character.is_control() || character == '\\'))
    .then_some(path)
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

pub(crate) async fn start(
    pool: &MySqlPool,
    theme: Option<&str>,
    record: Option<&PreviewRecord>,
    path: Option<&str>,
) -> Result<String, AppError> {
    sqlx::query("DELETE FROM site_previews WHERE expires_at < NOW(6)")
        .execute(pool)
        .await?;
    let issued = Sha256SecureTokens.issue()?;
    let fields = record
        .map(|record| serde_json::to_string(&record.fields))
        .transpose()
        .map_err(|error| AppError::Internal(error.to_string()))?;
    sqlx::query(
        "INSERT INTO site_previews (token_hash, theme, record_type, record_id, draft_fields, target_path, expires_at) \
         VALUES (?, ?, ?, ?, ?, ?, NOW(6) + INTERVAL ? MINUTE)",
    )
    .bind(&issued.hash)
    .bind(theme)
    .bind(record.map(|record| record.record_type.as_str()))
    .bind(record.map(|record| record.record_id.as_str()))
    .bind(fields)
    .bind(path)
    .bind(PREVIEW_MINUTES)
    .execute(pool)
    .await?;
    Ok(issued.token.expose().to_owned())
}

type PreviewRow = (
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
);

async fn find(
    pool: &MySqlPool,
    token: &str,
) -> Result<Option<(Preview, Option<String>)>, AppError> {
    let row: Option<PreviewRow> = sqlx::query_as(
        "SELECT theme, record_type, record_id, draft_fields, target_path FROM site_previews \
         WHERE token_hash = ? AND expires_at > NOW(6)",
    )
    .bind(Sha256SecureTokens.digest(token))
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|(theme, record_type, record_id, fields, path)| {
        let record = record_type
            .zip(record_id)
            .map(|(record_type, record_id)| PreviewRecord {
                record_type,
                record_id,
                fields: fields
                    .and_then(|fields| serde_json::from_str(&fields).ok())
                    .unwrap_or_default(),
                values: BTreeMap::new(),
            });
        (
            Preview {
                theme,
                record,
                appearance: None,
            },
            path,
        )
    }))
}

pub(crate) async fn active(state: &AppState, headers: &HeaderMap) -> Option<Preview> {
    let token = cookie_token(headers)?;
    let pool = state.db().ok()?;
    match find(&pool, &token).await {
        Ok(preview) => preview.map(|(preview, _)| preview),
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
    redirect_to(state, cookie, "/")
}

fn redirect_to(state: &AppState, cookie: HeaderValue, path: &str) -> Response {
    let home = format!(
        "{}{}",
        state
            .organization_config
            .snapshot()
            .site_base_url()
            .trim_end_matches('/'),
        local_path(path).unwrap_or("/")
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
        Ok(Some((_, path))) => {
            let secure = match state.organization_config.snapshot().secure_cookies() {
                true => "; Secure",
                false => "",
            };
            match HeaderValue::from_str(&format!(
                "{PREVIEW_COOKIE}={rest}; Path=/; HttpOnly; SameSite=Lax; Max-Age={}{secure}",
                PREVIEW_MINUTES * 60
            )) {
                Ok(cookie) => redirect_to(state, cookie, path.as_deref().unwrap_or("/")),
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

pub(crate) enum BannerSubject<'a> {
    Theme(&'a str),
    Draft,
}

pub(crate) fn with_banner(html: String, subject: BannerSubject<'_>, locale: &str) -> String {
    let portuguese = locale.starts_with("pt");
    let leave = match portuguese {
        true => "Sair",
        false => "Leave",
    };
    let draft = matches!(subject, BannerSubject::Draft);
    let label = match (subject, portuguese) {
        (BannerSubject::Theme(name), true) => {
            format!(
                "Pré-visualização do tema <strong>{}</strong>",
                escape_html(name)
            )
        }
        (BannerSubject::Theme(name), false) => {
            format!(
                "Previewing the theme <strong>{}</strong>",
                escape_html(name)
            )
        }
        (BannerSubject::Draft, true) => "Pré-visualização do rascunho".to_owned(),
        (BannerSubject::Draft, false) => "Previewing the draft".to_owned(),
    };
    let script = match draft {
        true => format!(
            "<script type=\"module\" src=\"/_aphel/site{}\"></script>",
            crate::dynamic::PREVIEW_SCRIPT_PATH
        ),
        false => String::new(),
    };
    let banner = format!(
        "{script}<div role=\"status\" data-aphel-preview-banner style=\"position:fixed;left:50%;bottom:16px;transform:translateX(-50%);z-index:2147483647;background:#111827;color:#fff;padding:10px 18px;border-radius:999px;font:14px/1.4 system-ui,sans-serif;box-shadow:0 4px 16px rgba(0,0,0,.25)\">{label} · <a href=\"/_aphel/site/{PREVIEW_SEGMENT}/{END_SEGMENT}\" style=\"color:#fff;text-decoration:underline\">{leave}</a></div>"
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
