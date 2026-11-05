pub(crate) mod visitor;

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use sqlx::{AssertSqlSafe, MySqlPool, Row};

use crate::{DemoCatalog, DemoCleanup, DemoStage, NewDemoVisitor};
use aphel_kernel::Ctx;
use aphel_kernel::application::fields::is_sql_identifier;
use aphel_kernel::application::shared::{RecordRef, RecordType};
use aphel_kernel::database::transaction::MySqlTx;
use aphel_kernel::domain::UserId;
use aphel_kernel::errors::AppError;
use aphel_kernel::infrastructure::query::compiler::quoted;
use aphel_kernel::infrastructure::repositories::mapper;
use aphel_sdk::maintenance::DatasetSeed;
use privacy::mysql::sql::{file_reference_columns, unreferenced_file_predicate};

const TRACKED_SELECT: &str = "SELECT record_type, record_id FROM demo_records";
const LEADING_BUSINESS_TABLES: [&str; 1] = ["tasks"];
const TRAILING_BUSINESS_TABLES: [&str; 3] = ["products", "custom_records", "notes"];

fn seeded_record_tables() -> Vec<String> {
    let mut seeds = aphel_kernel::modules::app().contributions::<dyn DatasetSeed>();
    seeds.sort_by_key(|seed| std::cmp::Reverse(seed.order()));
    seeds
        .iter()
        .flat_map(|seed| seed.entities())
        .filter_map(|record_type| record_table(&record_type))
        .collect()
}

fn business_tables() -> Vec<String> {
    LEADING_BUSINESS_TABLES
        .iter()
        .map(|table| (*table).to_owned())
        .chain(seeded_record_tables())
        .chain(
            TRAILING_BUSINESS_TABLES
                .iter()
                .map(|table| (*table).to_owned()),
        )
        .collect()
}

#[derive(Clone)]
pub(crate) struct DemoStore {
    pool: MySqlPool,
}

impl DemoStore {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    pub(crate) fn of(ctx: &Ctx<'_>) -> Self {
        Self::new(ctx.pool().clone())
    }
}

const NOTE_TYPE: &str = "notes.note";

fn record_table(record_type: &RecordType) -> Option<String> {
    if record_type.as_str() == NOTE_TYPE {
        return Some("notes".to_owned());
    }
    aphel_kernel::modules::app()
        .record_types()
        .get(record_type)
        .filter(|definition| definition.partition.is_none())
        .map(|definition| definition.table.clone())
}

async fn tracked_counts(tx: &mut MySqlTx) -> Result<BTreeMap<String, u64>, AppError> {
    sqlx::query("SELECT record_type, COUNT(*) AS total FROM demo_records GROUP BY record_type")
        .fetch_all(&mut **tx)
        .await?
        .into_iter()
        .map(|row| {
            let total: i64 = row.try_get("total")?;
            Ok((row.try_get("record_type")?, total.max(0) as u64))
        })
        .collect()
}

async fn record_counts(tx: &mut MySqlTx) -> Result<BTreeMap<String, u64>, AppError> {
    let mut counts = BTreeMap::new();
    for table in business_tables() {
        let total: i64 = sqlx::query_scalar(AssertSqlSafe(format!("SELECT COUNT(*) FROM {table}")))
            .fetch_one(&mut **tx)
            .await?;
        if total > 0 {
            counts.insert(table.to_owned(), total as u64);
        }
    }
    Ok(counts)
}

const TRACKED_RECORDS: &str = "*";
const MODULE_CLEANUP_BEFORE_RECORDS: [(&str, &str); 6] = [
    ("approval_requests", TRACKED_RECORDS),
    ("approval_policies", "approval_policy"),
    ("validation_rules", "validation_rule"),
    ("quotes", "quotes.quote"),
    ("price_lists", "price_list"),
    ("money_exchange_rates", "exchange_rate"),
];
const MODULE_CLEANUP_AFTER_RECORDS: [(&str, &str); 1] = [("products", "catalog_items.item")];

async fn table_exists(tx: &mut MySqlTx, table: &str) -> Result<bool, AppError> {
    let found: Option<i64> = sqlx::query_scalar(
        "SELECT 1 FROM information_schema.tables WHERE table_schema = DATABASE() AND table_name = ?",
    )
    .bind(table)
    .fetch_optional(&mut **tx)
    .await?;
    Ok(found.is_some())
}

async fn remove_module_rows(tx: &mut MySqlTx, targets: &[(&str, &str)]) -> Result<(), AppError> {
    for (table, record_type) in targets {
        if !is_sql_identifier(table) || !table_exists(tx, table).await? {
            continue;
        }
        if *record_type == TRACKED_RECORDS {
            sqlx::query(AssertSqlSafe(format!(
                "DELETE FROM {table} WHERE (record_type, record_id) IN ({TRACKED_SELECT})"
            )))
            .execute(&mut **tx)
            .await?;
        } else {
            sqlx::query(AssertSqlSafe(format!(
                "DELETE FROM {table} WHERE id IN (SELECT record_id FROM demo_records WHERE record_type = ?)"
            )))
            .bind(*record_type)
            .execute(&mut **tx)
            .await?;
        }
    }
    Ok(())
}

async fn remove_seeded_records(tx: &mut MySqlTx) -> Result<(), AppError> {
    let mut seeds = aphel_kernel::modules::app().contributions::<dyn DatasetSeed>();
    seeds.sort_by_key(|seed| std::cmp::Reverse(seed.order()));
    for record_type in seeds.iter().flat_map(|seed| seed.entities()) {
        let Some(table) = record_table(&record_type) else {
            continue;
        };
        sqlx::query(AssertSqlSafe(format!(
            "DELETE FROM {} WHERE id IN (SELECT record_id FROM demo_records WHERE record_type = ?)",
            quoted(&table)
        )))
        .bind(record_type.as_str())
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}

impl DemoStore {
    pub(crate) async fn key_exists(&self, table: &str, key: &str) -> Result<bool, AppError> {
        if !is_sql_identifier(table) {
            return Err(AppError::Internal(format!("invalid table name {table:?}")));
        }
        let found: Option<i64> = sqlx::query_scalar(AssertSqlSafe(format!(
            "SELECT 1 FROM {table} WHERE `key` = ? LIMIT 1"
        )))
        .bind(key)
        .fetch_optional(&self.pool)
        .await?;
        Ok(found.is_some())
    }

    pub(crate) async fn admin_role_id(&self) -> Result<Option<String>, AppError> {
        Ok(
            sqlx::query_scalar("SELECT id FROM roles WHERE `key` = 'admin' LIMIT 1")
                .fetch_optional(&self.pool)
                .await?,
        )
    }

    pub(crate) async fn active_users(&self) -> Result<Vec<String>, AppError> {
        Ok(sqlx::query_scalar(
            "SELECT u.id FROM users u JOIN roles r ON r.id = u.role_id WHERE u.status = 'active' \
             ORDER BY r.`key` = 'admin' DESC, u.joined_at, u.id",
        )
        .fetch_all(&self.pool)
        .await?)
    }

    pub(crate) async fn admin_user(&self) -> Result<Option<UserId>, AppError> {
        let user_id: Option<String> = sqlx::query_scalar(
            "SELECT u.id FROM users u JOIN roles r ON r.id = u.role_id \
             WHERE u.status = 'active' AND r.`key` = 'admin' \
             ORDER BY u.joined_at, u.id LIMIT 1",
        )
        .fetch_optional(&self.pool)
        .await?;
        user_id
            .map(|id| mapper::typed_id(id, UserId::new))
            .transpose()
    }

    pub(crate) async fn has_business_records(&self) -> Result<bool, AppError> {
        for table in business_tables() {
            let exists: i64 = sqlx::query_scalar(AssertSqlSafe(format!(
                "SELECT EXISTS(SELECT 1 FROM {table})"
            )))
            .fetch_one(&self.pool)
            .await?;
            if exists != 0 {
                return Ok(true);
            }
        }
        Ok(false)
    }

    pub(crate) async fn catalog(&self) -> Result<DemoCatalog, AppError> {
        let stages = sqlx::query(
            "SELECT s.id, s.kind FROM workflow_stages s \
             JOIN workflow p ON p.id = s.pipeline_id \
             WHERE p.is_default = TRUE AND s.is_active = TRUE \
             ORDER BY s.position, s.id",
        )
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(|row| {
            Ok(DemoStage {
                id: row.try_get("id")?,
                kind: row.try_get("kind")?,
            })
        })
        .collect::<Result<Vec<_>, AppError>>()?;
        let lost_reason_ids: Vec<String> = sqlx::query_scalar(
            "SELECT id FROM lost_reasons WHERE is_active = TRUE ORDER BY position, id",
        )
        .fetch_all(&self.pool)
        .await?;
        let task_types = sqlx::query(
            "SELECT `key`, id FROM task_types WHERE is_active = TRUE ORDER BY position, id",
        )
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(|row| Ok((row.try_get("key")?, row.try_get("id")?)))
        .collect::<Result<BTreeMap<String, String>, AppError>>()?;
        Ok(DemoCatalog {
            stages,
            lost_reason_ids,
            task_types,
        })
    }

    pub(crate) async fn tracked_count(&self) -> Result<u64, AppError> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM demo_records")
            .fetch_one(&self.pool)
            .await?;
        Ok(count.max(0) as u64)
    }

    pub(crate) async fn track(&self, record: &RecordRef) -> Result<(), AppError> {
        sqlx::query("INSERT IGNORE INTO demo_records (record_type, record_id) VALUES (?, ?)")
            .bind(record.record_type.as_str())
            .bind(record.record_id.to_string())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub(crate) async fn backdate(
        &self,
        record: &RecordRef,
        created_at: DateTime<Utc>,
        last_change_at: DateTime<Utc>,
    ) -> Result<(), AppError> {
        let record_id = record.record_id.to_string();
        let last_change_at = last_change_at.max(created_at);
        let mut transaction = self.pool.begin().await?;
        if let Some(table) = record_table(&record.record_type) {
            let statement =
                format!("UPDATE {table} SET created_at = ?, updated_at = ? WHERE id = ?");
            let query = sqlx::query(AssertSqlSafe(statement))
                .bind(mapper::naive(created_at))
                .bind(mapper::naive(last_change_at));
            query.bind(&record_id).execute(&mut *transaction).await?;
        }
        for table in ["domain_events", "audit_logs"] {
            let ids: Vec<String> = sqlx::query_scalar(AssertSqlSafe(format!(
                "SELECT id FROM {table} WHERE record_type = ? AND record_id = ? ORDER BY occurred_at, id"
            )))
            .bind(record.record_type.as_str())
            .bind(&record_id)
            .fetch_all(&mut *transaction)
            .await?;
            let span = last_change_at - created_at;
            let steps = ids.len().saturating_sub(1).max(1) as i32;
            for (index, id) in ids.iter().enumerate() {
                let occurred_at = created_at + span * index as i32 / steps;
                sqlx::query(AssertSqlSafe(format!(
                    "UPDATE {table} SET occurred_at = ? WHERE id = ?"
                )))
                .bind(mapper::naive(occurred_at))
                .bind(id)
                .execute(&mut *transaction)
                .await?;
            }
        }
        transaction.commit().await?;
        Ok(())
    }

    pub(crate) async fn remove_tracked(&self, tx: &mut MySqlTx) -> Result<DemoCleanup, AppError> {
        let counts = tracked_counts(tx).await?;
        let tracked = format!("({TRACKED_SELECT})");
        let tracked_of = |record_type: &str| {
            format!("(SELECT record_id FROM demo_records WHERE record_type = '{record_type}')")
        };
        let attached_files: Vec<(String, String)> = sqlx::query_as(AssertSqlSafe(format!(
            "SELECT DISTINCT f.id, f.storage_key FROM attachments a JOIN files f ON f.id = a.file_id \
             WHERE (a.record_type, a.record_id) IN {tracked}"
        )))
        .fetch_all(&mut **tx)
        .await?;

        remove_module_rows(tx, &MODULE_CLEANUP_BEFORE_RECORDS).await?;
        let statements = [
            format!("DELETE FROM attachments WHERE (record_type, record_id) IN {tracked}"),
            format!("DELETE FROM notifications WHERE (record_type, record_id) IN {tracked}"),
            format!("DELETE FROM domain_events WHERE (record_type, record_id) IN {tracked}"),
            format!("DELETE FROM audit_logs WHERE (record_type, record_id) IN {tracked}"),
            format!(
                "DELETE FROM custom_field_values WHERE (record_type, record_id) IN {tracked} OR field_id IN {}",
                tracked_of("field")
            ),
            format!(
                "DELETE FROM custom_field_multi_values WHERE (record_type, record_id) IN {tracked} OR field_id IN {}",
                tracked_of("field")
            ),
            format!(
                "DELETE FROM notes WHERE id IN {} OR (record_type, record_id) IN {tracked}",
                tracked_of("notes.note")
            ),
            format!("DELETE FROM task_links WHERE (record_type, record_id) IN {tracked}"),
            format!(
                "UPDATE tasks SET record_type = NULL, record_id = NULL WHERE (record_type, record_id) IN {tracked}"
            ),
            format!("DELETE FROM tasks WHERE id IN {}", tracked_of("tasks.task")),
            format!(
                "DELETE FROM field_definitions WHERE id IN {}",
                tracked_of("field")
            ),
        ];
        for statement in statements {
            sqlx::query(AssertSqlSafe(statement))
                .execute(&mut **tx)
                .await?;
        }
        remove_seeded_records(tx).await?;
        remove_module_rows(tx, &MODULE_CLEANUP_AFTER_RECORDS).await?;
        sqlx::query("DELETE FROM demo_records")
            .execute(&mut **tx)
            .await?;

        let mut storage_keys = Vec::new();
        let unreferenced = unreferenced_file_predicate(&file_reference_columns(&self.pool).await?);
        for (file_id, storage_key) in attached_files {
            let deleted = sqlx::query(AssertSqlSafe(format!(
                "DELETE f FROM files f WHERE f.id = ? AND {unreferenced}"
            )))
            .bind(&file_id)
            .execute(&mut **tx)
            .await?
            .rows_affected();
            if deleted > 0 {
                storage_keys.push(storage_key);
            }
        }
        Ok(DemoCleanup {
            counts,
            storage_keys,
        })
    }

    pub(crate) async fn wipe(&self, tx: &mut MySqlTx) -> Result<DemoCleanup, AppError> {
        let tracked = self.remove_tracked(tx).await?;
        let mut counts = record_counts(tx).await?;
        for (record_type, count) in tracked.counts {
            *counts.entry(record_type).or_default() += count;
        }
        let linked_tables: Vec<String> = sqlx::query_scalar(
            "SELECT c.table_name FROM information_schema.columns c \
             JOIN information_schema.columns t ON t.table_schema = c.table_schema \
               AND t.table_name = c.table_name AND t.column_name = 'record_type' \
             JOIN information_schema.tables s ON s.table_schema = c.table_schema \
               AND s.table_name = c.table_name AND s.table_type = 'BASE TABLE' \
             WHERE c.table_schema = DATABASE() AND c.column_name = 'record_id' \
             ORDER BY c.table_name",
        )
        .fetch_all(&mut **tx)
        .await?;
        for table in linked_tables
            .iter()
            .filter(|table| is_sql_identifier(table))
        {
            sqlx::query(AssertSqlSafe(format!("DELETE FROM {}", quoted(table))))
                .execute(&mut **tx)
                .await?;
        }
        for table in business_tables() {
            sqlx::query(AssertSqlSafe(format!("DELETE FROM {table}")))
                .execute(&mut **tx)
                .await?;
        }
        let kept_files = "SELECT logo_file_id FROM organization_settings WHERE logo_file_id IS NOT NULL \
             UNION SELECT u.avatar_file_id FROM users u JOIN roles r ON r.id = u.role_id \
             WHERE r.`key` = 'admin' AND u.avatar_file_id IS NOT NULL";
        let storage_keys: Vec<String> = sqlx::query_scalar(AssertSqlSafe(format!(
            "SELECT storage_key FROM files WHERE id NOT IN ({kept_files})"
        )))
        .fetch_all(&mut **tx)
        .await?;
        sqlx::query(AssertSqlSafe(format!(
            "DELETE FROM files WHERE id NOT IN ({kept_files})"
        )))
        .execute(&mut **tx)
        .await?;
        let removed_users = sqlx::query(
            "DELETE FROM users WHERE role_id NOT IN (SELECT r.id FROM roles r WHERE r.`key` = 'admin')",
        )
        .execute(&mut **tx)
        .await?
        .rows_affected();
        if removed_users > 0 {
            counts.insert("users".to_owned(), removed_users);
        }
        let mut storage_keys = storage_keys;
        storage_keys.extend(tracked.storage_keys);
        Ok(DemoCleanup {
            counts,
            storage_keys,
        })
    }

    pub(crate) async fn ensure_visitor(&self, visitor: &NewDemoVisitor) -> Result<(), AppError> {
        visitor::ensure_visitor(&self.pool, visitor).await
    }
}
