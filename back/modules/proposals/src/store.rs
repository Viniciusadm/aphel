use chrono::{DateTime, NaiveDateTime, Utc};
use sqlx::mysql::MySqlRow;
use sqlx::{MySqlPool, Row};
use uuid::Uuid;

use aphel_kernel::database::transaction::MySqlTx;
use aphel_kernel::errors::AppError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TemplateRow {
    pub(crate) id: Uuid,
    pub(crate) key: String,
    pub(crate) name: String,
    pub(crate) description: Option<String>,
    pub(crate) body: String,
    pub(crate) is_active: bool,
    pub(crate) is_default: bool,
    pub(crate) created_at: DateTime<Utc>,
    pub(crate) updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TemplateValues {
    pub(crate) key: String,
    pub(crate) name: String,
    pub(crate) description: Option<String>,
    pub(crate) body: String,
    pub(crate) is_active: bool,
    pub(crate) is_default: bool,
}

impl From<&TemplateRow> for TemplateValues {
    fn from(row: &TemplateRow) -> Self {
        Self {
            key: row.key.clone(),
            name: row.name.clone(),
            description: row.description.clone(),
            body: row.body.clone(),
            is_active: row.is_active,
            is_default: row.is_default,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DocumentRow {
    pub(crate) id: Uuid,
    pub(crate) deal_id: Uuid,
    pub(crate) template_id: Option<Uuid>,
    pub(crate) template_name: String,
    pub(crate) number: u32,
    pub(crate) file_id: Uuid,
    pub(crate) file_name: String,
    pub(crate) file_size: u64,
    pub(crate) generated_by: Option<Uuid>,
    pub(crate) generated_by_name: Option<String>,
    pub(crate) created_at: DateTime<Utc>,
}

const TEMPLATE_COLUMNS: &str =
    "id, template_key, name, description, body, is_active, is_default, created_at, updated_at";

fn uuid(row: &MySqlRow, column: &str) -> Result<Uuid, AppError> {
    let raw: String = row.try_get(column)?;
    Uuid::parse_str(&raw).map_err(|error| AppError::Internal(error.to_string()))
}

fn optional_uuid(row: &MySqlRow, column: &str) -> Result<Option<Uuid>, AppError> {
    let raw: Option<String> = row.try_get(column)?;
    raw.map(|raw| Uuid::parse_str(&raw))
        .transpose()
        .map_err(|error| AppError::Internal(error.to_string()))
}

fn utc(row: &MySqlRow, column: &str) -> Result<DateTime<Utc>, AppError> {
    let value: NaiveDateTime = row.try_get(column)?;
    Ok(value.and_utc())
}

fn template(row: &MySqlRow) -> Result<TemplateRow, AppError> {
    Ok(TemplateRow {
        id: uuid(row, "id")?,
        key: row.try_get("template_key")?,
        name: row.try_get("name")?,
        description: row.try_get("description")?,
        body: row.try_get("body")?,
        is_active: row.try_get("is_active")?,
        is_default: row.try_get("is_default")?,
        created_at: utc(row, "created_at")?,
        updated_at: utc(row, "updated_at")?,
    })
}

fn document(row: &MySqlRow) -> Result<DocumentRow, AppError> {
    let number: u32 = row.try_get("number")?;
    let file_size: u64 = row.try_get("file_size")?;
    Ok(DocumentRow {
        id: uuid(row, "id")?,
        deal_id: uuid(row, "deal_id")?,
        template_id: optional_uuid(row, "template_id")?,
        template_name: row.try_get("template_name")?,
        number,
        file_id: uuid(row, "file_id")?,
        file_name: row.try_get("file_name")?,
        file_size,
        generated_by: optional_uuid(row, "generated_by")?,
        generated_by_name: row.try_get("generated_by_name")?,
        created_at: utc(row, "created_at")?,
    })
}

pub(crate) async fn list_templates(
    pool: &MySqlPool,
    only_active: bool,
) -> Result<Vec<TemplateRow>, AppError> {
    let sql = format!(
        "SELECT {TEMPLATE_COLUMNS} FROM proposals_templates{} ORDER BY is_default DESC, name, id",
        if only_active {
            " WHERE is_active = 1"
        } else {
            ""
        }
    );
    sqlx::query(sqlx::AssertSqlSafe(sql))
        .fetch_all(pool)
        .await?
        .iter()
        .map(template)
        .collect()
}

pub(crate) async fn find_template(
    executor: impl sqlx::MySqlExecutor<'_>,
    id: Uuid,
) -> Result<Option<TemplateRow>, AppError> {
    let sql = format!("SELECT {TEMPLATE_COLUMNS} FROM proposals_templates WHERE id = ?");
    sqlx::query(sqlx::AssertSqlSafe(sql))
        .bind(id.to_string())
        .fetch_optional(executor)
        .await?
        .as_ref()
        .map(template)
        .transpose()
}

pub(crate) async fn find_template_by_key(
    executor: impl sqlx::MySqlExecutor<'_>,
    key: &str,
) -> Result<Option<TemplateRow>, AppError> {
    let sql = format!("SELECT {TEMPLATE_COLUMNS} FROM proposals_templates WHERE template_key = ?");
    sqlx::query(sqlx::AssertSqlSafe(sql))
        .bind(key)
        .fetch_optional(executor)
        .await?
        .as_ref()
        .map(template)
        .transpose()
}

pub(crate) async fn default_template(pool: &MySqlPool) -> Result<Option<TemplateRow>, AppError> {
    let sql = format!(
        "SELECT {TEMPLATE_COLUMNS} FROM proposals_templates WHERE is_active = 1 ORDER BY is_default DESC, created_at, id LIMIT 1"
    );
    sqlx::query(sqlx::AssertSqlSafe(sql))
        .fetch_optional(pool)
        .await?
        .as_ref()
        .map(template)
        .transpose()
}

pub(crate) async fn count_templates(pool: &MySqlPool) -> Result<i64, AppError> {
    Ok(
        sqlx::query_scalar("SELECT COUNT(*) FROM proposals_templates")
            .fetch_one(pool)
            .await?,
    )
}

pub(crate) async fn clear_default(tx: &mut MySqlTx, except: Uuid) -> Result<(), AppError> {
    sqlx::query("UPDATE proposals_templates SET is_default = 0 WHERE id <> ? AND is_default = 1")
        .bind(except.to_string())
        .execute(&mut **tx)
        .await?;
    Ok(())
}

pub(crate) async fn insert_template(
    tx: &mut MySqlTx,
    user_id: Uuid,
    values: &TemplateValues,
) -> Result<Uuid, AppError> {
    let id = Uuid::now_v7();
    let now = Utc::now().naive_utc();
    sqlx::query(
        "INSERT INTO proposals_templates (id, template_key, name, description, body, is_active, is_default, created_by, updated_by, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(id.to_string())
    .bind(&values.key)
    .bind(&values.name)
    .bind(&values.description)
    .bind(&values.body)
    .bind(values.is_active)
    .bind(values.is_default)
    .bind(user_id.to_string())
    .bind(user_id.to_string())
    .bind(now)
    .bind(now)
    .execute(&mut **tx)
    .await
    .map_err(key_conflict)?;
    if values.is_default {
        clear_default(tx, id).await?;
    }
    Ok(id)
}

pub(crate) async fn update_template(
    tx: &mut MySqlTx,
    user_id: Uuid,
    id: Uuid,
    values: &TemplateValues,
) -> Result<(), AppError> {
    sqlx::query(
        "UPDATE proposals_templates SET name = ?, description = ?, body = ?, is_active = ?, is_default = ?, updated_by = ?, updated_at = ? WHERE id = ?",
    )
    .bind(&values.name)
    .bind(&values.description)
    .bind(&values.body)
    .bind(values.is_active)
    .bind(values.is_default)
    .bind(user_id.to_string())
    .bind(Utc::now().naive_utc())
    .bind(id.to_string())
    .execute(&mut **tx)
    .await?;
    if values.is_default {
        clear_default(tx, id).await?;
    }
    Ok(())
}

pub(crate) async fn delete_template(tx: &mut MySqlTx, id: Uuid) -> Result<(), AppError> {
    sqlx::query("DELETE FROM proposals_templates WHERE id = ?")
        .bind(id.to_string())
        .execute(&mut **tx)
        .await?;
    Ok(())
}

fn key_conflict(error: sqlx::Error) -> AppError {
    match &error {
        sqlx::Error::Database(database) if database.is_unique_violation() => {
            AppError::conflict_code(
                crate::errors::TEMPLATE_KEY_TAKEN,
                "a template with this key already exists",
            )
        }
        _ => error.into(),
    }
}

pub(crate) async fn next_number(pool: &MySqlPool) -> Result<u32, AppError> {
    let mut tx = pool.begin().await?;
    sqlx::query(
        "INSERT INTO proposals_counters (id, last_number) VALUES (1, 1) ON DUPLICATE KEY UPDATE last_number = last_number + 1",
    )
    .execute(&mut *tx)
    .await?;
    let number: u32 = sqlx::query_scalar("SELECT last_number FROM proposals_counters WHERE id = 1")
        .fetch_one(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(number)
}

pub(crate) async fn insert_document(
    tx: &mut MySqlTx,
    document: &DocumentRow,
    actor_type: &str,
) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO proposals_documents (id, deal_id, template_id, template_name, number, file_id, file_name, file_size, generated_by, generated_by_name, actor_type, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(document.id.to_string())
    .bind(document.deal_id.to_string())
    .bind(document.template_id.map(|id| id.to_string()))
    .bind(&document.template_name)
    .bind(document.number)
    .bind(document.file_id.to_string())
    .bind(&document.file_name)
    .bind(document.file_size)
    .bind(document.generated_by.map(|id| id.to_string()))
    .bind(&document.generated_by_name)
    .bind(actor_type)
    .bind(document.created_at.naive_utc())
    .execute(&mut **tx)
    .await?;
    Ok(())
}

pub(crate) async fn list_documents(
    pool: &MySqlPool,
    deal_id: Uuid,
) -> Result<Vec<DocumentRow>, AppError> {
    sqlx::query(
        "SELECT id, deal_id, template_id, template_name, number, file_id, file_name, file_size, generated_by, generated_by_name, created_at FROM proposals_documents WHERE deal_id = ? ORDER BY created_at DESC, number DESC LIMIT 200",
    )
    .bind(deal_id.to_string())
    .fetch_all(pool)
    .await?
    .iter()
    .map(document)
    .collect()
}
