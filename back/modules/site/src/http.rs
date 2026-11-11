use aphel_kernel::Ctx;
use aphel_kernel::errors::AppError;
use aphel_kernel::records::{RecordMutation, publish_mutation};
use aphel_sdk::platform::api::dto::common::{Data, ErrorEnvelope};
use aphel_sdk::platform::api::error::ApiError;
use aphel_sdk::platform::api::extractors::{AuthContext, ClientInfo};
use aphel_sdk::platform::api::state::AppState;
use aphel_sdk::platform::application::access::Authorizer;
use aphel_sdk::platform::application::shared::{FieldChange, RecordRef};
use aphel_sdk::platform::domain::access::Permission;
use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use chrono::{DateTime, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::Row;
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::cache::{self, ALL_TAG};
use crate::redirects;
use crate::runtime;
use crate::settings::site_settings;

pub(crate) const TAG: &str = "site";
const REDIRECT_DELETED: &str = "site.redirect_deleted";
const CACHE_PURGED: &str = "site.cache_purged";
const PATH_REDIRECT_CREATED: &str = "site.path_redirect_created";
const PATH_REDIRECT_DELETED: &str = "site.path_redirect_deleted";
const MAX_REDIRECTS: i64 = 500;

pub(crate) fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .merge(crate::appearance_http::router())
        .routes(routes!(overview))
        .routes(routes!(list_redirects))
        .routes(routes!(delete_redirect))
        .routes(routes!(list_path_redirects, create_path_redirect))
        .routes(routes!(delete_path_redirect))
        .routes(routes!(start_preview))
        .routes(routes!(purge_cache))
        .routes(routes!(list_embeds))
        .routes(routes!(list_collections))
        .routes(routes!(list_languages))
        .routes(routes!(render_preview))
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct SiteThemeResponse {
    #[schema(example = "clean_theme")]
    pub key: String,
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct SiteMenuItemResponse {
    #[schema(example = "blog.index")]
    pub id: String,
    pub label: String,
    #[schema(example = "/blog")]
    pub path: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct SiteOverviewResponse {
    #[schema(example = "https://www.example.com")]
    pub base_url: String,
    pub themes: Vec<SiteThemeResponse>,
    pub active_theme: Option<String>,
    pub menu_items: Vec<SiteMenuItemResponse>,
    pub pages: usize,
    pub customers_required: bool,
    #[schema(example = "pt-BR")]
    pub default_language: String,
    pub available_languages: Vec<String>,
    pub people_available: bool,
    pub home_candidates: Vec<SiteHomeCandidateResponse>,
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct SiteHomeCandidateResponse {
    #[schema(example = "pages")]
    pub owner: String,
    pub name: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct SiteRedirectResponse {
    pub id: u64,
    #[schema(example = "blog.post")]
    pub record_type: String,
    #[schema(example = "slug")]
    pub field: String,
    pub old_value: String,
    pub new_value: String,
    #[schema(example = "/blog/old-title")]
    pub old_path: Option<String>,
    #[schema(example = "/blog/new-title")]
    pub new_path: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct SitePathRedirectResponse {
    pub id: u64,
    #[schema(example = "/old-page")]
    pub source_path: String,
    #[schema(example = "/blog")]
    pub target: String,
    pub permanent: bool,
    pub created_at: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub(crate) struct CreateSitePathRedirect {
    #[schema(example = "/old-page")]
    pub source_path: String,
    #[schema(example = "/blog")]
    pub target: String,
    #[serde(default = "permanent_by_default")]
    pub permanent: bool,
}

#[derive(Debug, Deserialize, ToSchema)]
pub(crate) struct StartSitePreview {
    #[schema(example = "clean_theme")]
    pub theme: Option<String>,
    pub record: Option<StartSitePreviewRecord>,
    #[schema(example = "/about-us")]
    pub path: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub(crate) struct StartSitePreviewRecord {
    #[schema(example = "pages.page")]
    pub record_type: String,
    #[schema(example = "01932c3e-8f3a-7c4e-9a51-1f0e5b2d4c6a")]
    pub record_id: String,
    #[schema(example = json!({ "blocks": "draft_blocks" }))]
    #[serde(default)]
    pub fields: std::collections::BTreeMap<String, String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub(crate) struct RenderSitePreview {
    pub record: RenderSitePreviewRecord,
    #[schema(example = "/about-us")]
    pub path: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub(crate) struct RenderSitePreviewRecord {
    #[schema(example = "pages.page")]
    pub record_type: String,
    #[schema(example = "01932c3e-8f3a-7c4e-9a51-1f0e5b2d4c6a")]
    pub record_id: String,
    #[schema(example = json!({ "title": "About us", "blocks": "[]" }))]
    #[serde(default)]
    pub values: std::collections::BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct SitePreviewRenderResponse {
    pub html: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct SitePreviewResponse {
    #[schema(example = "https://www.example.com/_aphel/site/preview/3f2a...")]
    pub url: String,
    #[schema(example = "https://www.example.com/about-us")]
    pub page_url: String,
    pub expires_in_minutes: i64,
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct SiteEmbedOptionResponse {
    #[schema(example = "01932c3e-8f3a-7c4e-9a51-1f0e5b2d4c6a")]
    pub key: String,
    pub label: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct SiteEmbedSourceResponse {
    #[schema(example = "form")]
    pub kind: String,
    pub options: Vec<SiteEmbedOptionResponse>,
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct SiteLanguageResponse {
    #[schema(example = "en")]
    pub code: String,
    #[schema(example = "/en")]
    pub prefix: String,
    #[schema(example = "en")]
    pub field_suffix: String,
    pub default: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct SiteCollectionResponse {
    #[schema(example = "blog")]
    pub owner: String,
    #[schema(example = "post")]
    pub entity: String,
    #[schema(example = "blog.post")]
    pub record_type: String,
    pub label: String,
    pub fields: Vec<String>,
}

fn permanent_by_default() -> bool {
    true
}

fn require_manage(
    scope: &aphel_sdk::platform::domain::access::AccessScope,
) -> Result<(), AppError> {
    scope.require(&Permission::SETTINGS_MANAGE).map(|_| ())
}

#[utoipa::path(
    get,
    path = "/site",
    tag = TAG,
    operation_id = "site_overview",
    description = "What the public site offers right now: the themes of installed plugins, the menu items they contribute (in declaration order, with one item per published record for items that list records; the order and visibility chosen by the organization live in the `site` organization setting) and the absolute site address, and the plugins that offer a home page (`home_candidates`; the organization picks one in the `home` key of the `site` setting, otherwise the first one answers). Requires `settings.manage`.",
    security(("session_cookie" = [])),
    responses(
        (status = 200, description = "Site overview", body = Data<SiteOverviewResponse>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`)", body = ErrorEnvelope),
    )
)]
async fn overview(
    State(state): State<AppState>,
    tenant: AuthContext,
) -> Result<Json<Data<SiteOverviewResponse>>, ApiError> {
    require_manage(&tenant.scope)?;
    let config = state.organization_config.snapshot();
    let runtime = runtime::active(&state).await?;
    let default_language = {
        use aphel_sdk::platform::application::organization_profile::OrganizationRepository;
        aphel_sdk::platform::infrastructure::repositories::mysql_organization_profile::MySqlOrganizationRepository::new(state.db()?)
            .find()
            .await?
            .map(|organization| organization.locale)
            .unwrap_or_default()
    };
    let (record_lines, _) = crate::handler::record_menu(&state, &runtime, None).await?;
    Ok(Json(Data::new(SiteOverviewResponse {
        base_url: config.site_base_url(),
        themes: runtime
            .themes
            .iter()
            .map(|theme| SiteThemeResponse {
                key: theme.key.clone(),
                name: theme.name.clone(),
                description: theme.description.clone(),
            })
            .collect(),
        active_theme: runtime.theme.clone(),
        menu_items: runtime
            .menu
            .iter()
            .filter(|item| item.item.records.is_none())
            .map(|item| SiteMenuItemResponse {
                id: item.id.clone(),
                label: item.item.label.clone(),
                path: item.path.clone(),
            })
            .chain(record_lines.into_iter().map(|line| SiteMenuItemResponse {
                id: line.id,
                label: line.label,
                path: line.path,
            }))
            .collect(),
        pages: runtime.routes.len(),
        customers_required: runtime.uses_customers,
        default_language: default_language.clone(),
        available_languages: crate::settings::SITE_LANGUAGES
            .iter()
            .filter(|language| !language.eq_ignore_ascii_case(&default_language))
            .map(|language| (*language).to_owned())
            .collect(),
        people_available: state.app.is_enabled(aphel_sdk::people::KEY),
        home_candidates: runtime
            .home_candidates()
            .into_iter()
            .map(|(owner, name)| SiteHomeCandidateResponse { owner, name })
            .collect(),
    })))
}

#[utoipa::path(
    get,
    path = "/site/redirects",
    tag = TAG,
    operation_id = "site_redirects_list",
    description = "Addresses that moved because the field of a record page changed (for example a new slug), newest first, up to 500. Requires `settings.manage`.",
    security(("session_cookie" = [])),
    responses(
        (status = 200, description = "Redirects", body = Data<Vec<SiteRedirectResponse>>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`)", body = ErrorEnvelope),
    )
)]
async fn list_redirects(
    State(state): State<AppState>,
    tenant: AuthContext,
) -> Result<Json<Data<Vec<SiteRedirectResponse>>>, ApiError> {
    require_manage(&tenant.scope)?;
    let runtime = runtime::active(&state).await?;
    let path_of = |record_type: &str, field: &str, value: &str| {
        runtime
            .routes
            .iter()
            .find_map(|route| match &route.page.source {
                aphel_contract_site::PageSource::Record {
                    entity,
                    match_field,
                } if entity == record_type && match_field == field => route.pattern.build(
                    &std::collections::BTreeMap::from([(field.to_owned(), value.to_owned())]),
                ),
                _ => None,
            })
    };
    let rows = sqlx::query(
        "SELECT id, record_type, field, old_value, new_value, created_at FROM site_redirects \
         ORDER BY created_at DESC, id DESC LIMIT ?",
    )
    .bind(MAX_REDIRECTS)
    .fetch_all(&state.db()?)
    .await
    .map_err(AppError::from)?;
    let mut redirects = Vec::with_capacity(rows.len());
    for row in rows {
        let created_at: DateTime<Utc> = row.try_get("created_at").map_err(AppError::from)?;
        let record_type: String = row.try_get("record_type").map_err(AppError::from)?;
        let field: String = row.try_get("field").map_err(AppError::from)?;
        let old_value: String = row.try_get("old_value").map_err(AppError::from)?;
        let new_value: String = row.try_get("new_value").map_err(AppError::from)?;
        redirects.push(SiteRedirectResponse {
            id: row.try_get("id").map_err(AppError::from)?,
            old_path: path_of(&record_type, &field, &old_value),
            new_path: path_of(&record_type, &field, &new_value),
            record_type,
            field,
            old_value,
            new_value,
            created_at: created_at.to_rfc3339_opts(SecondsFormat::Secs, true),
        });
    }
    Ok(Json(Data::new(redirects)))
}

#[utoipa::path(
    delete,
    path = "/site/redirects/{redirect_id}",
    tag = TAG,
    operation_id = "site_redirects_delete",
    description = "Stops redirecting an old address; it answers 404 again unless a record takes it. Publishes `site.redirect_deleted` and writes the audit entry. Requires `settings.manage`.",
    security(("session_cookie" = [])),
    params(("redirect_id" = u64, Path)),
    responses(
        (status = 204, description = "Redirect removed"),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Unknown redirect (`site_redirect_not_found`)", body = ErrorEnvelope),
    )
)]
async fn delete_redirect(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(redirect_id): Path<u64>,
) -> Result<StatusCode, ApiError> {
    require_manage(&tenant.scope)?;
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let mut tx = ctx.begin().await?;
    let row = sqlx::query(
        "SELECT record_type, field, old_value, new_value FROM site_redirects WHERE id = ? FOR UPDATE",
    )
    .bind(redirect_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(AppError::from)?
    .ok_or_else(|| AppError::not_found_code("site_redirect_not_found", "the redirect was not found"))?;
    let record_type: String = row.try_get("record_type").map_err(AppError::from)?;
    let field: String = row.try_get("field").map_err(AppError::from)?;
    let old_value: String = row.try_get("old_value").map_err(AppError::from)?;
    let new_value: String = row.try_get("new_value").map_err(AppError::from)?;
    sqlx::query("DELETE FROM site_redirects WHERE id = ?")
        .bind(redirect_id)
        .execute(&mut *tx)
        .await
        .map_err(AppError::from)?;
    publish_mutation(
        ctx.events(),
        ctx.audit(),
        &mut tx,
        ctx.scope,
        ctx.now(),
        RecordMutation {
            record: RecordRef::organization(),
            event_type: REDIRECT_DELETED,
            payload: json!({
                "record_type": record_type,
                "field": field,
                "old_value": old_value,
                "new_value": new_value,
            }),
            changes: vec![FieldChange::new(
                "site.redirect",
                json!(format!("{old_value} -> {new_value}")),
                serde_json::Value::Null,
            )],
            ip: client.0.ip,
        },
    )
    .await?;
    tx.commit().await.map_err(AppError::from)?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    post,
    path = "/site/cache/purge",
    tag = TAG,
    operation_id = "site_cache_purge",
    description = "Renders every site page again on the next visit, in every server process within a few seconds. Publishes `site.cache_purged` and writes the audit entry. Requires `settings.manage`.",
    security(("session_cookie" = [])),
    responses(
        (status = 204, description = "Cache cleared"),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`)", body = ErrorEnvelope),
    )
)]
async fn purge_cache(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
) -> Result<StatusCode, ApiError> {
    require_manage(&tenant.scope)?;
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let mut tx = ctx.begin().await?;
    sqlx::query("INSERT INTO site_cache_invalidations (tag) VALUES (?)")
        .bind(ALL_TAG)
        .execute(&mut *tx)
        .await
        .map_err(AppError::from)?;
    publish_mutation(
        ctx.events(),
        ctx.audit(),
        &mut tx,
        ctx.scope,
        ctx.now(),
        RecordMutation {
            record: RecordRef::organization(),
            event_type: CACHE_PURGED,
            payload: json!({}),
            changes: Vec::new(),
            ip: client.0.ip,
        },
    )
    .await?;
    tx.commit().await.map_err(AppError::from)?;
    cache::forget_all();
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    get,
    path = "/site/path-redirects",
    tag = TAG,
    operation_id = "site_path_redirects_list",
    description = "Redirects added by hand: an old address of the site (relative to the site address) that sends visitors to a site path or to another website, newest first, up to 500. They apply only when no page answers the old address. Requires `settings.manage`.",
    security(("session_cookie" = [])),
    responses(
        (status = 200, description = "Redirects", body = Data<Vec<SitePathRedirectResponse>>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`)", body = ErrorEnvelope),
    )
)]
async fn list_path_redirects(
    State(state): State<AppState>,
    tenant: AuthContext,
) -> Result<Json<Data<Vec<SitePathRedirectResponse>>>, ApiError> {
    require_manage(&tenant.scope)?;
    let rows: Vec<(u64, String, String, bool, DateTime<Utc>)> = sqlx::query_as(
        "SELECT id, source_path, target, permanent, created_at FROM site_path_redirects \
         ORDER BY created_at DESC, id DESC LIMIT ?",
    )
    .bind(MAX_REDIRECTS)
    .fetch_all(&state.db()?)
    .await
    .map_err(AppError::from)?;
    Ok(Json(Data::new(
        rows.into_iter()
            .map(
                |(id, source_path, target, permanent, created_at)| SitePathRedirectResponse {
                    id,
                    source_path,
                    target,
                    permanent,
                    created_at: created_at.to_rfc3339_opts(SecondsFormat::Secs, true),
                },
            )
            .collect(),
    )))
}

#[utoipa::path(
    post,
    path = "/site/path-redirects",
    tag = TAG,
    operation_id = "site_path_redirects_create",
    description = "Sends visitors of an old address to a site path (`/blog`) or to another website (`https://...`). The old address is a plain path relative to the site address, without query or fragment; a trailing slash is ignored. Answers 301 when `permanent` (the default) and 302 otherwise. Publishes `site.path_redirect_created` and writes the audit entry. Requires `settings.manage`.",
    security(("session_cookie" = [])),
    request_body = CreateSitePathRedirect,
    responses(
        (status = 201, description = "Redirect created", body = Data<SitePathRedirectResponse>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`)", body = ErrorEnvelope),
        (status = 409, description = "The old address already redirects (`site_path_redirect_exists`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid address or target (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn create_path_redirect(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Json(input): Json<CreateSitePathRedirect>,
) -> Result<(StatusCode, Json<Data<SitePathRedirectResponse>>), ApiError> {
    require_manage(&tenant.scope)?;
    let source_path = redirects::normalize_source(&input.source_path)?;
    let target = redirects::normalize_target(&input.target, &source_path)?;
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let mut tx = ctx.begin().await?;
    let exists: Option<u64> =
        sqlx::query_scalar("SELECT id FROM site_path_redirects WHERE source_path = ? FOR UPDATE")
            .bind(&source_path)
            .fetch_optional(&mut *tx)
            .await
            .map_err(AppError::from)?;
    if exists.is_some() {
        return Err(AppError::conflict_code(
            "site_path_redirect_exists",
            "the old address already redirects",
        )
        .into());
    }
    let id = sqlx::query(
        "INSERT INTO site_path_redirects (source_path, target, permanent) VALUES (?, ?, ?)",
    )
    .bind(&source_path)
    .bind(&target)
    .bind(input.permanent)
    .execute(&mut *tx)
    .await
    .map_err(AppError::from)?
    .last_insert_id();
    publish_mutation(
        ctx.events(),
        ctx.audit(),
        &mut tx,
        ctx.scope,
        ctx.now(),
        RecordMutation {
            record: RecordRef::organization(),
            event_type: PATH_REDIRECT_CREATED,
            payload: json!({
                "source_path": source_path,
                "target": target,
                "permanent": input.permanent,
            }),
            changes: vec![FieldChange::new(
                "site.path_redirect",
                serde_json::Value::Null,
                json!(format!("{source_path} -> {target}")),
            )],
            ip: client.0.ip,
        },
    )
    .await?;
    tx.commit().await.map_err(AppError::from)?;
    Ok((
        StatusCode::CREATED,
        Json(Data::new(SitePathRedirectResponse {
            id,
            source_path,
            target,
            permanent: input.permanent,
            created_at: ctx.now().to_rfc3339_opts(SecondsFormat::Secs, true),
        })),
    ))
}

#[utoipa::path(
    delete,
    path = "/site/path-redirects/{redirect_id}",
    tag = TAG,
    operation_id = "site_path_redirects_delete",
    description = "Removes a redirect added by hand; the old address answers 404 again unless a page takes it. Publishes `site.path_redirect_deleted` and writes the audit entry. Requires `settings.manage`.",
    security(("session_cookie" = [])),
    params(("redirect_id" = u64, Path)),
    responses(
        (status = 204, description = "Redirect removed"),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Unknown redirect (`site_path_redirect_not_found`)", body = ErrorEnvelope),
    )
)]
async fn delete_path_redirect(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(redirect_id): Path<u64>,
) -> Result<StatusCode, ApiError> {
    require_manage(&tenant.scope)?;
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let mut tx = ctx.begin().await?;
    let (source_path, target): (String, String) = sqlx::query_as(
        "SELECT source_path, target FROM site_path_redirects WHERE id = ? FOR UPDATE",
    )
    .bind(redirect_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(AppError::from)?
    .ok_or_else(|| {
        AppError::not_found_code("site_path_redirect_not_found", "the redirect was not found")
    })?;
    sqlx::query("DELETE FROM site_path_redirects WHERE id = ?")
        .bind(redirect_id)
        .execute(&mut *tx)
        .await
        .map_err(AppError::from)?;
    publish_mutation(
        ctx.events(),
        ctx.audit(),
        &mut tx,
        ctx.scope,
        ctx.now(),
        RecordMutation {
            record: RecordRef::organization(),
            event_type: PATH_REDIRECT_DELETED,
            payload: json!({ "source_path": source_path, "target": target }),
            changes: vec![FieldChange::new(
                "site.path_redirect",
                json!(format!("{source_path} -> {target}")),
                serde_json::Value::Null,
            )],
            ip: client.0.ip,
        },
    )
    .await?;
    tx.commit().await.map_err(AppError::from)?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    post,
    path = "/site/preview",
    tag = TAG,
    operation_id = "site_preview_start",
    description = "Creates a link that, in the browser that opens it and for 30 minutes, shows the site without changing it for visitors: with another theme (`theme`, null for the built-in one; requires `settings.manage`) or with one record as it would look before publication (`record`: its pages show it even when it fails the public conditions, with each public field named in `fields` replaced by the value of another field of the record, such as `{\"blocks\": \"draft_blocks\"}`, and its images are served; requires permission to update that record or `settings.manage`). `path` is the site path the link opens (default `/`); `page_url` is its public address. Preview pages are not cached and carry a banner with a link that ends the preview.",
    security(("session_cookie" = [])),
    request_body = StartSitePreview,
    responses(
        (status = 201, description = "Preview link", body = Data<SitePreviewResponse>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`)", body = ErrorEnvelope),
        (status = 422, description = "Unknown theme (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn start_preview(
    State(state): State<AppState>,
    tenant: AuthContext,
    Json(input): Json<StartSitePreview>,
) -> Result<(StatusCode, Json<Data<SitePreviewResponse>>), ApiError> {
    let config = state.organization_config.snapshot();
    let runtime = runtime::active(&state).await?;
    let path = match input
        .path
        .as_deref()
        .map(str::trim)
        .filter(|path| !path.is_empty())
    {
        Some(path) => Some(crate::preview::local_path(path).ok_or_else(|| {
            AppError::Validation("path must be a site path such as /about-us".to_owned())
        })?),
        None => None,
    };
    let record = match &input.record {
        Some(record) => Some(preview_record(&state, &tenant, &runtime, record).await?),
        None => {
            require_manage(&tenant.scope)?;
            None
        }
    };
    let theme = match &record {
        Some(_) => None,
        None => input
            .theme
            .as_deref()
            .map(str::trim)
            .filter(|theme| !theme.is_empty()),
    };
    if let Some(theme) = theme
        && !runtime
            .themes
            .iter()
            .any(|candidate| candidate.key == theme)
    {
        return Err(AppError::Validation(format!("the theme {theme} is not installed")).into());
    }
    let token = crate::preview::start(&state.db()?, theme, record.as_ref(), path).await?;
    Ok((
        StatusCode::CREATED,
        Json(Data::new(SitePreviewResponse {
            url: crate::preview::link(&config.site_base_url(), &token),
            page_url: format!(
                "{}{}",
                config.site_base_url().trim_end_matches('/'),
                path.unwrap_or("/")
            ),
            expires_in_minutes: crate::preview::PREVIEW_MINUTES,
        })),
    ))
}

const MAX_PREVIEW_FIELDS: usize = 10;
const MAX_PREVIEW_VALUE_BYTES: usize = 64 * 1024;
const PREVIEW_RENDERS_PER_USER: u32 = 300;
const PREVIEW_RATE_LIMITED: &str = "too many preview renders; wait a moment";

static PREVIEW_RENDER_LIMIT: std::sync::LazyLock<
    aphel_sdk::platform::api::rate_limit::SharedRateLimiter,
> = std::sync::LazyLock::new(|| {
    aphel_sdk::platform::api::rate_limit::SharedRateLimiter::new(
        "site.preview_render",
        PREVIEW_RENDERS_PER_USER,
        std::time::Duration::from_secs(60),
    )
});

fn preview_value_target(
    exposure: &aphel_contract_site::SiteExposure,
    languages: &[String],
    target: &str,
) -> bool {
    let plain =
        exposure.exposes(target) && !aphel_contract_site::ALWAYS_PUBLIC_FIELDS.contains(&target);
    let translated = exposure.translated.iter().any(|field| {
        languages.iter().any(|language| {
            target == format!("{field}_{}", crate::settings::field_suffix(language))
        })
    });
    plain || translated
}

#[utoipa::path(
    post,
    path = "/site/preview/render",
    tag = TAG,
    operation_id = "site_preview_render",
    description = "Renders, without saving anything, the site page at `path` (relative to the site address, with the language prefix when needed) as it would look with the record's public fields replaced by `values`. Keys are public fields of the record's entity, or `<translated field>_<language suffix>` for an extra site language; values are strings up to 64 KB, at most 10. Used by editors for instant previews. Requires permission to update the record or `settings.manage`; limited to 300 renders per minute per user.",
    security(("session_cookie" = [])),
    request_body = RenderSitePreview,
    responses(
        (status = 200, description = "Rendered page", body = Data<SitePreviewRenderResponse>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "No permission to update the record", body = ErrorEnvelope),
        (status = 404, description = "The record or the page does not exist", body = ErrorEnvelope),
        (status = 422, description = "Invalid record, path or values", body = ErrorEnvelope),
        (status = 429, description = "Too many renders (`rate_limited`)", body = ErrorEnvelope),
    )
)]
async fn render_preview(
    State(state): State<AppState>,
    tenant: AuthContext,
    Json(input): Json<RenderSitePreview>,
) -> Result<Json<Data<SitePreviewRenderResponse>>, ApiError> {
    PREVIEW_RENDER_LIMIT
        .check(tenant.scope.user_id().as_str(), PREVIEW_RATE_LIMITED)
        .await?;
    let path = crate::preview::local_path(&input.path).ok_or_else(|| {
        AppError::Validation("path must be a site path such as /about-us".to_owned())
    })?;
    let config = state.organization_config.snapshot();
    let settings = site_settings(&config);
    let runtime = runtime::active(&state).await?;
    let mut record = preview_record(
        &state,
        &tenant,
        &runtime,
        &StartSitePreviewRecord {
            record_type: input.record.record_type.clone(),
            record_id: input.record.record_id.clone(),
            fields: std::collections::BTreeMap::new(),
        },
    )
    .await?;
    if input.record.values.len() > MAX_PREVIEW_FIELDS {
        return Err(AppError::Validation(format!(
            "values replaces at most {MAX_PREVIEW_FIELDS} fields"
        ))
        .into());
    }
    let exposure = runtime
        .entity_exposure(&record.record_type)
        .ok_or_else(|| {
            AppError::Validation(format!("{} has no public pages", record.record_type))
        })?;
    for (target, value) in &input.record.values {
        if !preview_value_target(exposure, &settings.languages, target) {
            return Err(AppError::Validation(format!(
                "{target} is not a public field that a preview can replace"
            ))
            .into());
        }
        let size = match value {
            serde_json::Value::String(text) => text.len(),
            serde_json::Value::Null => 0,
            _ => {
                return Err(AppError::Validation(format!("{target} must be text")).into());
            }
        };
        if size > MAX_PREVIEW_VALUE_BYTES {
            return Err(AppError::Validation(format!(
                "{target} has more than {MAX_PREVIEW_VALUE_BYTES} bytes"
            ))
            .into());
        }
    }
    record.values = input.record.values;
    let previewing = crate::preview::Preview {
        theme: None,
        record: Some(record),
        appearance: None,
    };
    let html = crate::handler::render_draft(&state, &runtime, &previewing, path)
        .await?
        .ok_or_else(|| AppError::NotFound("no site page answers this path".to_owned()))?;
    Ok(Json(Data::new(SitePreviewRenderResponse { html })))
}

async fn preview_record(
    state: &AppState,
    tenant: &AuthContext,
    runtime: &runtime::SiteRuntime,
    input: &StartSitePreviewRecord,
) -> Result<crate::preview::PreviewRecord, AppError> {
    let exposure = runtime.entity_exposure(&input.record_type).ok_or_else(|| {
        AppError::Validation(format!("{} has no public pages", input.record_type))
    })?;
    let id = uuid::Uuid::parse_str(&input.record_id)
        .map_err(|_| AppError::Validation("record_id must be a record id".to_owned()))?;
    if input.fields.len() > MAX_PREVIEW_FIELDS {
        return Err(AppError::Validation(format!(
            "fields replaces at most {MAX_PREVIEW_FIELDS} fields"
        )));
    }
    let registry = state.registry().await?;
    let may_update = registry
        .require(&input.record_type)?
        .action_permission("update")
        .is_some_and(|permission| tenant.scope.require(&permission).is_ok());
    if !may_update && require_manage(&tenant.scope).is_err() {
        return Err(AppError::Forbidden(
            "previewing this record needs permission to change it".to_owned(),
        ));
    }
    let loaded = registry
        .handler(&input.record_type)?
        .load(
            &aphel_kernel::records::RecordCtx::new(&tenant.scope, state),
            id,
        )
        .await?
        .ok_or_else(|| AppError::NotFound("the record does not exist".to_owned()))?;
    let readable = |field: &str| {
        loaded.get(field).is_some()
            || loaded
                .get("custom_fields")
                .is_some_and(|custom| custom.get(field).is_some())
    };
    for (target, source) in &input.fields {
        if !exposure.exposes(target)
            || aphel_contract_site::ALWAYS_PUBLIC_FIELDS.contains(&target.as_str())
        {
            return Err(AppError::Validation(format!(
                "{target} is not a public field that a draft can replace"
            )));
        }
        if !readable(source) {
            return Err(AppError::Validation(format!(
                "{source} is not a field of the record"
            )));
        }
    }
    Ok(crate::preview::PreviewRecord {
        record_type: input.record_type.clone(),
        record_id: id.to_string(),
        fields: input.fields.clone(),
        values: std::collections::BTreeMap::new(),
    })
}

#[utoipa::path(
    get,
    path = "/site/embeds",
    tag = TAG,
    operation_id = "site_embeds_list",
    description = "What site pages can embed with the template function `embed(kind, key)`: for each kind contributed by an enabled module (such as `form` or `booking`), the active items with their key and label. Only things that are already public are listed. Any signed-in user.",
    security(("session_cookie" = [])),
    responses(
        (status = 200, description = "Embeddable kinds", body = Data<Vec<SiteEmbedSourceResponse>>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
    )
)]
async fn list_embeds(
    State(state): State<AppState>,
    _tenant: AuthContext,
) -> Result<Json<Data<Vec<SiteEmbedSourceResponse>>>, ApiError> {
    let mut kinds = Vec::new();
    for source in state
        .app
        .contributions::<dyn aphel_contract_site::SiteEmbedSource>()
    {
        kinds.push(SiteEmbedSourceResponse {
            kind: source.kind().to_owned(),
            options: source
                .options(&state)
                .await?
                .into_iter()
                .map(|option| SiteEmbedOptionResponse {
                    key: option.key,
                    label: option.label,
                })
                .collect(),
        });
    }
    Ok(Json(Data::new(kinds)))
}

#[utoipa::path(
    get,
    path = "/site/collections",
    tag = TAG,
    operation_id = "site_collections_list",
    description = "Public entities whose owner ships a card template (`site/cards/<entity>.html`), so pages can list their published records with `site_list(owner, entity, sort=, limit=)`; `fields` are the public fields usable in `sort`. Any signed-in user.",
    security(("session_cookie" = [])),
    responses(
        (status = 200, description = "Collections", body = Data<Vec<SiteCollectionResponse>>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
    )
)]
async fn list_collections(
    State(state): State<AppState>,
    _tenant: AuthContext,
) -> Result<Json<Data<Vec<SiteCollectionResponse>>>, ApiError> {
    let runtime = runtime::active(&state).await?;
    let registry = state.registry().await?;
    let collections = runtime
        .exposures()
        .filter_map(|(owner, exposure)| {
            let entity = exposure.entity.strip_prefix(&format!("{owner}."))?;
            runtime
                .environment
                .get_template(&crate::deferred::card_template(owner, entity))
                .ok()?;
            let label = registry
                .require(&exposure.entity)
                .ok()
                .and_then(|definition| {
                    definition
                        .custom
                        .as_ref()
                        .map(|labels| labels.plural.clone())
                })
                .unwrap_or_else(|| exposure.entity.clone());
            let mut fields: Vec<String> = aphel_contract_site::ALWAYS_PUBLIC_FIELDS
                .iter()
                .map(|field| (*field).to_owned())
                .chain(exposure.fields.iter().cloned())
                .collect();
            fields.dedup();
            Some(SiteCollectionResponse {
                owner: owner.to_owned(),
                entity: entity.to_owned(),
                record_type: exposure.entity.clone(),
                label,
                fields,
            })
        })
        .collect();
    Ok(Json(Data::new(collections)))
}

#[utoipa::path(
    get,
    path = "/site/languages",
    tag = TAG,
    operation_id = "site_languages_list",
    description = "Languages the site answers in: the organization language first (`default`, no address prefix) and the extra languages chosen in Site → Languages, each with its address prefix and the suffix of the field variants that `translated` public fields read (`title_en`). Any signed-in user.",
    security(("session_cookie" = [])),
    responses(
        (status = 200, description = "Languages", body = Data<Vec<SiteLanguageResponse>>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
    )
)]
async fn list_languages(
    State(state): State<AppState>,
    _tenant: AuthContext,
) -> Result<Json<Data<Vec<SiteLanguageResponse>>>, ApiError> {
    use aphel_sdk::platform::application::organization_profile::OrganizationRepository;
    let default_language = aphel_sdk::platform::infrastructure::repositories::mysql_organization_profile::MySqlOrganizationRepository::new(state.db()?)
        .find()
        .await?
        .map(|organization| organization.locale)
        .unwrap_or_default();
    let settings = site_settings(&state.organization_config.snapshot());
    let languages = std::iter::once(SiteLanguageResponse {
        prefix: String::new(),
        field_suffix: crate::settings::field_suffix(&default_language),
        code: default_language.clone(),
        default: true,
    })
    .chain(
        settings
            .languages
            .iter()
            .filter(|code| !code.eq_ignore_ascii_case(&default_language))
            .map(|code| SiteLanguageResponse {
                prefix: format!("/{}", crate::settings::language_slug(code)),
                field_suffix: crate::settings::field_suffix(code),
                code: code.clone(),
                default: false,
            }),
    )
    .collect();
    Ok(Json(Data::new(languages)))
}
