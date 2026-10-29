use async_trait::async_trait;
use chrono::NaiveDateTime;
use serde_json::Value;
use sqlx::mysql::MySqlRow;
use sqlx::types::Json;
use sqlx::{MySqlPool, Row};
use uuid::Uuid;

use crate::application::config_packages::{ConfigInstallRepository, ConfigPackageInstall};
use crate::application::organization_admin::{ListWindow, Listing};
use crate::database::transaction::MySqlTx;
use crate::domain::UserId;
use crate::errors::AppError;
use crate::infrastructure::repositories::mapper;

const INSTALL_COLUMNS: &str = "i.id, i.name, i.format_version, i.template_key, i.summary, \
     i.installed_by, u.display_name AS installed_by_name, i.installed_at";

#[derive(Clone)]
pub struct MySqlConfigInstallRepository {
    pool: MySqlPool,
}

impl MySqlConfigInstallRepository {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

fn install_from_row(row: &MySqlRow, package: Value) -> Result<ConfigPackageInstall, AppError> {
    let id: String = row.try_get("id")?;
    let format_version: i32 = row.try_get("format_version")?;
    let Json(summary): Json<Value> = row.try_get("summary")?;
    let installed_at: NaiveDateTime = row.try_get("installed_at")?;
    Ok(ConfigPackageInstall {
        id: Uuid::parse_str(&id).map_err(|_| mapper::invalid_column("id", &id))?,
        name: row.try_get("name")?,
        format_version: u32::try_from(format_version)
            .map_err(|_| mapper::invalid_column("format_version", &format_version.to_string()))?,
        template_key: row.try_get("template_key")?,
        package,
        summary,
        installed_by: row
            .try_get::<Option<String>, _>("installed_by")?
            .map(|id| mapper::typed_id(id, UserId::new))
            .transpose()?,
        installed_by_name: row.try_get("installed_by_name")?,
        installed_at: mapper::utc(installed_at),
    })
}

#[async_trait]
impl ConfigInstallRepository for MySqlConfigInstallRepository {
    type Tx = MySqlTx;

    async fn organization_locale(&self) -> Result<Option<String>, AppError> {
        Ok(
            sqlx::query_scalar("SELECT locale FROM organization_settings WHERE id = 1")
                .fetch_optional(&self.pool)
                .await?,
        )
    }

    async fn insert(
        &self,
        tx: &mut MySqlTx,
        install: &ConfigPackageInstall,
    ) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO config_package_installs \
             (id, name, format_version, template_key, package, summary, installed_by, installed_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(install.id.to_string())
        .bind(&install.name)
        .bind(install.format_version)
        .bind(&install.template_key)
        .bind(Json(&install.package))
        .bind(Json(&install.summary))
        .bind(install.installed_by.as_ref().map(UserId::as_str))
        .bind(mapper::naive(install.installed_at))
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    async fn list(&self, window: ListWindow) -> Result<Listing<ConfigPackageInstall>, AppError> {
        let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM config_package_installs")
            .fetch_one(&self.pool)
            .await?;
        let rows = sqlx::query(sqlx::AssertSqlSafe(format!(
            "SELECT {INSTALL_COLUMNS} FROM config_package_installs i \
             LEFT JOIN users u ON u.id = i.installed_by \
             ORDER BY i.installed_at DESC, i.id DESC LIMIT ? OFFSET ?"
        )))
        .bind(window.limit)
        .bind(window.offset)
        .fetch_all(&self.pool)
        .await?;
        Ok(Listing {
            items: rows
                .iter()
                .map(|row| install_from_row(row, Value::Null))
                .collect::<Result<_, _>>()?,
            total: u64::try_from(total).unwrap_or_default(),
        })
    }

    async fn find(&self, install_id: Uuid) -> Result<Option<ConfigPackageInstall>, AppError> {
        let row = sqlx::query(sqlx::AssertSqlSafe(format!(
            "SELECT {INSTALL_COLUMNS}, i.package FROM config_package_installs i \
             LEFT JOIN users u ON u.id = i.installed_by \
             WHERE i.id = ?"
        )))
        .bind(install_id.to_string())
        .fetch_optional(&self.pool)
        .await?;
        row.map(|row| {
            let Json(package): Json<Value> = row.try_get("package")?;
            install_from_row(&row, package)
        })
        .transpose()
    }
}
