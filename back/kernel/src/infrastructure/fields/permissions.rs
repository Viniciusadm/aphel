use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, NaiveDateTime, Utc};
use sqlx::mysql::MySqlRow;
use sqlx::{MySqlPool, QueryBuilder, Row};

use crate::application::fields::RecordTypeRegistry;
use crate::application::fields::permissions::{
    FieldAccess, FieldAccessRule, FieldPermissionPorts, FieldPermissionRepository,
    FieldPermissionState, PermissionRole, StoredFieldPermission,
};
use crate::application::shared::{AuditLog, EventPublisher, RecordType};
use crate::database::transaction::MySqlTx;
use crate::domain::UserId;
use crate::domain::access::AccessScope;
use crate::errors::AppError;
use crate::infrastructure::auth::SystemClock;
use crate::infrastructure::fields::MySqlFieldDefinitionRepository;
use crate::infrastructure::fields::mysql_definitions::uuid_column;
use crate::infrastructure::repositories::mapper;
use crate::infrastructure::transactions::MySqlTransactionManager;

macro_rules! restrictable_field {
    () => {
        "(fd.is_required = FALSE OR (fd.is_system = FALSE \
         AND JSON_EXTRACT(fd.config, '$.default_value') IS NOT NULL \
         AND JSON_TYPE(JSON_EXTRACT(fd.config, '$.default_value')) <> 'NULL'))"
    };
}

fn access_column(row: &MySqlRow) -> Result<FieldAccess, AppError> {
    let value: String = row.try_get("access")?;
    FieldAccess::parse(&value).ok_or_else(|| mapper::invalid_column("access", &value))
}

fn rule_from_row(row: &MySqlRow) -> Result<FieldAccessRule, AppError> {
    Ok(FieldAccessRule {
        field_id: uuid_column(row, "field_id")?,
        key: row.try_get("key")?,
        is_system: row.try_get("is_system")?,
        access: access_column(row)?,
    })
}

pub async fn role_access_rules(
    pool: &MySqlPool,
    record_type: &RecordType,
    role_key: &str,
) -> Result<Vec<FieldAccessRule>, AppError> {
    let rows = sqlx::query(concat!(
        "SELECT fp.field_id, fd.`key`, fd.is_system, fp.access FROM field_permissions fp \
         JOIN roles r ON r.id = fp.role_id \
         JOIN field_definitions fd ON fd.id = fp.field_id \
         WHERE fp.record_type = ? AND r.`key` = ? AND ",
        restrictable_field!()
    ))
    .bind(record_type.as_str())
    .bind(role_key)
    .fetch_all(pool)
    .await?;
    rows.iter().map(rule_from_row).collect()
}

pub(crate) async fn strictest_access_rules(
    pool: &MySqlPool,
    record_type: &RecordType,
) -> Result<Vec<FieldAccessRule>, AppError> {
    let rows = sqlx::query(concat!(
        "SELECT fp.field_id, fd.`key`, fd.is_system, MIN(fp.access) AS access FROM field_permissions fp \
         JOIN field_definitions fd ON fd.id = fp.field_id \
         WHERE fp.record_type = ? AND ",
        restrictable_field!(),
        " GROUP BY fp.field_id, fd.`key`, fd.is_system"
    ))
    .bind(record_type.as_str())
    .fetch_all(pool)
    .await?;
    rows.iter().map(rule_from_row).collect()
}

pub(crate) async fn member_role_key(
    pool: &MySqlPool,
    user_id: &UserId,
) -> Result<Option<String>, AppError> {
    let row = sqlx::query(
        "SELECT r.`key` FROM users u \
         JOIN roles r ON r.id = u.role_id \
         WHERE u.id = ? AND u.status = 'active'",
    )
    .bind(user_id.as_str())
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|row| row.try_get("key")).transpose()?)
}

pub(crate) async fn scope_access_rules(
    pool: &MySqlPool,
    scope: &AccessScope,
    record_type: &RecordType,
) -> Result<Vec<FieldAccessRule>, AppError> {
    role_access_rules(pool, record_type, scope.role_key().as_str()).await
}

#[derive(Clone)]
pub struct MySqlFieldPermissionRepository {
    pool: MySqlPool,
}

impl MySqlFieldPermissionRepository {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl FieldPermissionRepository for MySqlFieldPermissionRepository {
    type Tx = MySqlTx;

    async fn roles(&self) -> Result<Vec<PermissionRole>, AppError> {
        let rows = sqlx::query(
            "SELECT id, `key`, name, is_system FROM roles \
             ORDER BY is_system DESC, created_at, name",
        )
        .fetch_all(&self.pool)
        .await?;
        rows.iter()
            .map(|row| {
                Ok(PermissionRole {
                    id: uuid_column(row, "id")?,
                    key: row.try_get("key")?,
                    name: row.try_get("name")?,
                    is_system: row.try_get("is_system")?,
                })
            })
            .collect()
    }

    async fn state(&self, record_type: &RecordType) -> Result<FieldPermissionState, AppError> {
        let rows = sqlx::query(
            "SELECT field_id, role_id, access FROM field_permissions \
             WHERE record_type = ?",
        )
        .bind(record_type.as_str())
        .fetch_all(&self.pool)
        .await?;
        let entries = rows
            .iter()
            .map(|row| {
                Ok(StoredFieldPermission {
                    field_id: uuid_column(row, "field_id")?,
                    role_id: uuid_column(row, "role_id")?,
                    access: access_column(row)?,
                })
            })
            .collect::<Result<Vec<_>, AppError>>()?;
        let version = sqlx::query(
            "SELECT updated_at, updated_by FROM field_permission_versions \
             WHERE record_type = ?",
        )
        .bind(record_type.as_str())
        .fetch_optional(&self.pool)
        .await?;
        let (updated_at, updated_by) = match version {
            Some(row) => {
                let updated_at: NaiveDateTime = row.try_get("updated_at")?;
                let updated_by: Option<String> = row.try_get("updated_by")?;
                (
                    Some(mapper::utc(updated_at)),
                    updated_by.and_then(|id| UserId::new(id).ok()),
                )
            }
            None => (None, None),
        };
        Ok(FieldPermissionState {
            entries,
            updated_at,
            updated_by,
        })
    }

    async fn replace(
        &self,
        tx: &mut MySqlTx,
        scope: &AccessScope,
        record_type: &RecordType,
        entries: &[StoredFieldPermission],
        now: DateTime<Utc>,
    ) -> Result<(), AppError> {
        sqlx::query("DELETE FROM field_permissions WHERE record_type = ?")
            .bind(record_type.as_str())
            .execute(&mut **tx)
            .await?;
        if !entries.is_empty() {
            let mut builder = QueryBuilder::new(
                "INSERT INTO field_permissions (field_id, role_id, record_type, access, created_at, updated_at) ",
            );
            builder.push_values(entries, |mut values, entry| {
                values
                    .push_bind(entry.field_id.to_string())
                    .push_bind(entry.role_id.to_string())
                    .push_bind(record_type.as_str().to_owned())
                    .push_bind(entry.access.as_str())
                    .push_bind(now.naive_utc())
                    .push_bind(now.naive_utc());
            });
            builder.build().execute(&mut **tx).await?;
        }
        sqlx::query(
            "INSERT INTO field_permission_versions (record_type, updated_at, updated_by) \
             VALUES (?, ?, ?) ON DUPLICATE KEY UPDATE updated_at = VALUES(updated_at), updated_by = VALUES(updated_by)",
        )
        .bind(record_type.as_str())
        .bind(now.naive_utc())
        .bind(scope.user_id().as_str())
        .execute(&mut **tx)
        .await?;
        Ok(())
    }
}

pub struct MySqlFieldPermissionPorts {
    registry: Arc<RecordTypeRegistry>,
    transactions: MySqlTransactionManager,
    definitions: MySqlFieldDefinitionRepository,
    permissions: MySqlFieldPermissionRepository,
    events: Arc<dyn EventPublisher<MySqlTx>>,
    audit: Arc<dyn AuditLog<MySqlTx>>,
}

impl MySqlFieldPermissionPorts {
    pub(crate) fn new(
        pool: MySqlPool,
        registry: Arc<RecordTypeRegistry>,
        events: Arc<dyn EventPublisher<MySqlTx>>,
        audit: Arc<dyn AuditLog<MySqlTx>>,
    ) -> Self {
        Self {
            registry,
            transactions: MySqlTransactionManager::new(pool.clone()),
            definitions: MySqlFieldDefinitionRepository::new(pool.clone()),
            permissions: MySqlFieldPermissionRepository::new(pool),
            events,
            audit,
        }
    }
}

impl FieldPermissionPorts for MySqlFieldPermissionPorts {
    type Transactions = MySqlTransactionManager;
    type Definitions = MySqlFieldDefinitionRepository;
    type Permissions = MySqlFieldPermissionRepository;
    type Events = Arc<dyn EventPublisher<MySqlTx>>;
    type Audit = Arc<dyn AuditLog<MySqlTx>>;
    type Clock = SystemClock;

    fn registry(&self) -> &RecordTypeRegistry {
        &self.registry
    }

    fn transactions(&self) -> &Self::Transactions {
        &self.transactions
    }

    fn definitions(&self) -> &Self::Definitions {
        &self.definitions
    }

    fn permissions(&self) -> &Self::Permissions {
        &self.permissions
    }

    fn events(&self) -> &Self::Events {
        &self.events
    }

    fn audit(&self) -> &Self::Audit {
        &self.audit
    }

    fn clock(&self) -> &Self::Clock {
        &SystemClock
    }
}
