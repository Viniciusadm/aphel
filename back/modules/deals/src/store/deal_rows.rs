use aphel_kernel::errors::AppError;
use aphel_sdk::platform::application::fields::EntityDefinition;
use aphel_sdk::platform::database::Db;
use aphel_sdk::platform::database::transaction::Tx;
use aphel_sdk::platform::domain::access::{AccessScope, RecordScopeFilter};
use aphel_sdk::platform::infrastructure::query::compiler::push_record_scope;
use chrono::NaiveDateTime;
use sqlx::{MySql, QueryBuilder};
use uuid::Uuid;

pub(crate) async fn stage_counts(
    pool: &Db,
    scope: &AccessScope,
    definition: &EntityDefinition,
    visibility: &RecordScopeFilter,
    pipeline_id: Uuid,
) -> Result<Vec<(String, i64)>, AppError> {
    let mut query = QueryBuilder::<MySql>::new(
        "SELECT r.stage_id, COUNT(*) FROM deals r WHERE r.deleted_at IS NULL AND r.pipeline_id = ",
    );
    query.push_bind(pipeline_id.to_string());
    push_record_scope(&mut query, scope, definition, visibility);
    query.push(" GROUP BY r.stage_id");
    Ok(query
        .build_query_as::<(String, i64)>()
        .fetch_all(pool)
        .await?)
}

pub(crate) enum UsageColumn {
    Stage,
    Pipeline,
}

pub(crate) async fn usage(
    pool: &Db,
    column: UsageColumn,
    id: Uuid,
) -> Result<(i64, u64), AppError> {
    let sql = match column {
        UsageColumn::Stage => {
            "SELECT COUNT(*), CAST(COALESCE(SUM(deleted_at IS NULL), 0) AS UNSIGNED) FROM deals WHERE stage_id = ?"
        }
        UsageColumn::Pipeline => {
            "SELECT COUNT(*), CAST(COALESCE(SUM(deleted_at IS NULL), 0) AS UNSIGNED) FROM deals WHERE pipeline_id = ?"
        }
    };
    Ok(sqlx::query_as(sql)
        .bind(id.to_string())
        .fetch_one(pool)
        .await?)
}

pub(crate) async fn location(pool: &Db, id: Uuid) -> Result<Option<(String, String)>, AppError> {
    Ok(sqlx::query_as(
        "SELECT pipeline_id, stage_id FROM deals WHERE id = ? AND deleted_at IS NULL",
    )
    .bind(id.to_string())
    .fetch_optional(pool)
    .await?)
}

pub(crate) async fn lock_stage(
    tx: &mut Tx,
    stage_id: Uuid,
) -> Result<Vec<(String, bool)>, AppError> {
    Ok(sqlx::query_as(
        "SELECT id, deleted_at IS NOT NULL FROM deals WHERE stage_id = ? ORDER BY id FOR UPDATE",
    )
    .bind(stage_id.to_string())
    .fetch_all(&mut **tx)
    .await?)
}

pub(crate) async fn move_stage(
    tx: &mut Tx,
    from: Uuid,
    to: Uuid,
    pipeline_id: Uuid,
    at: NaiveDateTime,
) -> Result<(), AppError> {
    sqlx::query("UPDATE deals SET stage_id = ?, pipeline_id = ?, stage_entered_at = ?, updated_at = ? WHERE stage_id = ?")
        .bind(to.to_string())
        .bind(pipeline_id.to_string())
        .bind(at)
        .bind(at)
        .bind(from.to_string())
        .execute(&mut **tx)
        .await?;
    Ok(())
}

pub(crate) async fn title_and_owner(
    pool: &Db,
    id: Uuid,
) -> Result<Option<(String, Option<String>)>, AppError> {
    Ok(
        sqlx::query_as("SELECT title, owner_id FROM deals WHERE id = ? AND deleted_at IS NULL")
            .bind(id.to_string())
            .fetch_optional(pool)
            .await?,
    )
}

pub(crate) async fn title(pool: &Db, id: &str) -> Result<Option<String>, AppError> {
    Ok(
        sqlx::query_scalar("SELECT title FROM deals WHERE id = ? AND deleted_at IS NULL")
            .bind(id)
            .fetch_optional(pool)
            .await?,
    )
}

pub(crate) async fn renewal_of(
    pool: &Db,
    original_id: Uuid,
) -> Result<Option<(String, Option<NaiveDateTime>)>, AppError> {
    Ok(
        sqlx::query_as("SELECT id, deleted_at FROM deals WHERE renewal_of_deal_id = ?")
            .bind(original_id.to_string())
            .fetch_optional(pool)
            .await?,
    )
}

pub(crate) async fn unlink_contact(tx: &mut Tx, contact_id: Uuid) -> Result<(), AppError> {
    sqlx::query("DELETE FROM deal_contacts WHERE contact_id = ?")
        .bind(contact_id.to_string())
        .execute(&mut **tx)
        .await?;
    Ok(())
}

pub(crate) async fn open_deals_of_contacts(
    pool: &Db,
    contact_ids: &[Uuid],
) -> Result<Vec<String>, AppError> {
    let mut query = QueryBuilder::<MySql>::new(
        "SELECT id FROM deals WHERE status = 'open' AND deleted_at IS NULL AND archived_at IS NULL AND (contact_id IN (",
    );
    let mut separated = query.separated(", ");
    for id in contact_ids {
        separated.push_bind(id.to_string());
    }
    query.push(") OR id IN (SELECT deal_id FROM deal_contacts WHERE contact_id IN (");
    let mut separated = query.separated(", ");
    for id in contact_ids {
        separated.push_bind(id.to_string());
    }
    query.push(")))");
    Ok(query.build_query_scalar::<String>().fetch_all(pool).await?)
}

#[derive(sqlx::FromRow)]
pub(crate) struct RenewalRow {
    pub id: String,
    pub title: String,
    pub amount: Option<rust_decimal::Decimal>,
    pub currency: String,
    pub stage_id: String,
    pub contract_end: Option<chrono::NaiveDate>,
    pub owner_id: Option<String>,
    pub owner_name: Option<String>,
}

pub(crate) async fn renewal_rows(pool: &Db, ids: &[String]) -> Result<Vec<RenewalRow>, AppError> {
    let mut query = QueryBuilder::<MySql>::new(
        "SELECT d.id, d.title, d.amount, d.currency, d.stage_id, d.contract_end, d.owner_id, u.display_name AS owner_name FROM deals d LEFT JOIN users u ON u.id = d.owner_id WHERE d.id IN (",
    );
    let mut list = query.separated(", ");
    for id in ids {
        list.push_bind(id.clone());
    }
    query.push(")");
    Ok(query.build_query_as().fetch_all(pool).await?)
}
