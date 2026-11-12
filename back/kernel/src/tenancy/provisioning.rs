use std::collections::HashMap;
use std::ffi::OsString;
use std::path::Path;
use std::sync::Arc;

use chrono::{DateTime, NaiveDateTime, Utc};
use serde_json::json;
use uuid::Uuid;

use crate::application::fields::RecordTypeRegistry;
use crate::application::installer::InstallCommand;
use crate::config::AppConfig;
use crate::database;
use crate::errors::AppError;
use crate::infrastructure::custom_objects::sync_custom_object_fields;
use crate::infrastructure::fields::sync_system_fields;
use crate::modules::ModuleCommand;
use crate::tenancy::audit::{OperatorActor, record_operator_action};
use crate::tenancy::{ControlPlane, Tenant, TenantRuntime, TenantStatus, normalize_host, scope};

const SLUG_MIN: usize = 3;
const SLUG_MAX: usize = 40;
const NAME_MAX: usize = 140;
const DATABASE_NAME_MAX: usize = 64;
const EXPORT_COMMAND: &str = "org";

pub struct TenantRegistration {
    pub slug: String,
    pub name: String,
    pub hosts: Vec<String>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct MigrationSummary {
    pub migrated: usize,
    pub failed: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TenantHost {
    pub host: String,
    pub is_primary: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TenantSummary {
    pub tenant: Tenant,
    pub hosts: Vec<TenantHost>,
    pub database_bytes: Option<u64>,
    pub last_write_at: Option<DateTime<Utc>>,
    pub migrated_at: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
    pub created_at: DateTime<Utc>,
}

pub fn validate_slug(slug: &str) -> Result<(), AppError> {
    let valid = (SLUG_MIN..=SLUG_MAX).contains(&slug.len())
        && slug.chars().all(|character| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || character == '-'
        })
        && !slug.starts_with('-')
        && !slug.ends_with('-')
        && !slug.contains("--");
    if valid {
        Ok(())
    } else {
        Err(AppError::validation_code(
            "tenant_slug_invalid",
            format!(
                "the slug must have {SLUG_MIN} to {SLUG_MAX} lowercase letters, digits or single hyphens inside"
            ),
        ))
    }
}

fn validate_host(host: &str) -> Result<String, AppError> {
    normalize_host(host).ok_or_else(|| {
        AppError::validation_code(
            "tenant_host_invalid",
            format!("{host:?} is not a valid host"),
        )
    })
}

pub fn export_available() -> bool {
    export_command().is_some()
}

fn export_command() -> Option<Arc<dyn ModuleCommand>> {
    crate::modules::app()
        .all_contributions::<dyn ModuleCommand>()
        .into_iter()
        .map(|(_, command)| command)
        .find(|command| command.command().get_name() == EXPORT_COMMAND)
}

pub async fn list_tenants(control: &ControlPlane) -> Result<Vec<TenantSummary>, AppError> {
    type Row = (
        String,
        String,
        Option<NaiveDateTime>,
        Option<String>,
        NaiveDateTime,
    );
    let rows: Vec<Row> = sqlx::query_as(
        "SELECT id, slug, migrated_at, last_error, created_at FROM tenants ORDER BY slug",
    )
    .fetch_all(control.pool())
    .await?;
    let hosts: Vec<(String, String, bool)> = sqlx::query_as(
        "SELECT tenant_id, host, is_primary FROM tenant_hosts ORDER BY is_primary DESC, host",
    )
    .fetch_all(control.pool())
    .await?;
    let sizes: Vec<(String, Option<u64>, Option<NaiveDateTime>)> = sqlx::query_as(
        "SELECT table_schema, CAST(SUM(data_length + index_length) AS UNSIGNED), MAX(update_time) \
         FROM information_schema.tables WHERE table_schema IN (SELECT database_name FROM tenants) \
         GROUP BY table_schema",
    )
    .fetch_all(control.pool())
    .await
    .unwrap_or_default();
    let sizes: HashMap<String, (Option<u64>, Option<NaiveDateTime>)> = sizes
        .into_iter()
        .map(|(schema, bytes, updated)| (schema, (bytes, updated)))
        .collect();
    let mut summaries = Vec::with_capacity(rows.len());
    for (id, slug, migrated_at, last_error, created_at) in rows {
        let tenant = control.require_tenant(&slug).await?;
        let (database_bytes, last_write_at) = sizes
            .get(&tenant.database_name)
            .cloned()
            .unwrap_or_default();
        summaries.push(TenantSummary {
            hosts: hosts
                .iter()
                .filter(|(tenant_id, _, _)| *tenant_id == id)
                .map(|(_, host, is_primary)| TenantHost {
                    host: host.clone(),
                    is_primary: *is_primary,
                })
                .collect(),
            tenant,
            database_bytes,
            last_write_at: last_write_at.map(|at| at.and_utc()),
            migrated_at: migrated_at.map(|at| at.and_utc()),
            last_error,
            created_at: created_at.and_utc(),
        });
    }
    Ok(summaries)
}

pub async fn tenant_summary(control: &ControlPlane, slug: &str) -> Result<TenantSummary, AppError> {
    list_tenants(control)
        .await?
        .into_iter()
        .find(|summary| summary.tenant.slug == slug)
        .ok_or_else(|| {
            AppError::not_found_code(
                "tenant_not_found",
                format!("no tenant has the slug {slug:?}"),
            )
        })
}

pub async fn migrate_tenant(control: &ControlPlane, tenant: &Tenant) -> Result<(), AppError> {
    let result = run_tenant_migrations(control, tenant).await;
    let recorded =
        match &result {
            Ok(()) => sqlx::query(
                "UPDATE tenants SET migrated_at = UTC_TIMESTAMP(6), last_error = NULL WHERE id = ?",
            )
            .bind(tenant.id.to_string())
            .execute(control.pool())
            .await,
            Err(error) => {
                sqlx::query("UPDATE tenants SET last_error = ? WHERE id = ?")
                    .bind(format!("migration failed: {error}"))
                    .bind(tenant.id.to_string())
                    .execute(control.pool())
                    .await
            }
        };
    if let Err(error) = recorded {
        tracing::error!(
            tenant = tenant.slug,
            "could not record the migration state: {error}"
        );
    }
    result
}

async fn run_tenant_migrations(control: &ControlPlane, tenant: &Tenant) -> Result<(), AppError> {
    let pool = control.connect_tenant(tenant).await?;
    let runtime = TenantRuntime::bare(Arc::new(tenant.clone()));
    let result = scope(runtime.clone(), async {
        database::migrations::run(&pool).await?;
        sync_system_fields(&pool, &RecordTypeRegistry::builtin()).await?;
        sync_custom_object_fields(&pool).await?;
        Ok(())
    })
    .await;
    runtime.stop();
    pool.close().await;
    result
}

pub async fn migrate_all(control: &ControlPlane) -> Result<MigrationSummary, AppError> {
    control.migrate().await?;
    let slugs: Vec<String> = sqlx::query_scalar(
        "SELECT slug FROM tenants WHERE status IN ('active', 'suspended') ORDER BY slug",
    )
    .fetch_all(control.pool())
    .await?;
    let mut summary = MigrationSummary::default();
    for slug in slugs {
        let Some(tenant) = control.find_tenant(&slug).await? else {
            continue;
        };
        tracing::info!(tenant = tenant.slug, "running tenant migrations");
        match migrate_tenant(control, &tenant).await {
            Ok(()) => summary.migrated += 1,
            Err(error) => {
                tracing::error!(tenant = tenant.slug, "tenant migrations failed: {error}");
                summary.failed += 1;
            }
        }
    }
    Ok(summary)
}

pub async fn create_tenant(
    control: &ControlPlane,
    config: &AppConfig,
    actor: &OperatorActor,
    registration: TenantRegistration,
    install: InstallCommand,
) -> Result<Tenant, AppError> {
    let tenant = register_tenant(control, actor, registration).await?;
    provision_tenant(control, config, &tenant, install).await
}

pub async fn register_tenant(
    control: &ControlPlane,
    actor: &OperatorActor,
    registration: TenantRegistration,
) -> Result<Tenant, AppError> {
    validate_slug(&registration.slug)?;
    let name = registration.name.trim().to_owned();
    if name.is_empty() || name.chars().count() > NAME_MAX {
        return Err(AppError::validation_code(
            "tenant_name_invalid",
            format!("the tenant name must have 1 to {NAME_MAX} characters"),
        ));
    }
    let mut hosts = Vec::new();
    for host in &registration.hosts {
        let host = validate_host(host)?;
        if !hosts.contains(&host) {
            hosts.push(host);
        }
    }
    if hosts.is_empty() {
        return Err(AppError::validation_code(
            "tenant_host_invalid",
            "a tenant needs at least one host",
        ));
    }
    let database_name = control.tenant_database_name(&registration.slug);
    if database_name.len() > DATABASE_NAME_MAX {
        return Err(AppError::validation_code(
            "tenant_slug_invalid",
            format!(
                "the database name {database_name:?} is longer than {DATABASE_NAME_MAX} characters"
            ),
        ));
    }
    let tenant = Tenant {
        id: Uuid::now_v7(),
        slug: registration.slug,
        name,
        database_name,
        status: TenantStatus::Provisioning,
        plugin_management: true,
    };
    insert_registration(control, &tenant, &hosts).await?;
    record_operator_action(
        control.pool(),
        actor,
        "tenant.created",
        Some(tenant.id),
        json!({ "slug": tenant.slug, "hosts": hosts, "database": tenant.database_name }),
    )
    .await;
    Ok(tenant)
}

async fn insert_registration(
    control: &ControlPlane,
    tenant: &Tenant,
    hosts: &[String],
) -> Result<(), AppError> {
    let mut transaction = control.pool().begin().await?;
    sqlx::query(
        "INSERT INTO tenants (id, slug, name, database_name, status) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(tenant.id.to_string())
    .bind(&tenant.slug)
    .bind(&tenant.name)
    .bind(&tenant.database_name)
    .bind(tenant.status.as_str())
    .execute(&mut *transaction)
    .await
    .map_err(|error| {
        unique_conflict(
            error,
            "tenant_slug_taken",
            "another tenant already uses this slug",
        )
    })?;
    for (index, host) in hosts.iter().enumerate() {
        sqlx::query("INSERT INTO tenant_hosts (host, tenant_id, is_primary) VALUES (?, ?, ?)")
            .bind(host)
            .bind(tenant.id.to_string())
            .bind(index == 0)
            .execute(&mut *transaction)
            .await
            .map_err(|error| {
                unique_conflict(
                    error,
                    "tenant_host_taken",
                    &format!("the host {host} is already in use"),
                )
            })?;
    }
    transaction.commit().await?;
    Ok(())
}

pub async fn provision_tenant(
    control: &ControlPlane,
    config: &AppConfig,
    tenant: &Tenant,
    install: InstallCommand,
) -> Result<Tenant, AppError> {
    match provision(control, config, tenant, install).await {
        Ok(()) => {
            set_status(control, tenant, TenantStatus::Active).await?;
            Ok(Tenant {
                status: TenantStatus::Active,
                ..tenant.clone()
            })
        }
        Err(error) => {
            record_error(control, tenant, &format!("provisioning failed: {error}")).await;
            Err(error)
        }
    }
}

async fn provision(
    control: &ControlPlane,
    config: &AppConfig,
    tenant: &Tenant,
    install: InstallCommand,
) -> Result<(), AppError> {
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "CREATE DATABASE `{}` CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci",
        checked_database_name(&tenant.database_name)?
    )))
    .execute(control.pool())
    .await?;
    migrate_tenant(control, tenant).await?;
    let runtime = TenantRuntime::open(control, config, tenant.clone()).await?;
    let result = scope(runtime.clone(), async {
        let state = runtime
            .state()
            .ok_or_else(|| AppError::Internal("the tenant state is not ready".to_owned()))?;
        crate::api::routes::v1::installer::install_organization(state, install).await
    })
    .await;
    runtime.stop();
    result.map(|_| ())
}

async fn record_error(control: &ControlPlane, tenant: &Tenant, message: &str) {
    if let Err(error) = sqlx::query("UPDATE tenants SET last_error = ? WHERE id = ?")
        .bind(message)
        .bind(tenant.id.to_string())
        .execute(control.pool())
        .await
    {
        tracing::error!(
            tenant = tenant.slug,
            "could not record the tenant error: {error}"
        );
    }
}

async fn set_status(
    control: &ControlPlane,
    tenant: &Tenant,
    status: TenantStatus,
) -> Result<(), AppError> {
    sqlx::query("UPDATE tenants SET status = ? WHERE id = ?")
        .bind(status.as_str())
        .bind(tenant.id.to_string())
        .execute(control.pool())
        .await?;
    Ok(())
}

pub async fn suspend_tenant(
    control: &ControlPlane,
    actor: &OperatorActor,
    slug: &str,
) -> Result<Tenant, AppError> {
    change_status(
        control,
        actor,
        slug,
        TenantStatus::Active,
        TenantStatus::Suspended,
        "tenant.suspended",
    )
    .await
}

pub async fn resume_tenant(
    control: &ControlPlane,
    actor: &OperatorActor,
    slug: &str,
) -> Result<Tenant, AppError> {
    change_status(
        control,
        actor,
        slug,
        TenantStatus::Suspended,
        TenantStatus::Active,
        "tenant.resumed",
    )
    .await
}

async fn change_status(
    control: &ControlPlane,
    actor: &OperatorActor,
    slug: &str,
    from: TenantStatus,
    to: TenantStatus,
    action: &str,
) -> Result<Tenant, AppError> {
    let tenant = control.require_tenant(slug).await?;
    if tenant.status != from {
        return Err(AppError::conflict_code(
            "tenant_status_conflict",
            format!(
                "the tenant is {}, not {}",
                tenant.status.as_str(),
                from.as_str()
            ),
        ));
    }
    set_status(control, &tenant, to).await?;
    record_operator_action(
        control.pool(),
        actor,
        action,
        Some(tenant.id),
        json!({ "slug": tenant.slug }),
    )
    .await;
    Ok(Tenant {
        status: to,
        ..tenant
    })
}

pub async fn set_plugin_management(
    control: &ControlPlane,
    actor: &OperatorActor,
    slug: &str,
    enabled: bool,
) -> Result<Tenant, AppError> {
    let tenant = control.require_tenant(slug).await?;
    sqlx::query("UPDATE tenants SET plugin_management_enabled = ? WHERE id = ?")
        .bind(enabled)
        .bind(tenant.id.to_string())
        .execute(control.pool())
        .await?;
    record_operator_action(
        control.pool(),
        actor,
        "tenant.features_updated",
        Some(tenant.id),
        json!({ "slug": tenant.slug, "plugin_management": enabled }),
    )
    .await;
    Ok(Tenant {
        plugin_management: enabled,
        ..tenant
    })
}

pub async fn add_host(
    control: &ControlPlane,
    actor: &OperatorActor,
    slug: &str,
    host: &str,
) -> Result<String, AppError> {
    let tenant = control.require_tenant(slug).await?;
    let host = validate_host(host)?;
    sqlx::query(
        "INSERT INTO tenant_hosts (host, tenant_id, is_primary) \
         SELECT ?, ?, NOT EXISTS (SELECT 1 FROM tenant_hosts WHERE tenant_id = ?)",
    )
    .bind(&host)
    .bind(tenant.id.to_string())
    .bind(tenant.id.to_string())
    .execute(control.pool())
    .await
    .map_err(|error| {
        unique_conflict(
            error,
            "tenant_host_taken",
            &format!("the host {host} is already in use"),
        )
    })?;
    record_operator_action(
        control.pool(),
        actor,
        "tenant.host_added",
        Some(tenant.id),
        json!({ "slug": tenant.slug, "host": host }),
    )
    .await;
    Ok(host)
}

pub async fn remove_host(
    control: &ControlPlane,
    actor: &OperatorActor,
    slug: &str,
    host: &str,
) -> Result<(), AppError> {
    let tenant = control.require_tenant(slug).await?;
    let host = validate_host(host)?;
    let mut transaction = control.pool().begin().await?;
    let hosts: Vec<(String, bool)> =
        sqlx::query_as("SELECT host, is_primary FROM tenant_hosts WHERE tenant_id = ? FOR UPDATE")
            .bind(tenant.id.to_string())
            .fetch_all(&mut *transaction)
            .await?;
    let Some((_, was_primary)) = hosts.iter().find(|(existing, _)| *existing == host) else {
        return Err(AppError::not_found_code(
            "tenant_host_not_found",
            format!("the tenant has no host {host}"),
        ));
    };
    if hosts.len() == 1 {
        return Err(AppError::conflict_code(
            "tenant_last_host",
            "a tenant needs at least one host",
        ));
    }
    sqlx::query("DELETE FROM tenant_hosts WHERE host = ?")
        .bind(&host)
        .execute(&mut *transaction)
        .await?;
    if *was_primary && let Some((next, _)) = hosts.iter().find(|(existing, _)| *existing != host) {
        sqlx::query("UPDATE tenant_hosts SET is_primary = TRUE WHERE host = ?")
            .bind(next)
            .execute(&mut *transaction)
            .await?;
    }
    transaction.commit().await?;
    record_operator_action(
        control.pool(),
        actor,
        "tenant.host_removed",
        Some(tenant.id),
        json!({ "slug": tenant.slug, "host": host }),
    )
    .await;
    Ok(())
}

pub async fn set_primary_host(
    control: &ControlPlane,
    actor: &OperatorActor,
    slug: &str,
    host: &str,
) -> Result<(), AppError> {
    let tenant = control.require_tenant(slug).await?;
    let host = validate_host(host)?;
    let mut transaction = control.pool().begin().await?;
    let updated =
        sqlx::query("UPDATE tenant_hosts SET is_primary = (host = ?) WHERE tenant_id = ?")
            .bind(&host)
            .bind(tenant.id.to_string())
            .execute(&mut *transaction)
            .await?;
    let exists: Option<i32> =
        sqlx::query_scalar("SELECT 1 FROM tenant_hosts WHERE host = ? AND tenant_id = ?")
            .bind(&host)
            .bind(tenant.id.to_string())
            .fetch_optional(&mut *transaction)
            .await?;
    if updated.rows_affected() == 0 || exists.is_none() {
        return Err(AppError::not_found_code(
            "tenant_host_not_found",
            format!("the tenant has no host {host}"),
        ));
    }
    transaction.commit().await?;
    record_operator_action(
        control.pool(),
        actor,
        "tenant.primary_host_changed",
        Some(tenant.id),
        json!({ "slug": tenant.slug, "host": host }),
    )
    .await;
    Ok(())
}

pub async fn begin_deletion(
    control: &ControlPlane,
    actor: &OperatorActor,
    slug: &str,
    export_to: Option<&Path>,
) -> Result<Tenant, AppError> {
    if export_to.is_some() && !export_available() {
        return Err(AppError::Configuration(
            "this build has no `org` command to export the tenant".to_owned(),
        ));
    }
    let tenant = control.require_tenant(slug).await?;
    set_status(control, &tenant, TenantStatus::Deleting).await?;
    record_operator_action(
        control.pool(),
        actor,
        "tenant.deletion_started",
        Some(tenant.id),
        json!({ "slug": tenant.slug, "export": export_to.map(|path| path.display().to_string()) }),
    )
    .await;
    Ok(Tenant {
        status: TenantStatus::Deleting,
        ..tenant
    })
}

pub async fn delete_tenant(
    control: &ControlPlane,
    config: &AppConfig,
    actor: &OperatorActor,
    tenant: &Tenant,
    export_to: Option<&Path>,
) -> Result<u64, AppError> {
    let result = remove(control, config, actor, tenant, export_to).await;
    if let Err(error) = &result {
        record_error(control, tenant, &format!("deletion failed: {error}")).await;
    }
    result
}

async fn remove(
    control: &ControlPlane,
    config: &AppConfig,
    actor: &OperatorActor,
    tenant: &Tenant,
    export_to: Option<&Path>,
) -> Result<u64, AppError> {
    if tenant.status != TenantStatus::Deleting {
        return Err(AppError::conflict_code(
            "tenant_status_conflict",
            "the tenant must be marked for deletion first",
        ));
    }
    let runtime = TenantRuntime::open(control, config, tenant.clone()).await?;
    let cleared = scope(runtime.clone(), async {
        if let Some(path) = export_to {
            export(control, config, tenant, path).await?;
        }
        let state = runtime
            .state()
            .ok_or_else(|| AppError::Internal("the tenant state is not ready".to_owned()))?;
        state.storage.delete_prefix("").await
    })
    .await;
    runtime.stop();
    let removed_files = cleared?;
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "DROP DATABASE IF EXISTS `{}`",
        checked_database_name(&tenant.database_name)?
    )))
    .execute(control.pool())
    .await?;
    sqlx::query("DELETE FROM tenants WHERE id = ?")
        .bind(tenant.id.to_string())
        .execute(control.pool())
        .await?;
    record_operator_action(
        control.pool(),
        actor,
        "tenant.deleted",
        Some(tenant.id),
        json!({
            "slug": tenant.slug,
            "database": tenant.database_name,
            "removed_files": removed_files,
            "exported_to": export_to.map(|path| path.display().to_string()),
        }),
    )
    .await;
    Ok(removed_files)
}

async fn export(
    control: &ControlPlane,
    config: &AppConfig,
    tenant: &Tenant,
    path: &Path,
) -> Result<(), AppError> {
    let command = export_command().ok_or_else(|| {
        AppError::Configuration("this build has no `org` command to export the tenant".to_owned())
    })?;
    if let Some(directory) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        std::fs::create_dir_all(directory).map_err(|error| {
            AppError::Configuration(format!(
                "the export directory {} cannot be created: {error}",
                directory.display()
            ))
        })?;
    }
    let mut tenant_config = config.clone();
    tenant_config.database_url = Some(control.tenant_database_url(tenant));
    tenant_config.control_database_url = None;
    tenant_config.console_host = None;
    let matches = command
        .command()
        .try_get_matches_from([
            OsString::from(EXPORT_COMMAND),
            "export".into(),
            "--output".into(),
            path.as_os_str().to_owned(),
        ])
        .map_err(|error| AppError::Internal(format!("invalid export command: {error}")))?;
    command
        .run(tenant_config, &matches)
        .await
        .map_err(|error| AppError::Internal(format!("the tenant export failed: {error}")))
}

fn checked_database_name(name: &str) -> Result<&str, AppError> {
    let valid = !name.is_empty()
        && name.len() <= DATABASE_NAME_MAX
        && name
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '_');
    if valid {
        Ok(name)
    } else {
        Err(AppError::Internal(format!(
            "refusing the database name {name:?}"
        )))
    }
}

fn unique_conflict(error: sqlx::Error, code: &'static str, message: &str) -> AppError {
    match &error {
        sqlx::Error::Database(database) if database.is_unique_violation() => {
            AppError::conflict_code(code, message.to_owned())
        }
        _ => error.into(),
    }
}
