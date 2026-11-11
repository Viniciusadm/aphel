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
        .routes(routes!(overview))
        .routes(routes!(list_redirects))
        .routes(routes!(delete_redirect))
        .routes(routes!(list_path_redirects, create_path_redirect))
        .routes(routes!(delete_path_redirect))
        .routes(routes!(start_preview))
        .routes(routes!(purge_cache))
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
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct SitePreviewResponse {
    #[schema(example = "https://www.example.com/_aphel/site/preview/3f2a...")]
    pub url: String,
    pub expires_in_minutes: i64,
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
    description = "What the public site offers right now: the themes of installed plugins, the menu items they contribute (in declaration order; the order and visibility chosen by the organization live in the `site` organization setting) and the absolute site address. Requires `settings.manage`.",
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
    let theme = site_settings(&config).theme;
    let runtime = runtime::current(&state.app, theme.as_deref()).await?;
    let default_language = {
        use aphel_sdk::platform::application::organization_profile::OrganizationRepository;
        aphel_sdk::platform::infrastructure::repositories::mysql_organization_profile::MySqlOrganizationRepository::new(state.db()?)
            .find()
            .await?
            .map(|organization| organization.locale)
            .unwrap_or_default()
    };
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
            .map(|item| SiteMenuItemResponse {
                id: item.id.clone(),
                label: item.item.label.clone(),
                path: item.path.clone(),
            })
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
    let theme = site_settings(&state.organization_config.snapshot()).theme;
    let runtime = runtime::current(&state.app, theme.as_deref()).await?;
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
    description = "Creates a link that shows the site with another theme (`theme` null for the built-in one) in the browser that opens it, for 30 minutes, without changing the site for visitors. Preview pages are not cached and carry a banner with a link that ends the preview. Requires `settings.manage`.",
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
    require_manage(&tenant.scope)?;
    let config = state.organization_config.snapshot();
    let theme = input
        .theme
        .as_deref()
        .map(str::trim)
        .filter(|theme| !theme.is_empty());
    if let Some(theme) = theme {
        let current = site_settings(&config).theme;
        let runtime = runtime::current(&state.app, current.as_deref()).await?;
        if !runtime
            .themes
            .iter()
            .any(|candidate| candidate.key == theme)
        {
            return Err(AppError::Validation(format!("the theme {theme} is not installed")).into());
        }
    }
    let token = crate::preview::start(&state.db()?, theme).await?;
    Ok((
        StatusCode::CREATED,
        Json(Data::new(SitePreviewResponse {
            url: crate::preview::link(&config.site_base_url(), &token),
            expires_in_minutes: crate::preview::PREVIEW_MINUTES,
        })),
    ))
}
