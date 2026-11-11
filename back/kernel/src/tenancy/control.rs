use sqlx::MySqlPool;
use sqlx::migrate::Migrator;
use sqlx::mysql::MySqlConnectOptions;

use crate::config::MultiTenancyConfig;
use crate::database;
use crate::errors::AppError;
use crate::tenancy::Tenant;

static CONTROL_MIGRATOR: Migrator = sqlx::migrate!("./control_migrations");

const CONTROL_MAX_CONNECTIONS: u32 = 5;

#[derive(Clone)]
pub struct ControlPlane {
    pool: MySqlPool,
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
            server,
            tenant_max_connections: config.tenant_max_connections,
        })
    }

    pub fn pool(&self) -> &MySqlPool {
        &self.pool
    }

    pub async fn migrate(&self) -> Result<(), AppError> {
        CONTROL_MIGRATOR
            .run(&self.pool)
            .await
            .map_err(|error| AppError::Persistence(error.to_string()))
    }

    pub async fn connect_tenant(&self, tenant: &Tenant) -> Result<MySqlPool, AppError> {
        Ok(database::pool::connect_with(
            self.server.clone().database(&tenant.database_name),
            self.tenant_max_connections,
        )
        .await?)
    }

    pub async fn close(&self) {
        self.pool.close().await;
    }
}
