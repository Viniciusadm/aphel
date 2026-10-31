use axum::Json;
use axum::extract::{DefaultBodyLimit, Path, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use chrono::SecondsFormat;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use utoipa_axum::router::{OpenApiRouter, UtoipaMethodRouterExt};
use utoipa_axum::routes;

use crate::css;
use crate::domain::{FONT_NOT_FOUND, SOURCE_UPLOAD};
use crate::package::{ThemePackage, TokenSets};
use crate::service::{self, PreviewReport, ThemeList, ThemeSummary};
use crate::store::ThemeStore;
use crate::validate::{Issue, MAX_PACKAGE_BYTES};
use aphel_kernel::Ctx;
use aphel_kernel::api::dto::common::{Data, ErrorEnvelope};
use aphel_kernel::api::error::ApiError;
use aphel_kernel::api::extractors::{AuthContext, ClientInfo, JsonBody};
use aphel_kernel::api::state::AppState;
use aphel_kernel::errors::AppError;

pub(crate) const TAG: &str = "themes";
const BODY_LIMIT: usize = MAX_PACKAGE_BYTES * 2 + 512 * 1024;
const FONT_EXTENSION: &str = ".woff2";

pub(crate) fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list_themes))
        .routes(routes!(install_theme).layer(DefaultBodyLimit::max(BODY_LIMIT)))
        .routes(routes!(validate_theme).layer(DefaultBodyLimit::max(BODY_LIMIT)))
        .routes(routes!(activate_theme))
        .routes(routes!(save_theme).layer(DefaultBodyLimit::max(BODY_LIMIT)))
        .routes(routes!(delete_theme))
        .routes(routes!(export_theme))
        .routes(routes!(active_stylesheet))
        .routes(routes!(theme_font))
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct ThemeResponse {
    key: String,
    name: String,
    version: String,
    author: String,
    description: Option<String>,
    license: String,
    min_aphel_version: Option<String>,
    source: String,
    active: bool,
    font_count: usize,
    tokens: TokenSets,
    checksum: String,
    created_at: String,
    updated_at: String,
}

impl From<ThemeSummary> for ThemeResponse {
    fn from(summary: ThemeSummary) -> Self {
        let record = summary.record;
        Self {
            key: record.key,
            name: record.name,
            version: record.version,
            author: record.author,
            description: record.description,
            license: record.license,
            min_aphel_version: record.min_aphel_version,
            source: record.source,
            active: summary.active,
            font_count: summary.font_count,
            tokens: record.tokens,
            checksum: record.checksum,
            created_at: record
                .created_at
                .to_rfc3339_opts(SecondsFormat::Millis, true),
            updated_at: record
                .updated_at
                .to_rfc3339_opts(SecondsFormat::Millis, true),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct ThemeListResponse {
    active_key: Option<String>,
    themes: Vec<ThemeResponse>,
}

impl From<ThemeList> for ThemeListResponse {
    fn from(list: ThemeList) -> Self {
        Self {
            active_key: list.active_key,
            themes: list.themes.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct IssueResponse {
    path: String,
    message: String,
}

impl From<Issue> for IssueResponse {
    fn from(issue: Issue) -> Self {
        Self {
            path: issue.path,
            message: issue.message,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct PreviewFontResponse {
    family: String,
    weight: u16,
    style: String,
    bytes: usize,
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct PreviewResponse {
    issues: Vec<IssueResponse>,
    warnings: Vec<IssueResponse>,
    token_count: usize,
    fonts: Vec<PreviewFontResponse>,
    installed_version: Option<String>,
}

impl From<PreviewReport> for PreviewResponse {
    fn from(report: PreviewReport) -> Self {
        Self {
            issues: report.issues.into_iter().map(Into::into).collect(),
            warnings: report.warnings.into_iter().map(Into::into).collect(),
            token_count: report.token_count,
            fonts: report
                .fonts
                .into_iter()
                .map(|font| PreviewFontResponse {
                    family: font.family,
                    weight: font.weight,
                    style: font.style,
                    bytes: font.bytes,
                })
                .collect(),
            installed_version: report.installed_version,
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct ActivationRequest {
    key: Option<String>,
}

#[utoipa::path(
    get,
    path = "/themes",
    tag = TAG,
    operation_id = "themes_list",
    security(("session_cookie" = [])),
    responses(
        (status = 200, description = "Installed themes and the active one. The default look is always available and is selected with no active theme", body = Data<ThemeListResponse>),
        (status = 401, description = "Missing or invalid credentials", body = ErrorEnvelope),
        (status = 403, description = "Missing `themes.manage` (`permission_denied`)", body = ErrorEnvelope),
    )
)]
async fn list_themes(
    State(state): State<AppState>,
    tenant: AuthContext,
) -> Result<Json<Data<ThemeListResponse>>, ApiError> {
    let list = service::list(&Ctx::from_state(&state, &tenant.scope).await?).await?;
    Ok(Json(Data::new(list.into())))
}

#[utoipa::path(
    post,
    path = "/themes",
    tag = TAG,
    operation_id = "themes_install",
    security(("session_cookie" = [])),
    request_body = ThemePackage,
    responses(
        (status = 201, description = "Theme installed, or replaced when the id is already installed. An active theme stays active", body = Data<ThemeResponse>),
        (status = 401, description = "Missing or invalid credentials", body = ErrorEnvelope),
        (status = 403, description = "Missing `themes.manage` (`permission_denied`)", body = ErrorEnvelope),
        (status = 409, description = "Limit of 20 themes reached (`theme_limit_reached`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid package (`theme_invalid`, with `details.fields`)", body = ErrorEnvelope),
    )
)]
async fn install_theme(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    JsonBody(package): JsonBody<ThemePackage>,
) -> Result<(StatusCode, Json<Data<ThemeResponse>>), ApiError> {
    let summary = service::install(
        &Ctx::from_state(&state, &tenant.scope).await?,
        package,
        SOURCE_UPLOAD,
        client.0.ip.clone(),
    )
    .await?;
    Ok((StatusCode::CREATED, Json(Data::new(summary.into()))))
}

#[utoipa::path(
    post,
    path = "/themes/validate",
    tag = TAG,
    operation_id = "themes_validate",
    security(("session_cookie" = [])),
    request_body = ThemePackage,
    responses(
        (status = 200, description = "Validation report without installing: blocking `issues`, `warnings` such as ignored tokens, the fonts and the installed version of the same id", body = Data<PreviewResponse>),
        (status = 401, description = "Missing or invalid credentials", body = ErrorEnvelope),
        (status = 403, description = "Missing `themes.manage` (`permission_denied`)", body = ErrorEnvelope),
    )
)]
async fn validate_theme(
    State(state): State<AppState>,
    tenant: AuthContext,
    JsonBody(package): JsonBody<ThemePackage>,
) -> Result<Json<Data<PreviewResponse>>, ApiError> {
    let report = service::preview(&Ctx::from_state(&state, &tenant.scope).await?, package).await?;
    Ok(Json(Data::new(report.into())))
}

#[utoipa::path(
    post,
    path = "/themes/activation",
    tag = TAG,
    operation_id = "themes_activate",
    security(("session_cookie" = [])),
    request_body = ActivationRequest,
    responses(
        (status = 200, description = "Theme activated for the whole installation, public pages included. A null `key` goes back to the default look", body = Data<ThemeListResponse>),
        (status = 401, description = "Missing or invalid credentials", body = ErrorEnvelope),
        (status = 403, description = "Missing `themes.manage` (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Theme not installed (`theme_not_found`)", body = ErrorEnvelope),
    )
)]
async fn activate_theme(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    JsonBody(request): JsonBody<ActivationRequest>,
) -> Result<Json<Data<ThemeListResponse>>, ApiError> {
    let list = service::activate(
        &Ctx::from_state(&state, &tenant.scope).await?,
        request.key,
        client.0.ip.clone(),
    )
    .await?;
    Ok(Json(Data::new(list.into())))
}

#[utoipa::path(
    put,
    path = "/themes/{key}",
    tag = TAG,
    operation_id = "themes_save",
    security(("session_cookie" = [])),
    params(("key" = String, Path, description = "Theme id, equal to `id` in the body")),
    request_body = ThemePackage,
    responses(
        (status = 200, description = "Theme created or updated from the editor", body = Data<ThemeResponse>),
        (status = 401, description = "Missing or invalid credentials", body = ErrorEnvelope),
        (status = 403, description = "Missing `themes.manage` (`permission_denied`)", body = ErrorEnvelope),
        (status = 409, description = "Limit of 20 themes reached (`theme_limit_reached`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid package (`theme_invalid`) or an id different from the path (`theme_key_mismatch`)", body = ErrorEnvelope),
    )
)]
async fn save_theme(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(key): Path<String>,
    JsonBody(package): JsonBody<ThemePackage>,
) -> Result<Json<Data<ThemeResponse>>, ApiError> {
    let summary = service::save_from_editor(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &key,
        package,
        client.0.ip.clone(),
    )
    .await?;
    Ok(Json(Data::new(summary.into())))
}

#[utoipa::path(
    delete,
    path = "/themes/{key}",
    tag = TAG,
    operation_id = "themes_delete",
    security(("session_cookie" = [])),
    params(("key" = String, Path, description = "Theme id")),
    responses(
        (status = 204, description = "Theme removed with its fonts"),
        (status = 401, description = "Missing or invalid credentials", body = ErrorEnvelope),
        (status = 403, description = "Missing `themes.manage` (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Theme not installed (`theme_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "The theme is active (`theme_active`)", body = ErrorEnvelope),
    )
)]
async fn delete_theme(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(key): Path<String>,
) -> Result<StatusCode, ApiError> {
    service::remove(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &key,
        client.0.ip.clone(),
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    get,
    path = "/themes/{key}/package",
    tag = TAG,
    operation_id = "themes_export",
    security(("session_cookie" = [])),
    params(("key" = String, Path, description = "Theme id")),
    responses(
        (status = 200, description = "The theme as an `aphel-theme` package with its fonts, ready to be installed elsewhere", body = ThemePackage),
        (status = 401, description = "Missing or invalid credentials", body = ErrorEnvelope),
        (status = 403, description = "Missing `themes.manage` (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Theme not installed (`theme_not_found`)", body = ErrorEnvelope),
    )
)]
async fn export_theme(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(key): Path<String>,
) -> Result<Json<ThemePackage>, ApiError> {
    let package = service::export(&Ctx::from_state(&state, &tenant.scope).await?, &key).await?;
    Ok(Json(package))
}

#[utoipa::path(
    get,
    path = "/public/themes/active.css",
    tag = TAG,
    operation_id = "themes_active_stylesheet",
    description = "Stylesheet of the active theme, linked by the web app before it renders. Public: it only holds design tokens. Empty when no theme is active. Uses an ETag that changes whenever the active theme does",
    responses(
        (status = 200, description = "CSS with the theme tokens and its font faces", content_type = "text/css", body = String),
        (status = 304, description = "The active theme did not change"),
    )
)]
async fn active_stylesheet(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let store = ThemeStore::new(state.db()?);
    let theme_state = store.state().await?;
    let etag = format!("\"theme-{}\"", theme_state.version);
    let unchanged = headers
        .get(header::IF_NONE_MATCH)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value == etag);
    let mut response = if unchanged {
        StatusCode::NOT_MODIFIED.into_response()
    } else {
        let body = match &theme_state.active_key {
            Some(key) => match store.find(key).await? {
                Some(record) => css::build(&record.tokens, &store.fonts(key).await?),
                None => String::new(),
            },
            None => String::new(),
        };
        body.into_response()
    };
    let response_headers = response.headers_mut();
    response_headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/css; charset=utf-8"),
    );
    response_headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-cache"));
    if let Ok(value) = HeaderValue::from_str(&etag) {
        response_headers.insert(header::ETAG, value);
    }
    Ok(response)
}

#[utoipa::path(
    get,
    path = "/public/themes/fonts/{file}",
    tag = TAG,
    operation_id = "themes_font",
    description = "A font file of an installed theme, addressed by the SHA-256 of its content (`<sha256>.woff2`), so it can be cached forever",
    params(("file" = String, Path, description = "`<sha256>.woff2`")),
    responses(
        (status = 200, description = "The woff2 file", content_type = "font/woff2", body = Vec<u8>),
        (status = 404, description = "Unknown font (`theme_font_not_found`)", body = ErrorEnvelope),
    )
)]
async fn theme_font(
    State(state): State<AppState>,
    Path(file): Path<String>,
) -> Result<Response, ApiError> {
    let not_found = || AppError::not_found_code(FONT_NOT_FOUND, "the font does not exist");
    let hash = file.strip_suffix(FONT_EXTENSION).ok_or_else(not_found)?;
    if hash.len() != 64 || !hash.chars().all(|character| character.is_ascii_hexdigit()) {
        return Err(not_found().into());
    }
    let bytes = ThemeStore::new(state.db()?)
        .font_by_hash(&hash.to_ascii_lowercase())
        .await?
        .ok_or_else(not_found)?;
    Ok((
        [
            (header::CONTENT_TYPE, "font/woff2"),
            (header::CACHE_CONTROL, "public, max-age=31536000, immutable"),
        ],
        bytes,
    )
        .into_response())
}
