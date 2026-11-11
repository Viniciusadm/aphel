use std::sync::Arc;

use serde_json::json;
use uuid::Uuid;

use crate::application::fields::RecordTypeRegistry;
use crate::application::installer::InstallCommand;
use crate::config::AppConfig;
use crate::database;
use crate::errors::AppError;
use crate::infrastructure::custom_objects::sync_custom_object_fields;
use crate::infrastructure::fields::sync_system_fields;
use crate::tenancy::audit::{OperatorAction, record_operator_action};
use crate::tenancy::{ControlPlane, Tenant, TenantRuntime, TenantStatus, normalize_host, scope};

const SLUG_MIN: usize = 3;
const SLUG_MAX: usize = 40;
const DATABASE_NAME_MAX: usize = 64;

pub struct NewTenant {
    pub slug: String,
    pub name: String,
    pub hosts: Vec<String>,
    pub install: InstallCommand,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct MigrationSummary {
    pub migrated: usize,
    pub failed: usize,
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
    request: NewTenant,
) -> Result<Tenant, AppError> {
    validate_slug(&request.slug)?;
    let name = request.name.trim().to_owned();
    if name.is_empty() || name.chars().count() > 140 {
        return Err(AppError::validation_code(
            "tenant_name_invalid",
            "the tenant name must have 1 to 140 characters",
        ));
    }
    let hosts = request
        .hosts
        .iter()
        .map(|host| {
            normalize_host(host).ok_or_else(|| {
                AppError::validation_code(
                    "tenant_host_invalid",
                    format!("{host:?} is not a valid host"),
                )
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    if hosts.is_empty() {
        return Err(AppError::validation_code(
            "tenant_host_invalid",
            "a tenant needs at least one host",
        ));
    }
    let database_name = control.tenant_database_name(&request.slug);
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
        slug: request.slug,
        name,
        database_name,
        status: TenantStatus::Provisioning,
    };
    register(control, &tenant, &hosts).await?;
    record_operator_action(
        control.pool(),
        OperatorAction {
            operator_id: None,
            action: "tenant.created",
            tenant_id: Some(tenant.id),
            details: json!({ "slug": tenant.slug, "hosts": hosts, "database": tenant.database_name }),
            ip: None,
        },
    )
    .await;
    match provision(control, config, &tenant, request.install).await {
        Ok(()) => {
            set_status(control, &tenant, TenantStatus::Active).await?;
            Ok(Tenant {
                status: TenantStatus::Active,
                ..tenant
            })
        }
        Err(error) => {
            if let Err(recording) = sqlx::query("UPDATE tenants SET last_error = ? WHERE id = ?")
                .bind(format!("provisioning failed: {error}"))
                .bind(tenant.id.to_string())
                .execute(control.pool())
                .await
            {
                tracing::error!(
                    tenant = tenant.slug,
                    "could not record the provisioning error: {recording}"
                );
            }
            Err(error)
        }
    }
}

async fn register(
    control: &ControlPlane,
    tenant: &Tenant,
    hosts: &[String],
) -> Result<(), AppError> {
    let mut transaction = control.pool().begin().await?;
    let inserted = sqlx::query(
        "INSERT INTO tenants (id, slug, name, database_name, status) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(tenant.id.to_string())
    .bind(&tenant.slug)
    .bind(&tenant.name)
    .bind(&tenant.database_name)
    .bind(tenant.status.as_str())
    .execute(&mut *transaction)
    .await;
    if let Err(error) = inserted {
        return Err(if is_unique_violation(&error) {
            AppError::conflict_code("tenant_slug_taken", "another tenant already uses this slug")
        } else {
            error.into()
        });
    }
    for (index, host) in hosts.iter().enumerate() {
        let inserted =
            sqlx::query("INSERT INTO tenant_hosts (host, tenant_id, is_primary) VALUES (?, ?, ?)")
                .bind(host)
                .bind(tenant.id.to_string())
                .bind(index == 0)
                .execute(&mut *transaction)
                .await;
        if let Err(error) = inserted {
            return Err(if is_unique_violation(&error) {
                AppError::conflict_code(
                    "tenant_host_taken",
                    format!("the host {host} is already in use"),
                )
            } else {
                error.into()
            });
        }
    }
    transaction.commit().await?;
    Ok(())
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

pub async fn set_status(
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

pub async fn suspend_tenant(control: &ControlPlane, slug: &str) -> Result<Tenant, AppError> {
    change_status(
        control,
        slug,
        TenantStatus::Active,
        TenantStatus::Suspended,
        "tenant.suspended",
    )
    .await
}

pub async fn resume_tenant(control: &ControlPlane, slug: &str) -> Result<Tenant, AppError> {
    change_status(
        control,
        slug,
        TenantStatus::Suspended,
        TenantStatus::Active,
        "tenant.resumed",
    )
    .await
}

async fn change_status(
    control: &ControlPlane,
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
        OperatorAction {
            operator_id: None,
            action,
            tenant_id: Some(tenant.id),
            details: json!({ "slug": tenant.slug }),
            ip: None,
        },
    )
    .await;
    Ok(Tenant {
        status: to,
        ..tenant
    })
}

pub async fn begin_deletion(control: &ControlPlane, slug: &str) -> Result<Tenant, AppError> {
    let tenant = control.require_tenant(slug).await?;
    set_status(control, &tenant, TenantStatus::Deleting).await?;
    Ok(Tenant {
        status: TenantStatus::Deleting,
        ..tenant
    })
}

pub async fn finish_deletion(
    control: &ControlPlane,
    config: &AppConfig,
    tenant: &Tenant,
    exported_to: Option<&str>,
) -> Result<u64, AppError> {
    if tenant.status != TenantStatus::Deleting {
        return Err(AppError::conflict_code(
            "tenant_status_conflict",
            "the tenant must be marked for deletion first",
        ));
    }
    let runtime = TenantRuntime::open(control, config, tenant.clone()).await?;
    let removed_files = scope(runtime.clone(), async {
        let state = runtime
            .state()
            .ok_or_else(|| AppError::Internal("the tenant state is not ready".to_owned()))?;
        state.storage.delete_prefix("").await
    })
    .await;
    runtime.stop();
    let removed_files = removed_files?;
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
        OperatorAction {
            operator_id: None,
            action: "tenant.deleted",
            tenant_id: Some(tenant.id),
            details: json!({
                "slug": tenant.slug,
                "database": tenant.database_name,
                "removed_files": removed_files,
                "exported_to": exported_to,
            }),
            ip: None,
        },
    )
    .await;
    Ok(removed_files)
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

fn is_unique_violation(error: &sqlx::Error) -> bool {
    matches!(error, sqlx::Error::Database(database) if database.is_unique_violation())
}
