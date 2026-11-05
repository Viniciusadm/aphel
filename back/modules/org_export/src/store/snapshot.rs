use std::collections::HashSet;

use sqlx::{AssertSqlSafe, Connection, MySqlConnection, MySqlPool};

use aphel_kernel::errors::AppError;
use aphel_sdk::platform::application::fields::is_sql_identifier;
use aphel_sdk::platform::infrastructure::query::compiler::quoted;

const PRESERVED_TABLES: [&str; 3] = ["_sqlx_migrations", "installed_modules", "module_migrations"];
const MAX_TRACKED_IDS: i64 = 50_000;
const DELETE_CHUNK: usize = 500;

#[derive(Debug, Clone, PartialEq, Eq)]
enum TableState {
    Empty,
    Ids(HashSet<String>),
    Untracked,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct InstallationSnapshot {
    tables: Vec<(String, TableState)>,
}

async fn base_tables(pool: &MySqlPool) -> Result<Vec<String>, AppError> {
    let tables: Vec<String> = sqlx::query_scalar(
        "SELECT table_name FROM information_schema.tables \
         WHERE table_schema = DATABASE() AND table_type = 'BASE TABLE' ORDER BY table_name",
    )
    .fetch_all(pool)
    .await?;
    Ok(tables
        .into_iter()
        .filter(|table| is_sql_identifier(table) && !PRESERVED_TABLES.contains(&table.as_str()))
        .collect())
}

async fn has_id_column(pool: &MySqlPool, table: &str) -> Result<bool, AppError> {
    let found: Option<i64> = sqlx::query_scalar(
        "SELECT 1 FROM information_schema.columns \
         WHERE table_schema = DATABASE() AND table_name = ? AND column_name = 'id'",
    )
    .bind(table)
    .fetch_optional(pool)
    .await?;
    Ok(found.is_some())
}

async fn ids(pool: &MySqlPool, table: &str) -> Result<HashSet<String>, AppError> {
    let ids: Vec<String> = sqlx::query_scalar(AssertSqlSafe(format!(
        "SELECT CAST(id AS CHAR) FROM {}",
        quoted(table)
    )))
    .fetch_all(pool)
    .await?;
    Ok(ids.into_iter().collect())
}

impl InstallationSnapshot {
    pub(crate) async fn capture(pool: &MySqlPool) -> Result<Self, AppError> {
        let mut tables = Vec::new();
        for table in base_tables(pool).await? {
            let count: i64 = sqlx::query_scalar(AssertSqlSafe(format!(
                "SELECT COUNT(*) FROM {}",
                quoted(&table)
            )))
            .fetch_one(pool)
            .await?;
            let state = if count == 0 {
                TableState::Empty
            } else if count <= MAX_TRACKED_IDS && has_id_column(pool, &table).await? {
                TableState::Ids(ids(pool, &table).await?)
            } else {
                TableState::Untracked
            };
            tables.push((table, state));
        }
        Ok(Self { tables })
    }

    pub(crate) async fn rollback(&self, pool: &MySqlPool) -> Result<(), AppError> {
        let mut connection = pool.acquire().await?.detach();
        let result = match sqlx::query("SET FOREIGN_KEY_CHECKS = 0")
            .execute(&mut connection)
            .await
        {
            Ok(_) => self.revert(&mut connection).await,
            Err(error) => Err(error.into()),
        };
        if let Err(error) = connection.close().await {
            tracing::warn!(%error, "failed to close the restore rollback connection");
        }
        result
    }

    async fn revert(&self, connection: &mut MySqlConnection) -> Result<(), AppError> {
        let mut result = Ok(());
        for (table, state) in &self.tables {
            let outcome = match state {
                TableState::Empty => {
                    sqlx::query(AssertSqlSafe(format!("DELETE FROM {}", quoted(table))))
                        .execute(&mut *connection)
                        .await
                        .map(|_| ())
                }
                TableState::Ids(kept) => {
                    let current: Vec<String> = match sqlx::query_scalar(AssertSqlSafe(format!(
                        "SELECT CAST(id AS CHAR) FROM {}",
                        quoted(table)
                    )))
                    .fetch_all(&mut *connection)
                    .await
                    {
                        Ok(current) => current,
                        Err(error) => {
                            result = Err(error.into());
                            continue;
                        }
                    };
                    let added: Vec<String> = current
                        .into_iter()
                        .filter(|id| !kept.contains(id))
                        .collect();
                    let mut outcome = Ok(());
                    for chunk in added.chunks(DELETE_CHUNK) {
                        let mut builder = sqlx::QueryBuilder::new(format!(
                            "DELETE FROM {} WHERE id IN (",
                            quoted(table)
                        ));
                        let mut separated = builder.separated(", ");
                        for id in chunk {
                            separated.push_bind(id.clone());
                        }
                        builder.push(")");
                        if let Err(error) = builder.build().execute(&mut *connection).await {
                            outcome = Err(error);
                            break;
                        }
                    }
                    outcome
                }
                TableState::Untracked => {
                    tracing::warn!(table = %table, "a table was not empty before the restore and could not be reverted");
                    Ok(())
                }
            };
            if let Err(error) = outcome {
                result = Err(error.into());
            }
        }
        result
    }
}
