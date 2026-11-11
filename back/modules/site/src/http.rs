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
use serde::Serialize;
use serde_json::json;
use sqlx::Row;
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::cache::{self, ALL_TAG};
use crate::runtime;
use crate::settings::site_settings;

pub(crate) const TAG: &str = "site";
const REDIRECT_DELETED: &str = "site.redirect_deleted";
const CACHE_PURGED: &str = "site.cache_purged";
const MAX_REDIRECTS: i64 = 500;

pub(crate) fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(overview))
        .routes(routes!(list_redirects))
        .routes(routes!(delete_redirect))
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
