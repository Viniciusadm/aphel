use std::sync::Arc;

use async_trait::async_trait;
use chrono::NaiveDateTime;
use serde_json::Value;
use sqlx::mysql::MySqlRow;
use sqlx::types::Json;
use sqlx::{AssertSqlSafe, MySqlPool, Row};
use uuid::Uuid;

use crate::application::versioning::{
    ConfigVersionPorts, ConfigVersionRepository, InstalledVersion, ItemResolution, VersionItem,
};
use crate::infrastructure::MySqlConfigPackagePorts;
use aphel_kernel::domain::UserId;
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::application::fields::RecordTypeRegistry;
use aphel_sdk::platform::database::transaction::MySqlTx;
use aphel_sdk::platform::infrastructure::platform::{SharedAuditLog, SharedEventPublisher};
use aphel_sdk::platform::infrastructure::repositories::mapper;

const MAX_LISTED_VERSIONS: i64 = 500;
const VERSION_COLUMNS: &str = "v.id, v.package_id, v.version, v.name, v.install_id, v.origin, \
     v.package_sha256, v.installed_by, u.display_name AS installed_by_name, v.installed_at";

#[derive(Clone)]
pub struct MySqlConfigVersionRepository {
    pool: MySqlPool,
}

impl MySqlConfigVersionRepository {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

fn uuid_column(row: &MySqlRow, column: &str) -> Result<Uuid, AppError> {
    let value: String = row.try_get(column)?;
    Uuid::parse_str(&value).map_err(|_| mapper::invalid_column(column, &value))
}

fn version_from_row(row: &MySqlRow, applied_package: Value) -> Result<InstalledVersion, AppError> {
    let installed_at: NaiveDateTime = row.try_get("installed_at")?;
    Ok(InstalledVersion {
        id: uuid_column(row, "id")?,
        package_id: row.try_get("package_id")?,
        version: row.try_get("version")?,
        name: row.try_get("name")?,
        install_id: uuid_column(row, "install_id")?,
        origin: row.try_get("origin")?,
        package_sha256: row.try_get("package_sha256")?,
        applied_package,
        installed_by: row
            .try_get::<Option<String>, _>("installed_by")?
            .map(|id| mapper::typed_id(id, UserId::new))
            .transpose()?,
        installed_by_name: row.try_get("installed_by_name")?,
        installed_at: mapper::utc(installed_at),
    })
}

#[async_trait]
impl ConfigVersionRepository for MySqlConfigVersionRepository {
    type Tx = MySqlTx;

    async fn insert(
        &self,
        tx: &mut MySqlTx,
        version: &InstalledVersion,
        items: &[VersionItem],
    ) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO config_package_versions \
             (id, package_id, version, name, install_id, origin, package_sha256, \
             applied_package, installed_by, installed_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(version.id.to_string())
        .bind(&version.package_id)
        .bind(&version.version)
        .bind(&version.name)
        .bind(version.install_id.to_string())
        .bind(&version.origin)
        .bind(&version.package_sha256)
        .bind(Json(&version.applied_package))
        .bind(version.installed_by.as_ref().map(UserId::as_str))
        .bind(mapper::naive(version.installed_at))
        .execute(&mut **tx)
        .await?;
        for item in items {
            sqlx::query(
                "INSERT INTO config_package_version_items \
                 (version_id, item_id, section, kind, item_key, package_hash, resolution) \
                 VALUES (?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(version.id.to_string())
            .bind(&item.item_id)
            .bind(&item.section)
            .bind(&item.kind)
            .bind(&item.key)
            .bind(&item.package_hash)
            .bind(item.resolution.as_str())
            .execute(&mut **tx)
            .await?;
        }
        Ok(())
    }

    async fn latest(&self, package_id: &str) -> Result<Option<InstalledVersion>, AppError> {
        let row = sqlx::query(AssertSqlSafe(format!(
            "SELECT {VERSION_COLUMNS}, v.applied_package FROM config_package_versions v \
             LEFT JOIN users u ON u.id = v.installed_by \
             WHERE v.package_id = ? \
             ORDER BY v.installed_at DESC, v.id DESC LIMIT 1"
        )))
        .bind(package_id)
        .fetch_optional(&self.pool)
        .await?;
        row.map(|row| {
            let Json(applied): Json<Value> = row.try_get("applied_package")?;
            version_from_row(&row, applied)
        })
        .transpose()
    }

    async fn items(&self, version_id: Uuid) -> Result<Vec<VersionItem>, AppError> {
        let rows = sqlx::query(
            "SELECT item_id, section, kind, item_key, package_hash, resolution \
             FROM config_package_version_items WHERE version_id = ?",
        )
        .bind(version_id.to_string())
        .fetch_all(&self.pool)
        .await?;
        rows.iter()
            .map(|row| {
                let resolution: String = row.try_get("resolution")?;
                Ok(VersionItem {
                    item_id: row.try_get("item_id")?,
                    section: row.try_get("section")?,
                    kind: row.try_get("kind")?,
                    key: row.try_get("item_key")?,
                    package_hash: row.try_get("package_hash")?,
                    resolution: ItemResolution::parse(&resolution)
                        .ok_or_else(|| mapper::invalid_column("resolution", &resolution))?,
                })
            })
            .collect()
    }

    async fn list(&self) -> Result<Vec<InstalledVersion>, AppError> {
        let rows = sqlx::query(AssertSqlSafe(format!(
            "SELECT {VERSION_COLUMNS} FROM config_package_versions v \
             LEFT JOIN users u ON u.id = v.installed_by \
             ORDER BY v.installed_at DESC, v.id DESC LIMIT ?"
        )))
        .bind(MAX_LISTED_VERSIONS)
        .fetch_all(&self.pool)
        .await?;
        rows.iter()
            .map(|row| version_from_row(row, Value::Null))
            .collect()
    }
}

pub struct MySqlConfigVersionPorts {
    packages: MySqlConfigPackagePorts,
    versions: MySqlConfigVersionRepository,
}

impl MySqlConfigVersionPorts {
    pub fn new(
        pool: MySqlPool,
        registry: Arc<RecordTypeRegistry>,
        events: SharedEventPublisher,
        audit: SharedAuditLog,
    ) -> Self {
        Self {
            packages: MySqlConfigPackagePorts::new(pool.clone(), registry, events, audit),
            versions: MySqlConfigVersionRepository::new(pool),
        }
    }
}

impl ConfigVersionPorts for MySqlConfigVersionPorts {
    type Packages = MySqlConfigPackagePorts;
    type Versions = MySqlConfigVersionRepository;

    fn packages(&self) -> &Self::Packages {
        &self.packages
    }

    fn versions(&self) -> &Self::Versions {
        &self.versions
    }
}
