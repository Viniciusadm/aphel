use sqlx::{MySqlConnection, MySqlPool};

use crate::errors::AppError;
use crate::infrastructure::query::compiler::quoted;
use crate::modules::BoxFuture;

pub(crate) type DefaultsSeedFn =
    for<'c> fn(&'c mut MySqlConnection) -> BoxFuture<'c, Result<(), AppError>>;

pub struct DefaultsSeed(pub DefaultsSeedFn);

pub struct DefaultsProbe(pub &'static str);

pub async fn seed_defaults(connection: &mut MySqlConnection) -> Result<(), AppError> {
    for (_, seed) in crate::modules::app().all_contributions::<DefaultsSeed>() {
        (seed.0)(&mut *connection).await?;
    }
    Ok(())
}

fn probed_tables() -> Vec<&'static str> {
    let mut tables: Vec<&'static str> = Vec::new();
    for (_, probe) in crate::modules::app().all_contributions::<DefaultsProbe>() {
        if !tables.contains(&probe.0) {
            tables.push(probe.0);
        }
    }
    tables
}

pub async fn sync_defaults(pool: &MySqlPool) -> Result<u64, AppError> {
    let missing_condition = probed_tables()
        .into_iter()
        .map(|table| format!("NOT EXISTS (SELECT 1 FROM {})", quoted(table)))
        .collect::<Vec<_>>()
        .join(" OR ");
    if missing_condition.is_empty() {
        return Ok(0);
    }
    let missing: bool = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
        "SELECT EXISTS(SELECT 1 FROM organization_settings WHERE id = 1) AND ({missing_condition})"
    )))
    .fetch_one(pool)
    .await?;
    if !missing {
        return Ok(0);
    }
    let mut transaction = pool.begin().await?;
    seed_defaults(&mut transaction).await?;
    transaction.commit().await?;
    Ok(1)
}
