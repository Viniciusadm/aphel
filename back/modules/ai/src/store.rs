use chrono::{NaiveDate, NaiveDateTime, SecondsFormat, Utc};
use sqlx::mysql::MySqlRow;
use sqlx::{MySqlPool, QueryBuilder, Row};
use uuid::Uuid;

use crate::model::{AiCall, AiUsageTotals};
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::database::transaction::Tx;

pub(crate) struct NewCall<'a> {
    pub(crate) user_id: Option<Uuid>,
    pub(crate) feature: &'a str,
    pub(crate) provider: &'a str,
    pub(crate) model: &'a str,
    pub(crate) status: &'a str,
    pub(crate) input_tokens: Option<u32>,
    pub(crate) output_tokens: Option<u32>,
    pub(crate) duration_ms: u32,
    pub(crate) error_code: Option<&'a str>,
}

pub(crate) struct CallFilter {
    pub(crate) feature: Option<String>,
    pub(crate) status: Option<String>,
}

pub(crate) async fn reserve_call(
    pool: &MySqlPool,
    day: NaiveDate,
    limit: u32,
) -> Result<bool, AppError> {
    sqlx::query(
        "INSERT INTO ai_usage (day, calls) VALUES (?, 0) ON DUPLICATE KEY UPDATE calls = calls",
    )
    .bind(day)
    .execute(pool)
    .await?;
    let reserved = sqlx::query("UPDATE ai_usage SET calls = calls + 1 WHERE day = ? AND calls < ?")
        .bind(day)
        .bind(limit)
        .execute(pool)
        .await?;
    Ok(reserved.rows_affected() > 0)
}

pub(crate) async fn used_on(pool: &MySqlPool, day: NaiveDate) -> Result<u32, AppError> {
    let calls: Option<u32> = sqlx::query_scalar("SELECT calls FROM ai_usage WHERE day = ?")
        .bind(day)
        .fetch_optional(pool)
        .await?;
    Ok(calls.unwrap_or(0))
}

pub(crate) async fn record_call(pool: &MySqlPool, call: NewCall<'_>) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO ai_calls (id, user_id, feature, provider, model, status, \
         input_tokens, output_tokens, duration_ms, error_code) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(Uuid::now_v7().to_string())
    .bind(call.user_id.map(|id| id.to_string()))
    .bind(call.feature)
    .bind(call.provider)
    .bind(truncate(call.model, 120))
    .bind(call.status)
    .bind(call.input_tokens)
    .bind(call.output_tokens)
    .bind(call.duration_ms)
    .bind(call.error_code)
    .execute(pool)
    .await?;
    Ok(())
}

fn truncate(value: &str, max: usize) -> String {
    value.chars().take(max).collect()
}

fn timestamp(value: NaiveDateTime) -> String {
    value.and_utc().to_rfc3339_opts(SecondsFormat::Secs, true)
}

fn call_from_row(row: &MySqlRow) -> Result<AiCall, AppError> {
    Ok(AiCall {
        id: row.try_get("id")?,
        user_id: row.try_get("user_id")?,
        feature: row.try_get("feature")?,
        provider: row.try_get("provider")?,
        model: row.try_get("model")?,
        status: row.try_get("status")?,
        input_tokens: row.try_get("input_tokens")?,
        output_tokens: row.try_get("output_tokens")?,
        duration_ms: row.try_get("duration_ms")?,
        error_code: row.try_get("error_code")?,
        created_at: timestamp(row.try_get("created_at")?),
    })
}

fn push_filters(builder: &mut QueryBuilder<sqlx::MySql>, filter: &CallFilter) {
    builder.push(" WHERE 1 = 1");
    if let Some(feature) = &filter.feature {
        builder.push(" AND feature = ");
        builder.push_bind(feature.clone());
    }
    if let Some(status) = &filter.status {
        builder.push(" AND status = ");
        builder.push_bind(status.clone());
    }
}

pub(crate) async fn list_calls(
    pool: &MySqlPool,
    filter: &CallFilter,
    offset: u64,
    limit: u64,
) -> Result<(Vec<AiCall>, u64), AppError> {
    let mut count = QueryBuilder::new("SELECT COUNT(*) AS total FROM ai_calls");
    push_filters(&mut count, filter);
    let total: i64 = count.build().fetch_one(pool).await?.try_get("total")?;
    let mut select = QueryBuilder::new(
        "SELECT id, user_id, feature, provider, model, status, input_tokens, output_tokens, \
         duration_ms, error_code, created_at FROM ai_calls",
    );
    push_filters(&mut select, filter);
    select.push(" ORDER BY created_at DESC, id DESC LIMIT ");
    select.push_bind(limit);
    select.push(" OFFSET ");
    select.push_bind(offset);
    let rows = select.build().fetch_all(pool).await?;
    let calls = rows
        .iter()
        .map(call_from_row)
        .collect::<Result<_, AppError>>()?;
    Ok((calls, u64::try_from(total).unwrap_or(0)))
}

pub(crate) async fn totals_since(
    pool: &MySqlPool,
    since: NaiveDateTime,
) -> Result<AiUsageTotals, AppError> {
    let row = sqlx::query(
        "SELECT COUNT(*) AS calls, \
         CAST(COALESCE(SUM(CASE WHEN status = 'error' THEN 1 ELSE 0 END), 0) AS SIGNED) AS errors, \
         CAST(COALESCE(SUM(input_tokens), 0) AS SIGNED) AS input_tokens, \
         CAST(COALESCE(SUM(output_tokens), 0) AS SIGNED) AS output_tokens \
         FROM ai_calls WHERE created_at >= ?",
    )
    .bind(since)
    .fetch_one(pool)
    .await?;
    Ok(AiUsageTotals {
        calls: u64::try_from(row.try_get::<i64, _>("calls")?).unwrap_or(0),
        errors: u64::try_from(row.try_get::<i64, _>("errors")?).unwrap_or(0),
        input_tokens: u64::try_from(row.try_get::<i64, _>("input_tokens")?).unwrap_or(0),
        output_tokens: u64::try_from(row.try_get::<i64, _>("output_tokens")?).unwrap_or(0),
    })
}

pub(crate) fn thirty_days_ago() -> NaiveDateTime {
    (Utc::now() - chrono::Duration::days(30)).naive_utc()
}

pub(crate) async fn purge(tx: &mut Tx) -> Result<(), AppError> {
    for table in ["ai_calls", "ai_usage"] {
        sqlx::query(sqlx::AssertSqlSafe(format!("DELETE FROM {table}")))
            .execute(&mut **tx)
            .await?;
    }
    Ok(())
}
