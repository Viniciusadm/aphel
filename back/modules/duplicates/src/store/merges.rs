use std::collections::{BTreeMap, HashMap};

use chrono::{DateTime, NaiveDateTime, Utc};
use serde_json::{Map, Value};
use sqlx::types::Json;
use sqlx::{MySql, MySqlPool, QueryBuilder, Row};
use uuid::Uuid;

use crate::DuplicateType;
use crate::domain::DuplicateFamily;
use aphel_kernel::Ctx;
use aphel_kernel::application::fields::{RecordTypeRegistry, RelationPath, SystemField};
use aphel_kernel::database::transaction::MySqlTx;
use aphel_kernel::domain::access::AccessScope;
use aphel_kernel::errors::AppError;
use aphel_kernel::infrastructure::fields::mysql_definitions::uuid_column;
use aphel_kernel::infrastructure::query::compiler::{aliased, push_partition};
use aphel_kernel::infrastructure::repositories::mapper;
use aphel_kernel::infrastructure::repositories::mapper::push_ids;

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

struct Move<'a> {
    record_type: &'a str,
    from: String,
    to: String,
}

impl Move<'_> {
    async fn update(&self, tx: &mut MySqlTx, sql: &str) -> Result<u64, AppError> {
        let result = sqlx::query(sqlx::AssertSqlSafe(sql.to_owned()))
            .bind(self.to.as_str())
            .bind(self.from.as_str())
            .execute(&mut **tx)
            .await?;
        Ok(result.rows_affected())
    }

    async fn record_rows(&self, tx: &mut MySqlTx, table: &str) -> Result<u64, AppError> {
        let result = sqlx::query(sqlx::AssertSqlSafe(format!(
            "UPDATE {table} SET record_id = ? WHERE record_type = ? AND record_id = ?"
        )))
        .bind(self.to.as_str())
        .bind(self.record_type)
        .bind(self.from.as_str())
        .execute(&mut **tx)
        .await?;
        Ok(result.rows_affected())
    }

    async fn unique_record_rows(&self, tx: &mut MySqlTx, table: &str) -> Result<u64, AppError> {
        let moved = sqlx::query(sqlx::AssertSqlSafe(format!(
            "UPDATE IGNORE {table} SET record_id = ? WHERE record_type = ? AND record_id = ?"
        )))
        .bind(self.to.as_str())
        .bind(self.record_type)
        .bind(self.from.as_str())
        .execute(&mut **tx)
        .await?
        .rows_affected();
        let left = sqlx::query(sqlx::AssertSqlSafe(format!(
            "DELETE FROM {table} WHERE record_type = ? AND record_id = ?"
        )))
        .bind(self.record_type)
        .bind(self.from.as_str())
        .execute(&mut **tx)
        .await?
        .rows_affected();
        Ok(moved.saturating_sub(left))
    }

    async fn unique_column_rows(
        &self,
        tx: &mut MySqlTx,
        table: &str,
        column: &str,
    ) -> Result<u64, AppError> {
        let moved = self
            .update(
                tx,
                &format!("UPDATE IGNORE {table} SET {column} = ? WHERE {column} = ?"),
            )
            .await?;
        let left = sqlx::query(sqlx::AssertSqlSafe(format!(
            "DELETE FROM {table} WHERE {column} = ?"
        )))
        .bind(self.from.as_str())
        .execute(&mut **tx)
        .await?
        .rows_affected();
        Ok(moved.saturating_sub(left))
    }

    async fn custom_references(&self, tx: &mut MySqlTx) -> Result<u64, AppError> {
        let result = sqlx::query(
            "UPDATE custom_field_values v JOIN field_definitions f ON f.id = v.field_id \
             SET v.value_text = ? \
             WHERE f.field_type = 'record' \
             AND JSON_UNQUOTE(JSON_EXTRACT(f.config, '$.target_record_type')) = ? \
             AND v.value_text = ?",
        )
        .bind(self.to.as_str())
        .bind(self.record_type)
        .bind(self.from.as_str())
        .execute(&mut **tx)
        .await?;
        Ok(result.rows_affected())
    }
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
        tx: &mut MySqlTx,
        kind: &DuplicateType,
        registry: &RecordTypeRegistry,
        from: Uuid,
        to: Uuid,
    ) -> Result<BTreeMap<String, u64>, AppError> {
        let step = Move {
            record_type: kind.as_str(),
            from: from.to_string(),
            to: to.to_string(),
        };
        let mut moved = BTreeMap::new();
        moved.insert(
            "activities".to_owned(),
            step.record_rows(tx, "activities").await?
                + step.unique_record_rows(tx, "activity_links").await?,
        );
        moved.insert("notes".to_owned(), step.record_rows(tx, "notes").await?);
        moved.insert(
            "attachments".to_owned(),
            step.unique_record_rows(tx, "attachments").await?,
        );
        moved.insert(
            "tags".to_owned(),
            step.unique_record_rows(tx, "taggings").await?,
        );
        moved.insert("references".to_owned(), step.custom_references(tx).await?);
        moved.insert(
            "relationships".to_owned(),
            step.unique_column_rows(tx, "record_relations", "from_id")
                .await?
                + step
                    .unique_column_rows(tx, "record_relations", "to_id")
                    .await?,
        );
        match kind.family {
            DuplicateFamily::Contact => {
                moved.insert(
                    "deals".to_owned(),
                    step.update(tx, "UPDATE deals SET contact_id = ? WHERE contact_id = ?")
                        .await?
                        + step
                            .unique_column_rows(tx, "deal_contacts", "contact_id")
                            .await?,
                );
                moved.insert(
                    "organizations".to_owned(),
                    step.unique_column_rows(tx, "people_organizations", "contact_id")
                        .await?,
                );
                sqlx::query(
                    "DELETE l FROM people_organizations l JOIN people c ON c.id = l.contact_id \
                     WHERE l.contact_id = ? AND l.company_id = c.company_id",
                )
                .bind(step.to.as_str())
                .execute(&mut **tx)
                .await?;
            }
            DuplicateFamily::Company => {
                moved.insert(
                    "people".to_owned(),
                    step.update(tx, "UPDATE people SET company_id = ? WHERE company_id = ?")
                        .await?
                        + step
                            .unique_column_rows(tx, "people_organizations", "company_id")
                            .await?,
                );
                moved.insert(
                    "deals".to_owned(),
                    step.update(tx, "UPDATE deals SET company_id = ? WHERE company_id = ?")
                        .await?,
                );
                sqlx::query(
                    "DELETE l FROM people_organizations l JOIN people c ON c.id = l.contact_id \
                     WHERE l.company_id = ? AND c.company_id = l.company_id",
                )
                .bind(step.to.as_str())
                .execute(&mut **tx)
                .await?;
            }
            DuplicateFamily::Generic => {
                let mut total = 0;
                for definition in registry.definitions() {
                    for relation in definition
                        .relations
                        .iter()
                        .filter(|relation| relation.target == *kind.record_type())
                    {
                        for path in &relation.paths {
                            if let RelationPath::BelongsTo { column } = path {
                                total += step
                                    .update(
                                        tx,
                                        &format!(
                                            "UPDATE {table} SET {column} = ? WHERE {column} = ?",
                                            table = definition.table,
                                        ),
                                    )
                                    .await?;
                            }
                        }
                    }
                }
                moved.insert("related".to_owned(), total);
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
        tx: &mut MySqlTx,
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

    pub(crate) async fn soft_delete(
        &self,
        tx: &mut MySqlTx,
        kind: &DuplicateType,
        id: Uuid,
        now: DateTime<Utc>,
    ) -> Result<(), AppError> {
        let deleted = aliased("r", kind.deleted_column());
        let mut builder =
            QueryBuilder::<MySql>::new(format!("UPDATE {} r SET {deleted} = ", kind.table()));
        builder
            .push_bind(mapper::naive(now))
            .push(" WHERE r.id = ")
            .push_bind(id.to_string())
            .push(format!(" AND {deleted} IS NULL"));
        push_partition(&mut builder, &kind.definition, "r");
        builder.build().execute(&mut **tx).await?;
        Ok(())
    }

    pub(crate) async fn copy_columns(
        &self,
        tx: &mut MySqlTx,
        kind: &DuplicateType,
        from: Uuid,
        to: Uuid,
        columns: &[String],
        now: DateTime<Utc>,
    ) -> Result<(), AppError> {
        if columns.is_empty() {
            return Ok(());
        }
        let assignments = columns
            .iter()
            .map(|column| format!("{} = {}", aliased("p", column), aliased("d", column)))
            .collect::<Vec<_>>()
            .join(", ");
        let mut builder = QueryBuilder::<MySql>::new(format!(
            "UPDATE {table} p JOIN {table} d ON d.id = ",
            table = kind.table()
        ));
        builder
            .push_bind(from.to_string())
            .push(format!(" SET {assignments}"));
        if kind.definition.system_field_by_key("updated_at").is_some() {
            builder
                .push(", p.updated_at = ")
                .push_bind(mapper::naive(now));
        }
        builder.push(" WHERE p.id = ").push_bind(to.to_string());
        push_partition(&mut builder, &kind.definition, "p");
        builder.build().execute(&mut **tx).await?;
        Ok(())
    }

    pub(crate) async fn record_merge(
        &self,
        tx: &mut MySqlTx,
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
