pub(crate) mod visitor;

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use sqlx::{AssertSqlSafe, MySqlPool, Row};

use crate::{DemoCleanup, NewDemoVisitor};
use aphel_kernel::Ctx;
use aphel_kernel::application::fields::is_sql_identifier;
use aphel_kernel::application::shared::{RecordRef, RecordType};
use aphel_kernel::database::transaction::MySqlTx;
use aphel_kernel::domain::UserId;
use aphel_kernel::errors::AppError;
use aphel_kernel::infrastructure::query::compiler::quoted;
use aphel_kernel::infrastructure::repositories::mapper;
use aphel_sdk::maintenance::DatasetSeed;
use aphel_sdk::records::cleanup_dataset_files;

const TRACKED_SELECT: &str = "SELECT record_type, record_id FROM demo_records";
const TRAILING_BUSINESS_TABLES: [&str; 2] = ["products", "custom_records"];

fn ordered_seeds(descending: bool) -> Vec<std::sync::Arc<dyn DatasetSeed>> {
    let mut seeds = aphel_kernel::modules::app().contributions::<dyn DatasetSeed>();
    seeds.sort_by_key(|seed| {
        if descending {
            -i64::from(seed.order())
        } else {
            i64::from(seed.order())
        }
    });
    seeds
}

async fn seed_counts(tx: &mut MySqlTx) -> Result<BTreeMap<String, u64>, AppError> {
    let mut counts = BTreeMap::new();
    for seed in ordered_seeds(false) {
        if let Some(owned) = seed.counts(tx).await? {
            counts.extend(owned);
            continue;
        }
        for record_type in seed.entities() {
            let Some(table) = record_table(&record_type) else {
                continue;
            };
            counts.extend(table_count(tx, &table).await?);
        }
    }
    for table in TRAILING_BUSINESS_TABLES {
        counts.extend(table_count(tx, table).await?);
    }
    Ok(counts)
}

async fn table_count(tx: &mut MySqlTx, table: &str) -> Result<Option<(String, u64)>, AppError> {
    let total: i64 = sqlx::query_scalar(AssertSqlSafe(format!(
        "SELECT COUNT(*) FROM {}",
        quoted(table)
    )))
    .fetch_one(&mut **tx)
    .await?;
    Ok((total > 0).then(|| (table.to_owned(), total as u64)))
}

async fn tracked_records(tx: &mut MySqlTx) -> Result<Vec<RecordRef>, AppError> {
    sqlx::query("SELECT record_type, record_id FROM demo_records")
        .fetch_all(&mut **tx)
        .await?
        .into_iter()
        .map(|row| {
            let record_type: String = row.try_get("record_type")?;
            let record_id: String = row.try_get("record_id")?;
            RecordRef::parse(&record_type, &record_id)
        })
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

async fn remove_seeded_records(
    tx: &mut MySqlTx,
    records: Option<&[RecordRef]>,
) -> Result<(), AppError> {
    for seed in ordered_seeds(true) {
        if seed.cleanup(tx, records).await? {
            continue;
        }
        for record_type in seed.entities() {
            let Some(table) = record_table(&record_type) else {
                continue;
            };
            let statement = match records {
                Some(_) => format!(
                    "DELETE FROM {} WHERE id IN (SELECT record_id FROM demo_records WHERE record_type = ?)",
                    quoted(&table)
                ),
                None => format!("DELETE FROM {}", quoted(&table)),
            };
            let mut query = sqlx::query(AssertSqlSafe(statement));
            if records.is_some() {
                query = query.bind(record_type.as_str());
            }
            query.execute(&mut **tx).await?;
        }
    }
    Ok(())
}

impl DemoStore {
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
        let mut tx = MySqlTx::from(self.pool.begin().await?);
        Ok(!seed_counts(&mut tx).await?.is_empty())
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

        let statements = [
            format!("DELETE FROM attachments WHERE (record_type, record_id) IN {tracked}"),
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
        ];
        for statement in statements {
            sqlx::query(AssertSqlSafe(statement))
                .execute(&mut **tx)
                .await?;
        }
        let tracked_list = tracked_records(tx).await?;
        remove_seeded_records(tx, Some(&tracked_list)).await?;
        sqlx::query(AssertSqlSafe(format!(
            "DELETE FROM field_definitions WHERE id IN {}",
            tracked_of("field")
        )))
        .execute(&mut **tx)
        .await?;
        sqlx::query("DELETE FROM demo_records")
            .execute(&mut **tx)
            .await?;

        let storage_keys = cleanup_dataset_files(tx, &attached_files, false).await?;
        Ok(DemoCleanup {
            counts,
            storage_keys,
        })
    }

    pub(crate) async fn wipe(&self, tx: &mut MySqlTx) -> Result<DemoCleanup, AppError> {
        let tracked = self.remove_tracked(tx).await?;
        let mut counts = seed_counts(tx).await?;
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
        remove_seeded_records(tx, None).await?;
        for table in TRAILING_BUSINESS_TABLES {
            sqlx::query(AssertSqlSafe(format!("DELETE FROM {table}")))
                .execute(&mut **tx)
                .await?;
        }
        let removed_users = sqlx::query(
            "DELETE FROM users WHERE role_id NOT IN (SELECT r.id FROM roles r WHERE r.`key` = 'admin')",
        )
        .execute(&mut **tx)
        .await?
        .rows_affected();
        if removed_users > 0 {
            counts.insert("users".to_owned(), removed_users);
        }
        let mut storage_keys = cleanup_dataset_files(tx, &[], true).await?;
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
