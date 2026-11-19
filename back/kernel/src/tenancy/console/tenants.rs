use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use chrono::Utc;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::api::dto::common::{Data, ErrorEnvelope, Page};
use crate::api::error::ApiError;
use crate::api::extractors::{ClientInfo, JsonBody, Pagination, PaginationQuery};
use crate::application::installer::{InstallCommand, InstallerBlueprintSource};
use crate::application::system::WORKER_ROLE;
use crate::application::system::admin::{SystemActor, SystemAdminUseCases};
use crate::domain::access::AccessScope;
use crate::domain::{Secret, UserId};
use crate::errors::AppError;
use crate::infrastructure::system::ports::MySqlSystemAdminPorts;
use crate::tenancy::console::dto::{
    AuditEntryResponse, CatalogModuleResponse, CatalogResponse, CreateTenantRequest,
    DeleteTenantRequest, HostRequest, QueueTotalsResponse, TenantFeaturesRequest,
    TenantHealthResponse, TenantResponse, WorkerResponse,
};
use crate::tenancy::console::operators::validate_password;
use crate::tenancy::console::session::actor;
use crate::tenancy::console::{OperatorSession, Shared};
use crate::tenancy::{
    TenantRegistration, TenantRuntime, add_host, begin_deletion, delete_tenant, export_available,
    list_tenants, provision_tenant, register_tenant, remove_host, resume_tenant, scope,
    set_plugin_management, set_primary_host, suspend_tenant, tenant_summary,
};

const TAG: &str = "console";

pub(super) fn router() -> OpenApiRouter<Shared> {
    OpenApiRouter::new()
        .routes(routes!(list, create))
        .routes(routes!(show, delete))
        .routes(routes!(suspend))
        .routes(routes!(update_features))
        .routes(routes!(resume))
        .routes(routes!(add_tenant_host))
        .routes(routes!(remove_tenant_host))
        .routes(routes!(make_primary))
        .routes(routes!(health))
        .routes(routes!(catalog))
        .routes(routes!(audit))
}

async fn summary(state: &Shared, slug: &str) -> Result<Json<Data<TenantResponse>>, ApiError> {
    Ok(Json(Data::new(
        tenant_summary(&state.control, slug).await?.into(),
    )))
}

#[utoipa::path(
    get,
    path = "/tenants",
    tag = TAG,
    operation_id = "console_list_tenants",
    responses(
        (status = 200, description = "Every tenant with its hosts, state, database size and last write", body = Data<Vec<TenantResponse>>),
        (status = 401, description = "No console session", body = ErrorEnvelope),
    )
)]
async fn list(
    State(state): State<Shared>,
    _session: OperatorSession,
) -> Result<Json<Data<Vec<TenantResponse>>>, ApiError> {
    let tenants = list_tenants(&state.control).await?;
    Ok(Json(Data::new(
        tenants.into_iter().map(TenantResponse::from).collect(),
    )))
}

#[utoipa::path(
    get,
    path = "/tenants/{slug}",
    tag = TAG,
    operation_id = "console_tenant",
    params(("slug" = String, Path, description = "Tenant slug")),
    responses(
        (status = 200, description = "The tenant", body = Data<TenantResponse>),
        (status = 401, description = "No console session", body = ErrorEnvelope),
        (status = 404, description = "Unknown tenant (`tenant_not_found`)", body = ErrorEnvelope),
    )
)]
async fn show(
    State(state): State<Shared>,
    _session: OperatorSession,
    Path(slug): Path<String>,
) -> Result<Json<Data<TenantResponse>>, ApiError> {
    summary(&state, &slug).await
}

#[utoipa::path(
    post,
    path = "/tenants",
    tag = TAG,
    operation_id = "console_create_tenant",
    request_body = CreateTenantRequest,
    responses(
        (status = 202, description = "Tenant registered as `provisioning`; the database, migrations and installation run in the background and the tenant turns `active`, or keeps `last_error`", body = Data<TenantResponse>),
        (status = 401, description = "No console session", body = ErrorEnvelope),
        (status = 409, description = "Slug or host already used (`tenant_slug_taken`, `tenant_host_taken`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid slug, name, host or administrator password (`tenant_slug_invalid`, `tenant_name_invalid`, `tenant_host_invalid`, `operator_password_invalid`)", body = ErrorEnvelope),
    )
)]
async fn create(
    State(state): State<Shared>,
    session: OperatorSession,
    client: ClientInfo,
    JsonBody(request): JsonBody<CreateTenantRequest>,
) -> Result<(StatusCode, Json<Data<TenantResponse>>), ApiError> {
    validate_password(&request.admin_password)?;
    let business = request
        .blueprint
        .as_ref()
        .filter(|key| !key.is_empty())
        .map(|key| serde_json::json!({ "business_type": key }));
    crate::application::installer::modules::prepare(
        crate::modules::app(),
        &request.modules,
        &Default::default(),
        business.as_ref(),
    )?;
    let tenant = register_tenant(
        &state.control,
        &actor(&session, &client),
        TenantRegistration {
            slug: request.slug.trim().to_owned(),
            name: request.name.clone(),
            hosts: vec![request.host.clone()],
        },
    )
    .await?;
    let install = InstallCommand {
        organization_name: request.name,
        timezone: request.timezone,
        currency: request.currency,
        locale: request.locale,
        admin_name: request.admin_name,
        admin_email: request.admin_email,
        admin_password: Secret::new(request.admin_password),
        public_base_url: crate::tenancy::normalize_host(&request.host)
            .map(|host| format!("https://{host}")),
        business: request
            .blueprint
            .filter(|blueprint| !blueprint.is_empty())
            .map(|blueprint| serde_json::json!({ "business_type": blueprint })),
        ip: client.0.ip.clone(),
        modules: request.modules,
        module_settings: Default::default(),
    };
    let background = state.clone();
    let provisioned = tenant.clone();
    tokio::spawn(async move {
        if let Err(error) = provision_tenant(
            &background.control,
            &background.config,
            &provisioned,
            install,
        )
        .await
        {
            tracing::error!(
                tenant = provisioned.slug,
                "tenant provisioning failed: {error}"
            );
        }
        background.refresh_directory().await;
    });
    let Json(body) = summary(&state, &tenant.slug).await?;
    Ok((StatusCode::ACCEPTED, Json(body)))
}

#[utoipa::path(
    post,
    path = "/tenants/{slug}/suspend",
    tag = TAG,
    operation_id = "console_suspend_tenant",
    params(("slug" = String, Path, description = "Tenant slug")),
    responses(
        (status = 200, description = "Tenant suspended: its hosts answer 503 and its jobs stop", body = Data<TenantResponse>),
        (status = 401, description = "No console session", body = ErrorEnvelope),
        (status = 409, description = "The tenant is not active (`tenant_status_conflict`)", body = ErrorEnvelope),
    )
)]
async fn suspend(
    State(state): State<Shared>,
    session: OperatorSession,
    client: ClientInfo,
    Path(slug): Path<String>,
) -> Result<Json<Data<TenantResponse>>, ApiError> {
    suspend_tenant(&state.control, &actor(&session, &client), &slug).await?;
    state.refresh_directory().await;
    summary(&state, &slug).await
}

#[utoipa::path(
    post,
    path = "/tenants/{slug}/resume",
    tag = TAG,
    operation_id = "console_resume_tenant",
    params(("slug" = String, Path, description = "Tenant slug")),
    responses(
        (status = 200, description = "Tenant served again", body = Data<TenantResponse>),
        (status = 401, description = "No console session", body = ErrorEnvelope),
        (status = 409, description = "The tenant is not suspended (`tenant_status_conflict`)", body = ErrorEnvelope),
    )
)]
async fn resume(
    State(state): State<Shared>,
    session: OperatorSession,
    client: ClientInfo,
    Path(slug): Path<String>,
) -> Result<Json<Data<TenantResponse>>, ApiError> {
    resume_tenant(&state.control, &actor(&session, &client), &slug).await?;
    state.refresh_directory().await;
    summary(&state, &slug).await
}

#[utoipa::path(
    put,
    path = "/tenants/{slug}/features",
    tag = TAG,
    operation_id = "console_update_tenant_features",
    params(("slug" = String, Path, description = "Tenant slug")),
    request_body = TenantFeaturesRequest,
    responses(
        (status = 200, description = "Features saved. With `plugin_management` off, the organization cannot open the plugins screen or install, enable, disable, configure or remove plugins (`plugin_management_disabled`); enabled plugins keep running", body = Data<TenantResponse>),
        (status = 401, description = "No console session", body = ErrorEnvelope),
        (status = 404, description = "Unknown tenant (`tenant_not_found`)", body = ErrorEnvelope),
    )
)]
async fn update_features(
    State(state): State<Shared>,
    session: OperatorSession,
    client: ClientInfo,
    Path(slug): Path<String>,
    JsonBody(request): JsonBody<TenantFeaturesRequest>,
) -> Result<Json<Data<TenantResponse>>, ApiError> {
    set_plugin_management(
        &state.control,
        &actor(&session, &client),
        &slug,
        request.plugin_management,
    )
    .await?;
    state.refresh_directory().await;
    summary(&state, &slug).await
}

#[utoipa::path(
    post,
    path = "/tenants/{slug}/hosts",
    tag = TAG,
    operation_id = "console_add_tenant_host",
    params(("slug" = String, Path, description = "Tenant slug")),
    request_body = HostRequest,
    responses(
        (status = 201, description = "Host added", body = Data<TenantResponse>),
        (status = 401, description = "No console session", body = ErrorEnvelope),
        (status = 409, description = "Host already used (`tenant_host_taken`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid host (`tenant_host_invalid`)", body = ErrorEnvelope),
    )
)]
async fn add_tenant_host(
    State(state): State<Shared>,
    session: OperatorSession,
    client: ClientInfo,
    Path(slug): Path<String>,
    JsonBody(request): JsonBody<HostRequest>,
) -> Result<(StatusCode, Json<Data<TenantResponse>>), ApiError> {
    add_host(
        &state.control,
        &actor(&session, &client),
        &slug,
        &request.host,
    )
    .await?;
    state.refresh_directory().await;
    Ok((StatusCode::CREATED, summary(&state, &slug).await?))
}

#[utoipa::path(
    delete,
    path = "/tenants/{slug}/hosts/{host}",
    tag = TAG,
    operation_id = "console_remove_tenant_host",
    params(
        ("slug" = String, Path, description = "Tenant slug"),
        ("host" = String, Path, description = "Host to remove"),
    ),
    responses(
        (status = 200, description = "Host removed; the next host becomes primary when the primary is removed", body = Data<TenantResponse>),
        (status = 401, description = "No console session", body = ErrorEnvelope),
        (status = 404, description = "Unknown host (`tenant_host_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "It is the only host (`tenant_last_host`)", body = ErrorEnvelope),
    )
)]
async fn remove_tenant_host(
    State(state): State<Shared>,
    session: OperatorSession,
    client: ClientInfo,
    Path((slug, host)): Path<(String, String)>,
) -> Result<Json<Data<TenantResponse>>, ApiError> {
    remove_host(&state.control, &actor(&session, &client), &slug, &host).await?;
    state.refresh_directory().await;
    summary(&state, &slug).await
}

#[utoipa::path(
    post,
    path = "/tenants/{slug}/hosts/{host}/primary",
    tag = TAG,
    operation_id = "console_set_primary_tenant_host",
    params(
        ("slug" = String, Path, description = "Tenant slug"),
        ("host" = String, Path, description = "Host to make primary"),
    ),
    responses(
        (status = 200, description = "Primary host changed", body = Data<TenantResponse>),
        (status = 401, description = "No console session", body = ErrorEnvelope),
        (status = 404, description = "Unknown host (`tenant_host_not_found`)", body = ErrorEnvelope),
    )
)]
async fn make_primary(
    State(state): State<Shared>,
    session: OperatorSession,
    client: ClientInfo,
    Path((slug, host)): Path<(String, String)>,
) -> Result<Json<Data<TenantResponse>>, ApiError> {
    set_primary_host(&state.control, &actor(&session, &client), &slug, &host).await?;
    summary(&state, &slug).await
}

#[utoipa::path(
    delete,
    path = "/tenants/{slug}",
    tag = TAG,
    operation_id = "console_delete_tenant",
    params(("slug" = String, Path, description = "Tenant slug")),
    request_body = DeleteTenantRequest,
    responses(
        (status = 202, description = "Tenant marked `deleting`; the export (into TENANT_EXPORT_DIR), file removal and database drop run in the background", body = Data<TenantResponse>),
        (status = 401, description = "No console session", body = ErrorEnvelope),
        (status = 404, description = "Unknown tenant (`tenant_not_found`)", body = ErrorEnvelope),
        (status = 422, description = "`confirm` does not repeat the slug (`tenant_deletion_unconfirmed`)", body = ErrorEnvelope),
        (status = 503, description = "Export asked but this build cannot export (`configuration_error`)", body = ErrorEnvelope),
    )
)]
async fn delete(
    State(state): State<Shared>,
    session: OperatorSession,
    client: ClientInfo,
    Path(slug): Path<String>,
    JsonBody(request): JsonBody<DeleteTenantRequest>,
) -> Result<(StatusCode, Json<Data<TenantResponse>>), ApiError> {
    if request.confirm.trim() != slug {
        return Err(AppError::validation_code(
            "tenant_deletion_unconfirmed",
            "type the tenant slug to confirm the deletion",
        )
        .into());
    }
    let actor = actor(&session, &client);
    let export = request.export.then(|| {
        state
            .export_dir
            .join(format!("{slug}-{}.zip", Utc::now().format("%Y%m%d-%H%M%S")))
    });
    let tenant = begin_deletion(&state.control, &actor, &slug, export.as_deref()).await?;
    state.refresh_directory().await;
    let Json(body) = summary(&state, &slug).await?;
    let background = state.clone();
    tokio::spawn(async move {
        match delete_tenant(
            &background.control,
            &background.config,
            &actor,
            &tenant,
            export.as_deref(),
        )
        .await
        {
            Ok(removed) => {
                tracing::info!(tenant = tenant.slug, removed, "tenant deleted")
            }
            Err(error) => tracing::error!(tenant = tenant.slug, "tenant deletion failed: {error}"),
        }
        background.refresh_directory().await;
    });
    Ok((StatusCode::ACCEPTED, Json(body)))
}

#[utoipa::path(
    get,
    path = "/tenants/{slug}/health",
    tag = TAG,
    operation_id = "console_tenant_health",
    params(("slug" = String, Path, description = "Tenant slug")),
    responses(
        (status = 200, description = "Jobs, workers, pending migrations, plugin backend errors and recent failures of an active tenant; `available` is false otherwise", body = Data<TenantHealthResponse>),
        (status = 401, description = "No console session", body = ErrorEnvelope),
        (status = 404, description = "Unknown tenant (`tenant_not_found`)", body = ErrorEnvelope),
    )
)]
async fn health(
    State(state): State<Shared>,
    _session: OperatorSession,
    Path(slug): Path<String>,
) -> Result<Json<Data<TenantHealthResponse>>, ApiError> {
    let tenant = state.control.require_tenant(&slug).await?;
    if !tenant.status.serves_requests() {
        return Ok(Json(Data::new(TenantHealthResponse {
            error: Some(format!("the tenant is {}", tenant.status.as_str())),
            ..TenantHealthResponse::default()
        })));
    }
    let report = match state.runtimes.get(&Arc::new(tenant)).await {
        Ok(runtime) => scope(runtime.clone(), collect_health(runtime)).await,
        Err(error) => Err(error),
    };
    Ok(Json(Data::new(report.unwrap_or_else(|error| {
        TenantHealthResponse {
            error: Some(error.to_string()),
            ..TenantHealthResponse::default()
        }
    }))))
}

async fn collect_health(runtime: Arc<TenantRuntime>) -> Result<TenantHealthResponse, AppError> {
    let pool = runtime
        .state()
        .ok_or_else(|| AppError::Internal("the tenant state is not ready".to_owned()))?
        .db()?;
    let use_cases = SystemAdminUseCases::new(MySqlSystemAdminPorts::new(pool.clone()));
    let actor = SystemActor {
        scope: AccessScope::system(),
        user_id: UserId::system(),
        ip: None,
    };
    let queue = use_cases.queue_overview(&actor).await?;
    let workers = use_cases
        .heartbeats(&actor)
        .await?
        .into_iter()
        .filter(|beat| beat.role == WORKER_ROLE)
        .map(|beat| WorkerResponse {
            instance_id: beat.instance_id,
            hostname: beat.hostname,
            version: beat.version,
            last_seen_at: crate::api::dto::common::timestamp(beat.last_seen_at),
        })
        .collect();
    let pending_migrations = use_cases.migrations(&actor).await?.pending_count() as u64;
    let failures = use_cases.recent_failures(&actor).await?;
    let plugin_backend_errors = crate::modules::runtime_catalog::backend_errors(&pool)
        .await
        .unwrap_or_default();
    Ok(TenantHealthResponse {
        available: true,
        error: None,
        queue: QueueTotalsResponse {
            ready: queue.totals.ready,
            scheduled: queue.totals.scheduled,
            running: queue.totals.running,
            failed: queue.totals.failed,
        },
        oldest_ready_age_seconds: queue.oldest_ready_age_seconds,
        workers,
        pending_migrations,
        plugin_backend_errors,
        recent_failures: (failures.automations.len()
            + failures.webhooks.len()
            + failures.integrations.len()) as u64,
    })
}

#[utoipa::path(
    get,
    path = "/catalog",
    tag = TAG,
    operation_id = "console_catalog",
    responses(
        (status = 200, description = "Optional modules and blueprints a new tenant can start with, and whether tenants can be exported", body = Data<CatalogResponse>),
        (status = 401, description = "No console session", body = ErrorEnvelope),
    )
)]
async fn catalog(_session: OperatorSession) -> Json<Data<CatalogResponse>> {
    let app = crate::modules::app();
    Json(Data::new(CatalogResponse {
        server_public_ip: std::env::var("APHEL_PUBLIC_IP")
            .ok()
            .and_then(|value| value.trim().parse::<std::net::IpAddr>().ok())
            .map(|ip| ip.to_string()),
        modules: app
            .modules()
            .iter()
            .filter(|entry| !entry.manifest.required && entry.is_compatible())
            .map(|entry| CatalogModuleResponse {
                requires_configuration: entry
                    .manifest
                    .settings
                    .iter()
                    .any(|field| field.required && field.default.is_none()),
                key: entry.key().to_owned(),
                name: entry.manifest.name.to_owned(),
                description: entry.manifest.description.to_owned(),
                dependencies: entry
                    .manifest
                    .dependencies
                    .iter()
                    .map(|dependency| dependency.key.to_owned())
                    .collect(),
            })
            .collect(),
        blueprints: app
            .all_contributions::<dyn InstallerBlueprintSource>()
            .into_iter()
            .flat_map(|(_, source)| source.profiles())
            .collect(),
        export_available: export_available(),
    }))
}

#[utoipa::path(
    get,
    path = "/audit",
    tag = TAG,
    operation_id = "console_audit",
    params(PaginationQuery),
    responses(
        (status = 200, description = "Operator actions, newest first", body = Page<AuditEntryResponse>),
        (status = 401, description = "No console session", body = ErrorEnvelope),
    )
)]
async fn audit(
    State(state): State<Shared>,
    _session: OperatorSession,
    pagination: Pagination,
) -> Result<Json<Page<AuditEntryResponse>>, ApiError> {
    let pool = state.control.pool();
    let total = crate::tenancy::store::audit::count(pool).await?;
    let entries = crate::tenancy::store::audit::page(pool, pagination.limit(), pagination.offset())
        .await?
        .into_iter()
        .map(|entry| AuditEntryResponse {
            id: entry.id,
            operator_email: entry.operator_email,
            action: entry.action,
            tenant_id: entry.tenant_id,
            details: entry.details,
            ip: entry.ip,
            occurred_at: crate::api::dto::common::timestamp(entry.occurred_at.and_utc()),
        })
        .collect();
    Ok(Json(Page::new(
        entries,
        u64::try_from(total).unwrap_or_default(),
        pagination,
    )))
}
