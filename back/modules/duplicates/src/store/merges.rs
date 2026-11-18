use std::collections::{BTreeMap, HashMap};

use chrono::{DateTime, NaiveDateTime, Utc};
use serde_json::{Map, Value};
use sqlx::types::Json;
use sqlx::{MySql, MySqlPool, QueryBuilder, Row};
use uuid::Uuid;

use crate::DuplicateType;
use crate::domain::DuplicateFamily;
use aphel_kernel::Ctx;
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::application::fields::SystemField;
use aphel_sdk::platform::database::transaction::Tx;
use aphel_sdk::platform::domain::access::AccessScope;
use aphel_sdk::platform::infrastructure::fields::mysql_definitions::uuid_column;
use aphel_sdk::platform::infrastructure::query::compiler::{aliased, push_partition};
use aphel_sdk::platform::infrastructure::repositories::mapper;
use aphel_sdk::platform::infrastructure::repositories::mapper::push_ids;

pub(crate) type RecordVersions = HashMap<Uuid, Option<NaiveDateTime>>;

fn versions_query(kind: &DuplicateType, ids: &[Uuid], lock: bool) -> QueryBuilder<MySql> {
    let version = match kind.family {
        DuplicateFamily::Generic => kind
            .definition
            .system_field_by_key("updated_at")
            .map_or_else(
                || "CAST(NULL AS DATETIME)".to_owned(),
                |field| aliased("r", &field.column),
            ),
        _ => "r.updated_at".to_owned(),
    };
    let mut builder = QueryBuilder::<MySql>::new(format!(
        "SELECT r.id, {version} AS version FROM {} r WHERE {} IS NULL",
        kind.table(),
        aliased("r", kind.deleted_column())
    ));
    push_partition(&mut builder, &kind.definition, "r");
    builder.push(" AND r.id");
    push_ids(&mut builder, ids);
    if lock {
        builder.push(" FOR UPDATE");
    }
    builder
}

fn versions_from_rows(rows: &[sqlx::mysql::MySqlRow]) -> Result<RecordVersions, AppError> {
    rows.iter()
        .map(|row| Ok((uuid_column(row, "id")?, row.try_get("version")?)))
        .collect()
}

#[derive(Clone)]
pub(crate) struct MergeStore {
    pool: MySqlPool,
}

impl MergeStore {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    pub(crate) fn of(ctx: &Ctx<'_>) -> Self {
        Self::new(ctx.pool().clone())
    }

    pub(crate) async fn move_links(
        &self,
        ctx: &Ctx<'_>,
        tx: &mut Tx,
        kind: &DuplicateType,
        from: Uuid,
        to: Uuid,
    ) -> Result<BTreeMap<String, u64>, AppError> {
        let mut moved =
            aphel_sdk::records::move_core_references(tx, kind.as_str(), from, to).await?;
        for (_, owner) in ctx
            .app()
            .all_contributions::<dyn aphel_sdk::merges::MergeReferences>()
        {
            for (key, count) in owner
                .move_references(ctx, tx, kind.as_str(), from, to)
                .await?
            {
                *moved.entry(key).or_default() += count;
            }
        }
        Ok(moved)
    }

    pub(crate) async fn versions(
        &self,
        kind: &DuplicateType,
        ids: &[Uuid],
    ) -> Result<RecordVersions, AppError> {
        let rows = versions_query(kind, ids, false)
            .build()
            .fetch_all(&self.pool)
            .await?;
        versions_from_rows(&rows)
    }

    pub(crate) async fn lock_versions(
        &self,
        tx: &mut Tx,
        kind: &DuplicateType,
        ids: &[Uuid],
    ) -> Result<RecordVersions, AppError> {
        let rows = versions_query(kind, ids, true)
            .build()
            .fetch_all(&mut **tx)
            .await?;
        versions_from_rows(&rows)
    }

    pub(crate) async fn system_values(
        &self,
        kind: &DuplicateType,
        fields: &[&SystemField],
        id: Uuid,
    ) -> Result<Option<Map<String, Value>>, AppError> {
        let pairs = fields
            .iter()
            .map(|field| format!("'{}', {}", field.key.as_str(), aliased("r", &field.column)))
            .collect::<Vec<_>>()
            .join(", ");
        let mut builder = QueryBuilder::<MySql>::new(format!(
            "SELECT JSON_OBJECT({pairs}) FROM {} r WHERE r.id = ",
            kind.table()
        ));
        builder.push_bind(id.to_string());
        builder.push(format!(
            " AND {} IS NULL",
            aliased("r", kind.deleted_column())
        ));
        push_partition(&mut builder, &kind.definition, "r");
        let found: Option<Json<Value>> = builder
            .build_query_scalar()
            .fetch_optional(&self.pool)
            .await?;
        Ok(found.map(|Json(value)| match value {
            Value::Object(map) => map,
            _ => Map::new(),
        }))
    }

    pub(crate) async fn record_merge(
        &self,
        tx: &mut Tx,
        scope: &AccessScope,
        record_type: &str,
        merged_id: Uuid,
        survivor_id: Uuid,
        merged_at: DateTime<Utc>,
    ) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE record_merges SET survivor_id = ? \
             WHERE record_type = ? AND survivor_id = ?",
        )
        .bind(survivor_id.to_string())
        .bind(record_type)
        .bind(merged_id.to_string())
        .execute(&mut **tx)
        .await?;
        sqlx::query(
            "INSERT INTO record_merges (record_type, merged_id, survivor_id, merged_by, merged_at) \
             SELECT ?, ?, ?, u.id, ? FROM (SELECT 1) one LEFT JOIN users u ON u.id = ? \
             ON DUPLICATE KEY UPDATE survivor_id = VALUES(survivor_id), merged_by = VALUES(merged_by), \
             merged_at = VALUES(merged_at)",
        )
        .bind(record_type)
        .bind(merged_id.to_string())
        .bind(survivor_id.to_string())
        .bind(mapper::naive(merged_at))
        .bind(scope.user_id().as_str())
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub(crate) async fn merged_into(
        &self,
        record_type: &str,
        record_id: Uuid,
    ) -> Result<Option<Uuid>, AppError> {
        let survivor: Option<String> = sqlx::query_scalar(
            "SELECT survivor_id FROM record_merges \
             WHERE record_type = ? AND merged_id = ?",
        )
        .bind(record_type)
        .bind(record_id.to_string())
        .fetch_optional(&self.pool)
        .await?;
        Ok(survivor.and_then(|id| Uuid::parse_str(&id).ok()))
    }
}
