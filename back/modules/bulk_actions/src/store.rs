use std::collections::HashSet;

use chrono::NaiveDateTime;
use serde_json::Value;
use sqlx::mysql::MySqlRow;
use sqlx::types::Json;
use sqlx::{MySqlPool, QueryBuilder, Row};
use uuid::Uuid;

use crate::{BulkAction, BulkItem, BulkOperation, BulkSelection, BulkStatus, ItemOutcome};
use aphel_kernel::Ctx;
use aphel_kernel::domain::UserId;
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::application::organization_admin::{ListWindow, Listing};
use aphel_sdk::platform::database::transaction::MySqlTx;
use aphel_sdk::platform::infrastructure::repositories::mapper;

const OPERATION_COLUMNS: &str = "id, record_type, action, params, selection, status, total, \
     processed, succeeded, skipped, failed, failure_reason, requested_by, created_at, updated_at, finished_at";

fn to_json<T: serde::Serialize>(value: &T) -> Result<Json<Value>, AppError> {
    serde_json::to_value(value)
        .map(Json)
        .map_err(|error| AppError::Internal(format!("failed to serialize bulk data: {error}")))
}

fn counter(row: &MySqlRow, column: &str) -> Result<u64, AppError> {
    let value: u32 = row.try_get(column)?;
    Ok(u64::from(value))
}

fn to_counter(value: u64) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}

fn operation_from_row(row: &MySqlRow) -> Result<BulkOperation, AppError> {
    let id: String = row.try_get("id")?;
    let Json(params): Json<Value> = row.try_get("params")?;
    let Json(selection): Json<Value> = row.try_get("selection")?;
    let status: String = row.try_get("status")?;
    let created_at: NaiveDateTime = row.try_get("created_at")?;
    let updated_at: NaiveDateTime = row.try_get("updated_at")?;
    let finished_at: Option<NaiveDateTime> = row.try_get("finished_at")?;
    Ok(BulkOperation {
        id: Uuid::parse_str(&id).map_err(|_| mapper::invalid_column("id", &id))?,
        record_type: row.try_get("record_type")?,
        action: serde_json::from_value::<BulkAction>(params.clone())
            .map_err(|_| mapper::invalid_column("params", &params.to_string()))?,
        selection: serde_json::from_value::<BulkSelection>(selection)
            .map_err(|_| mapper::invalid_column("selection", "<selection>"))?,
        status: BulkStatus::parse(&status)
            .ok_or_else(|| mapper::invalid_column("status", &status))?,
        total: counter(row, "total")?,
        processed: counter(row, "processed")?,
        succeeded: counter(row, "succeeded")?,
        skipped: counter(row, "skipped")?,
        failed: counter(row, "failed")?,
        failure_reason: row.try_get("failure_reason")?,
        requested_by: mapper::typed_id(row.try_get("requested_by")?, UserId::new)?,
        created_at: mapper::utc(created_at),
        updated_at: mapper::utc(updated_at),
        finished_at: finished_at.map(mapper::utc),
    })
}

fn item_from_row(row: &MySqlRow) -> Result<BulkItem, AppError> {
    let record_id: String = row.try_get("record_id")?;
    let outcome: String = row.try_get("outcome")?;
    Ok(BulkItem {
        record_id: Uuid::parse_str(&record_id)
            .map_err(|_| mapper::invalid_column("record_id", &record_id))?,
        outcome: ItemOutcome::parse(&outcome)
            .ok_or_else(|| mapper::invalid_column("outcome", &outcome))?,
        code: row.try_get("code")?,
        message: row.try_get("message")?,
    })
}

fn update_statement(operation: &BulkOperation) -> Result<QueryBuilder<sqlx::MySql>, AppError> {
    let mut builder = QueryBuilder::new("UPDATE bulk_operations SET selection = ");
    builder
        .push_bind(to_json(&operation.selection)?)
        .push(", status = ")
        .push_bind(operation.status.as_str())
        .push(", total = ")
        .push_bind(to_counter(operation.total))
        .push(", processed = ")
        .push_bind(to_counter(operation.processed))
        .push(", succeeded = ")
        .push_bind(to_counter(operation.succeeded))
        .push(", skipped = ")
        .push_bind(to_counter(operation.skipped))
        .push(", failed = ")
        .push_bind(to_counter(operation.failed))
        .push(", failure_reason = ")
        .push_bind(operation.failure_reason.clone())
        .push(", updated_at = ")
        .push_bind(mapper::naive(operation.updated_at))
        .push(", finished_at = ")
        .push_bind(operation.finished_at.map(mapper::naive))
        .push(" WHERE id = ")
        .push_bind(operation.id.to_string());
    Ok(builder)
}

#[derive(Clone)]
pub(crate) struct BulkOperationStore {
    pool: MySqlPool,
}

impl BulkOperationStore {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    pub(crate) fn of(ctx: &Ctx<'_>) -> Self {
        Self::new(ctx.pool().clone())
    }

    pub(crate) async fn insert(
        &self,
        tx: &mut MySqlTx,
        operation: &BulkOperation,
    ) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO bulk_operations (id, record_type, action, params, selection, status, \
             total, processed, succeeded, skipped, failed, failure_reason, requested_by, created_at, updated_at, finished_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(operation.id.to_string())
        .bind(&operation.record_type)
        .bind(operation.action.kind())
        .bind(to_json(&operation.action)?)
        .bind(to_json(&operation.selection)?)
        .bind(operation.status.as_str())
        .bind(to_counter(operation.total))
        .bind(to_counter(operation.processed))
        .bind(to_counter(operation.succeeded))
        .bind(to_counter(operation.skipped))
        .bind(to_counter(operation.failed))
        .bind(operation.failure_reason.as_deref())
        .bind(operation.requested_by.as_str())
        .bind(mapper::naive(operation.created_at))
        .bind(mapper::naive(operation.updated_at))
        .bind(operation.finished_at.map(mapper::naive))
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub(crate) async fn save(&self, operation: &BulkOperation) -> Result<(), AppError> {
        update_statement(operation)?
            .build()
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub(crate) async fn save_in(
        &self,
        tx: &mut MySqlTx,
        operation: &BulkOperation,
    ) -> Result<(), AppError> {
        update_statement(operation)?
            .build()
            .execute(&mut **tx)
            .await?;
        Ok(())
    }

    pub(crate) async fn find(&self, operation_id: Uuid) -> Result<Option<BulkOperation>, AppError> {
        let mut builder = QueryBuilder::new(format!(
            "SELECT {OPERATION_COLUMNS} FROM bulk_operations WHERE "
        ));
        builder.push("id = ").push_bind(operation_id.to_string());
        builder
            .build()
            .fetch_optional(&self.pool)
            .await?
            .as_ref()
            .map(operation_from_row)
            .transpose()
    }

    pub(crate) async fn list(
        &self,
        requested_by: &UserId,
        window: ListWindow,
    ) -> Result<Listing<BulkOperation>, AppError> {
        let total: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM bulk_operations WHERE requested_by = ?")
                .bind(requested_by.as_str())
                .fetch_one(&self.pool)
                .await?;
        let mut builder = QueryBuilder::new(format!(
            "SELECT {OPERATION_COLUMNS} FROM bulk_operations WHERE "
        ));
        builder
            .push("requested_by = ")
            .push_bind(requested_by.as_str().to_owned())
            .push(" ORDER BY created_at DESC, id DESC LIMIT ")
            .push_bind(window.limit)
            .push(" OFFSET ")
            .push_bind(window.offset);
        let rows = builder.build().fetch_all(&self.pool).await?;
        Ok(Listing {
            items: rows
                .iter()
                .map(operation_from_row)
                .collect::<Result<_, _>>()?,
            total: u64::try_from(total).unwrap_or_default(),
        })
    }

    pub(crate) async fn record_item(
        &self,
        tx: &mut MySqlTx,
        operation: &BulkOperation,
        item: &BulkItem,
    ) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO bulk_operation_items (operation_id, record_id, outcome, code, message) \
             VALUES (?, ?, ?, ?, ?) \
             ON DUPLICATE KEY UPDATE outcome = VALUES(outcome), code = VALUES(code), message = VALUES(message)",
        )
        .bind(operation.id.to_string())
        .bind(item.record_id.to_string())
        .bind(item.outcome.as_str())
        .bind(item.code.as_deref())
        .bind(item.message.as_deref())
        .execute(&mut **tx)
        .await?;
        self.save_in(tx, operation).await
    }

    pub(crate) async fn processed_ids(
        &self,
        operation_id: Uuid,
    ) -> Result<HashSet<Uuid>, AppError> {
        let ids: Vec<String> =
            sqlx::query_scalar("SELECT record_id FROM bulk_operation_items WHERE operation_id = ?")
                .bind(operation_id.to_string())
                .fetch_all(&self.pool)
                .await?;
        Ok(ids
            .iter()
            .filter_map(|id| Uuid::parse_str(id).ok())
            .collect())
    }

    pub(crate) async fn items(
        &self,
        operation_id: Uuid,
        outcome: Option<ItemOutcome>,
        window: ListWindow,
    ) -> Result<Listing<BulkItem>, AppError> {
        let push_filter = |builder: &mut QueryBuilder<sqlx::MySql>| {
            builder
                .push(" WHERE operation_id = ")
                .push_bind(operation_id.to_string());
            match outcome {
                Some(outcome) => builder.push(" AND outcome = ").push_bind(outcome.as_str()),
                None => builder
                    .push(" AND outcome <> ")
                    .push_bind(ItemOutcome::Succeeded.as_str()),
            };
        };
        let mut count = QueryBuilder::new("SELECT COUNT(*) FROM bulk_operation_items");
        push_filter(&mut count);
        let total: i64 = count.build_query_scalar().fetch_one(&self.pool).await?;
        let mut builder =
            QueryBuilder::new("SELECT record_id, outcome, code, message FROM bulk_operation_items");
        push_filter(&mut builder);
        builder
            .push(" ORDER BY created_at, record_id LIMIT ")
            .push_bind(window.limit)
            .push(" OFFSET ")
            .push_bind(window.offset);
        let rows = builder.build().fetch_all(&self.pool).await?;
        Ok(Listing {
            items: rows.iter().map(item_from_row).collect::<Result<_, _>>()?,
            total: u64::try_from(total).unwrap_or_default(),
        })
    }
}
