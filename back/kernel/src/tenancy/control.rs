use sqlx::migrate::Migrator;
use sqlx::mysql::MySqlConnectOptions;

use crate::config::MultiTenancyConfig;
use crate::database;
use crate::database::Db;
use crate::errors::AppError;
use crate::tenancy::{Tenant, TenantStatus};

static CONTROL_MIGRATOR: Migrator = sqlx::migrate!("./control_migrations");

const CONTROL_MAX_CONNECTIONS: u32 = 5;

#[derive(Clone)]
pub struct ControlPlane {
    pool: Db,
    url: String,
    server: MySqlConnectOptions,
    tenant_max_connections: u32,
}

impl ControlPlane {
    pub async fn connect(config: &MultiTenancyConfig) -> Result<Self, AppError> {
        let server: MySqlConnectOptions = config.control_database_url.parse().map_err(|error| {
            AppError::Configuration(format!("CONTROL_DATABASE_URL is invalid: {error}"))
        })?;
        if server.get_database().is_none_or(str::is_empty) {
            return Err(AppError::Configuration(
                "CONTROL_DATABASE_URL must name the control database".to_owned(),
            ));
        }
        let pool = database::pool::connect_with(server.clone(), CONTROL_MAX_CONNECTIONS).await?;
        Ok(Self {
            pool,
            url: config.control_database_url.clone(),
            server,
            tenant_max_connections: config.tenant_max_connections,
        })
    }

    pub fn pool(&self) -> &Db {
        &self.pool
    }

    pub async fn migrate(&self) -> Result<(), AppError> {
        CONTROL_MIGRATOR
            .run(&self.pool)
            .await
            .map_err(|error| AppError::Persistence(error.to_string()))
    }

    pub async fn connect_tenant(&self, tenant: &Tenant) -> Result<Db, AppError> {
        Ok(database::pool::connect_with(
            self.server.clone().database(&tenant.database_name),
            self.tenant_max_connections,
        )
        .await?)
    }

    pub fn control_database_name(&self) -> &str {
        self.server.get_database().unwrap_or_default()
    }

    pub fn tenant_database_name(&self, slug: &str) -> String {
        format!(
            "{}_{}",
            self.control_database_name(),
            slug.replace('-', "_")
        )
    }

    pub fn tenant_database_url(&self, tenant: &Tenant) -> String {
        let (scheme, rest) = self.url.split_once("://").unwrap_or(("mysql", &self.url));
        let authority_end = rest.find('/').unwrap_or(rest.len());
        let query = rest[authority_end..]
            .find('?')
            .map(|start| &rest[authority_end + start..])
            .unwrap_or_default();
        format!(
            "{scheme}://{}/{}{query}",
            &rest[..authority_end],
            tenant.database_name
        )
    }

    pub async fn find_tenant(&self, slug: &str) -> Result<Option<Tenant>, AppError> {
        let row: Option<(String, String, String, String, String, bool)> = sqlx::query_as(
            "SELECT id, slug, name, database_name, status, plugin_management_enabled \
             FROM tenants WHERE slug = ?",
        )
        .bind(slug)
        .fetch_optional(&self.pool)
        .await?;
        row.map(
            |(id, slug, name, database_name, status, plugin_management)| {
                Ok(Tenant {
                    id: uuid::Uuid::parse_str(&id)
                        .map_err(|_| AppError::Persistence(format!("invalid tenant id {id:?}")))?,
                    slug,
                    name,
                    database_name,
                    status: TenantStatus::parse(&status).ok_or_else(|| {
                        AppError::Persistence(format!("invalid tenant status {status:?}"))
                    })?,
                    plugin_management,
                })
            },
        )
        .transpose()
    }

    pub async fn require_tenant(&self, slug: &str) -> Result<Tenant, AppError> {
        self.find_tenant(slug).await?.ok_or_else(|| {
            AppError::not_found_code(
                "tenant_not_found",
                format!("no tenant has the slug {slug:?}"),
            )
        })
    }

    pub async fn close(&self) {
        self.pool.close().await;
    }
}
