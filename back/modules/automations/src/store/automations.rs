use chrono::NaiveDateTime;
use serde_json::Value;
use sqlx::mysql::MySqlRow;
use sqlx::types::Json;
use sqlx::{MySqlPool, QueryBuilder, Row};
use uuid::Uuid;

use crate::{
    ActiveAutomation, Automation, AutomationDefinition, AutomationListFilter, AutomationVersion,
    TriggerSpec,
};
use aphel_contract_automation::domain::AutomationStatus;
use aphel_kernel::Ctx;
use aphel_kernel::domain::UserId;
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::application::organization_admin::{ListWindow, Listing};
use aphel_sdk::platform::database::transaction::MySqlTx;
use aphel_sdk::platform::infrastructure::fields::mysql_definitions::uuid_column;
use aphel_sdk::platform::infrastructure::repositories::mapper;

pub(crate) const AUTOMATION_COLUMNS: &str = "a.id, a.`key`, a.name, a.description, a.record_type, \
     a.`trigger`, a.definition, a.status, a.version, a.created_by, a.updated_by, a.created_at, a.updated_at";
pub(crate) const VERSION_COLUMNS: &str = "v.id AS version_id, v.automation_id, v.version AS version_number, v.name AS version_name, v.record_type AS version_record_type, \
     v.`trigger` AS version_trigger, v.definition AS version_definition, v.published_by, v.published_at";

#[derive(Clone)]
pub(crate) struct AutomationStore {
    pool: MySqlPool,
}

pub(crate) fn to_json<T: serde::Serialize>(value: &T) -> Result<Json<Value>, AppError> {
    serde_json::to_value(value)
        .map(Json)
        .map_err(|error| AppError::Internal(format!("cannot serialize automation: {error}")))
}

fn optional_user(row: &MySqlRow, column: &str) -> Result<Option<UserId>, AppError> {
    row.try_get::<Option<String>, _>(column)?
        .map(|id| mapper::typed_id(id, UserId::new))
        .transpose()
}

fn trigger_column(row: &MySqlRow, column: &str) -> Result<TriggerSpec, AppError> {
    let Json(value): Json<Value> = row.try_get(column)?;
    serde_json::from_value(value.clone())
        .map_err(|_| mapper::invalid_column(column, &value.to_string()))
}

fn definition_column(row: &MySqlRow, column: &str) -> Result<AutomationDefinition, AppError> {
    let Json(value): Json<Value> = row.try_get(column)?;
    serde_json::from_value(value.clone())
        .map_err(|_| mapper::invalid_column(column, &value.to_string()))
}

pub(crate) fn automation_from_row(row: &MySqlRow) -> Result<Automation, AppError> {
    let status: String = row.try_get("status")?;
    let version: i32 = row.try_get("version")?;
    Ok(Automation {
        id: uuid_column(row, "id")?,
        key: row.try_get("key")?,
        name: row.try_get("name")?,
        description: row.try_get("description")?,
        record_type: row.try_get("record_type")?,
        trigger: trigger_column(row, "trigger")?,
        definition: definition_column(row, "definition")?,
        status: AutomationStatus::parse(&status)
            .ok_or_else(|| mapper::invalid_column("status", &status))?,
        version: u32::try_from(version).unwrap_or_default(),
        created_by: optional_user(row, "created_by")?,
        updated_by: optional_user(row, "updated_by")?,
        created_at: mapper::utc(row.try_get::<NaiveDateTime, _>("created_at")?),
        updated_at: mapper::utc(row.try_get::<NaiveDateTime, _>("updated_at")?),
    })
}

pub(crate) fn version_from_row(row: &MySqlRow) -> Result<AutomationVersion, AppError> {
    let version: i32 = row.try_get("version_number")?;
    Ok(AutomationVersion {
        id: uuid_column(row, "version_id")?,
        automation_id: uuid_column(row, "automation_id")?,
        version: u32::try_from(version).unwrap_or_default(),
        name: row.try_get("version_name")?,
        record_type: row.try_get("version_record_type")?,
        trigger: trigger_column(row, "version_trigger")?,
        definition: definition_column(row, "version_definition")?,
        published_by: optional_user(row, "published_by")?,
        published_at: mapper::utc(row.try_get::<NaiveDateTime, _>("published_at")?),
    })
}

fn push_filter(builder: &mut QueryBuilder<sqlx::MySql>, filter: &AutomationListFilter) {
    if let Some(status) = filter.status {
        builder
            .push(" AND a.status = ")
            .push_bind(status.as_str().to_owned());
    }
    if let Some(record_type) = &filter.record_type {
        builder
            .push(" AND a.record_type = ")
            .push_bind(record_type.clone());
    }
    if let Some(search) = &filter.search {
        let pattern = format!(
            "%{}%",
            search
                .replace('\\', "\\\\")
                .replace('%', "\\%")
                .replace('_', "\\_")
        );
        builder
            .push(" AND (a.name LIKE ")
            .push_bind(pattern.clone())
            .push(" OR a.`key` LIKE ")
            .push_bind(pattern)
            .push(")");
    }
}

impl AutomationStore {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    pub(crate) fn of(ctx: &Ctx<'_>) -> Self {
        Self::new(ctx.pool().clone())
    }

    pub(crate) async fn insert(
        &self,
        tx: &mut MySqlTx,
        automation: &Automation,
    ) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO automations (id, `key`, name, description, record_type, `trigger`, \
             definition, status, version, created_by, updated_by, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(automation.id.to_string())
        .bind(&automation.key)
        .bind(&automation.name)
        .bind(&automation.description)
        .bind(&automation.record_type)
        .bind(to_json(&automation.trigger)?)
        .bind(to_json(&automation.definition)?)
        .bind(automation.status.as_str())
        .bind(automation.version)
        .bind(automation.created_by.as_ref().map(UserId::as_str))
        .bind(automation.updated_by.as_ref().map(UserId::as_str))
        .bind(mapper::naive(automation.created_at))
        .bind(mapper::naive(automation.updated_at))
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub(crate) async fn update(
        &self,
        tx: &mut MySqlTx,
        automation: &Automation,
    ) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE automations SET name = ?, description = ?, `trigger` = ?, definition = ?, status = ?, \
             version = ?, updated_by = ?, updated_at = ? WHERE id = ?",
        )
        .bind(&automation.name)
        .bind(&automation.description)
        .bind(to_json(&automation.trigger)?)
        .bind(to_json(&automation.definition)?)
        .bind(automation.status.as_str())
        .bind(automation.version)
        .bind(automation.updated_by.as_ref().map(UserId::as_str))
        .bind(mapper::naive(automation.updated_at))
        .bind(automation.id.to_string())
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub(crate) async fn delete(
        &self,
        tx: &mut MySqlTx,
        automation_id: Uuid,
    ) -> Result<(), AppError> {
        sqlx::query("DELETE FROM automations WHERE id = ?")
            .bind(automation_id.to_string())
            .execute(&mut **tx)
            .await?;
        Ok(())
    }

    pub(crate) async fn find(&self, automation_id: Uuid) -> Result<Option<Automation>, AppError> {
        let row = sqlx::query(sqlx::AssertSqlSafe(format!(
            "SELECT {AUTOMATION_COLUMNS} FROM automations a WHERE a.id = ?"
        )))
        .bind(automation_id.to_string())
        .fetch_optional(&self.pool)
        .await?;
        row.as_ref().map(automation_from_row).transpose()
    }

    pub(crate) async fn find_by_key(&self, key: &str) -> Result<Option<Automation>, AppError> {
        let row = sqlx::query(sqlx::AssertSqlSafe(format!(
            "SELECT {AUTOMATION_COLUMNS} FROM automations a WHERE a.`key` = ?"
        )))
        .bind(key)
        .fetch_optional(&self.pool)
        .await?;
        row.as_ref().map(automation_from_row).transpose()
    }

    pub(crate) async fn list(
        &self,
        filter: &AutomationListFilter,
        window: ListWindow,
    ) -> Result<Listing<Automation>, AppError> {
        let mut count = QueryBuilder::new("SELECT COUNT(*) FROM automations a WHERE 1 = 1");
        push_filter(&mut count, filter);
        let total: i64 = count.build_query_scalar().fetch_one(&self.pool).await?;
        let mut builder = QueryBuilder::new(format!(
            "SELECT {AUTOMATION_COLUMNS} FROM automations a WHERE 1 = 1"
        ));
        push_filter(&mut builder, filter);
        builder
            .push(" ORDER BY a.name, a.id LIMIT ")
            .push_bind(window.limit)
            .push(" OFFSET ")
            .push_bind(window.offset);
        let rows = builder.build().fetch_all(&self.pool).await?;
        Ok(Listing {
            items: rows
                .iter()
                .map(automation_from_row)
                .collect::<Result<_, _>>()?,
            total: u64::try_from(total).unwrap_or_default(),
        })
    }

    pub(crate) async fn list_all(&self) -> Result<Vec<Automation>, AppError> {
        let rows = sqlx::query(sqlx::AssertSqlSafe(format!(
            "SELECT {AUTOMATION_COLUMNS} FROM automations a ORDER BY a.`key`"
        )))
        .fetch_all(&self.pool)
        .await?;
        rows.iter().map(automation_from_row).collect()
    }

    pub(crate) async fn count(&self) -> Result<u64, AppError> {
        let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM automations")
            .fetch_one(&self.pool)
            .await?;
        Ok(u64::try_from(total).unwrap_or_default())
    }

    pub(crate) async fn insert_version(
        &self,
        tx: &mut MySqlTx,
        version: &AutomationVersion,
    ) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO automation_versions (id, automation_id, version, name, record_type, \
             trigger_type, `trigger`, definition, published_by, published_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(version.id.to_string())
        .bind(version.automation_id.to_string())
        .bind(version.version)
        .bind(&version.name)
        .bind(&version.record_type)
        .bind(&version.trigger.kind)
        .bind(to_json(&version.trigger)?)
        .bind(to_json(&version.definition)?)
        .bind(version.published_by.as_ref().map(UserId::as_str))
        .bind(mapper::naive(version.published_at))
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub(crate) async fn find_version(
        &self,
        automation_id: Uuid,
        version: u32,
    ) -> Result<Option<AutomationVersion>, AppError> {
        let row = sqlx::query(sqlx::AssertSqlSafe(format!(
            "SELECT {VERSION_COLUMNS} FROM automation_versions v \
             WHERE v.automation_id = ? AND v.version = ?"
        )))
        .bind(automation_id.to_string())
        .bind(version)
        .fetch_optional(&self.pool)
        .await?;
        row.as_ref().map(version_from_row).transpose()
    }

    pub(crate) async fn list_versions(
        &self,
        automation_id: Uuid,
    ) -> Result<Vec<AutomationVersion>, AppError> {
        let rows = sqlx::query(sqlx::AssertSqlSafe(format!(
            "SELECT {VERSION_COLUMNS} FROM automation_versions v \
             WHERE v.automation_id = ? ORDER BY v.version DESC"
        )))
        .bind(automation_id.to_string())
        .fetch_all(&self.pool)
        .await?;
        rows.iter().map(version_from_row).collect()
    }

    pub(crate) async fn active_trigger_pairs(&self) -> Result<Vec<(String, String)>, AppError> {
        let rows = sqlx::query(
            "SELECT DISTINCT v.record_type, v.trigger_type FROM automations a \
             JOIN automation_versions v ON v.automation_id = a.id AND v.version = a.version \
             WHERE a.status = 'active'",
        )
        .fetch_all(&self.pool)
        .await?;
        rows.iter()
            .map(|row| Ok((row.try_get("record_type")?, row.try_get("trigger_type")?)))
            .collect()
    }

    pub(crate) async fn active_for(
        &self,
        record_type: &str,
        trigger_type: &str,
    ) -> Result<Vec<ActiveAutomation>, AppError> {
        let rows = sqlx::query(sqlx::AssertSqlSafe(format!(
            "SELECT {AUTOMATION_COLUMNS}, {VERSION_COLUMNS} FROM automations a \
             JOIN automation_versions v ON v.automation_id = a.id AND v.version = a.version \
             WHERE a.status = 'active' \
             AND v.record_type = ? AND v.trigger_type = ? ORDER BY a.id"
        )))
        .bind(record_type)
        .bind(trigger_type)
        .fetch_all(&self.pool)
        .await?;
        rows.iter()
            .map(|row| {
                Ok(ActiveAutomation {
                    automation: automation_from_row(row)?,
                    version: version_from_row(row)?,
                })
            })
            .collect()
    }
}
