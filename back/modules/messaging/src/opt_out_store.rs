use sqlx::mysql::MySqlRow;
use sqlx::{MySqlPool, Row};
use uuid::Uuid;

use crate::opt_outs::{OPT_OUT_EXISTS, OptOut, OptOutScope, Purpose, normalize_address};
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::application::organization_admin::{ListWindow, Listing};
use aphel_sdk::platform::database::transaction::MySqlTx;
use aphel_sdk::platform::infrastructure::repositories::mapper;

const SELECT: &str =
    "SELECT id, address, scope, reason, source, created_by, created_at FROM communication_opt_outs";

fn opt_out_from_row(row: &MySqlRow) -> Result<OptOut, AppError> {
    let id: String = row.try_get("id")?;
    let scope: String = row.try_get("scope")?;
    Ok(OptOut {
        id: Uuid::parse_str(&id).map_err(|_| mapper::invalid_column("id", &id))?,
        address: row.try_get("address")?,
        scope: OptOutScope::parse(&scope).ok_or_else(|| mapper::invalid_column("scope", &scope))?,
        reason: row.try_get("reason")?,
        source: row.try_get("source")?,
        created_by: row.try_get("created_by")?,
        created_at: mapper::utc(row.try_get("created_at")?),
    })
}

pub async fn list(
    pool: &MySqlPool,
    search: Option<&str>,
    window: ListWindow,
) -> Result<Listing<OptOut>, AppError> {
    let pattern = search
        .map(str::trim)
        .filter(|search| !search.is_empty())
        .map(|search| {
            format!(
                "%{}%",
                search
                    .to_lowercase()
                    .replace('%', "\\%")
                    .replace('_', "\\_")
            )
        });
    let total: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM communication_opt_outs WHERE (? IS NULL OR address LIKE ?)",
    )
    .bind(&pattern)
    .bind(&pattern)
    .fetch_one(pool)
    .await?;
    let rows = sqlx::query(sqlx::AssertSqlSafe(format!(
        "{SELECT} WHERE (? IS NULL OR address LIKE ?) ORDER BY created_at DESC, id DESC LIMIT ? OFFSET ?"
    )))
    .bind(&pattern)
    .bind(&pattern)
    .bind(window.limit)
    .bind(window.offset)
    .fetch_all(pool)
    .await?;
    Ok(Listing {
        items: rows
            .iter()
            .map(opt_out_from_row)
            .collect::<Result<_, _>>()?,
        total: u64::try_from(total).unwrap_or_default(),
    })
}

pub async fn find(pool: &MySqlPool, id: Uuid) -> Result<Option<OptOut>, AppError> {
    sqlx::query(sqlx::AssertSqlSafe(format!("{SELECT} WHERE id = ?")))
        .bind(id.to_string())
        .fetch_optional(pool)
        .await?
        .as_ref()
        .map(opt_out_from_row)
        .transpose()
}

pub async fn insert(tx: &mut MySqlTx, opt_out: &OptOut) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO communication_opt_outs (id, address, scope, reason, source, created_by, created_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(opt_out.id.to_string())
    .bind(&opt_out.address)
    .bind(opt_out.scope.as_str())
    .bind(&opt_out.reason)
    .bind(&opt_out.source)
    .bind(&opt_out.created_by)
    .bind(mapper::naive(opt_out.created_at))
    .execute(&mut **tx)
    .await
    .map_err(|error| match &error {
        sqlx::Error::Database(database) if database.is_unique_violation() => {
            AppError::conflict_code(OPT_OUT_EXISTS, "this address already has an opt-out")
        }
        _ => AppError::from(error),
    })?;
    Ok(())
}

pub async fn delete(tx: &mut MySqlTx, id: Uuid) -> Result<(), AppError> {
    sqlx::query("DELETE FROM communication_opt_outs WHERE id = ?")
        .bind(id.to_string())
        .execute(&mut **tx)
        .await?;
    Ok(())
}

pub async fn opted_out(
    pool: &MySqlPool,
    address: &str,
    purpose: Purpose,
) -> Result<bool, AppError> {
    let address = normalize_address(address)?;
    let scope: Option<String> =
        sqlx::query_scalar("SELECT scope FROM communication_opt_outs WHERE address = ?")
            .bind(&address)
            .fetch_optional(pool)
            .await?;
    Ok(scope
        .as_deref()
        .and_then(OptOutScope::parse)
        .is_some_and(|scope| scope.blocks(purpose)))
}
