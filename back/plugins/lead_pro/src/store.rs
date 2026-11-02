use chrono::{DateTime, Utc};
use sqlx::{MySqlConnection, MySqlPool, Row};
use uuid::Uuid;

use aphel_kernel::errors::AppError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Touch {
    pub(crate) id: Uuid,
    pub(crate) kind: String,
    pub(crate) created_at: DateTime<Utc>,
}

pub(crate) async fn temperature(
    connection: &mut MySqlConnection,
    contact_id: Uuid,
) -> Result<Option<i64>, AppError> {
    let value: Option<Option<i64>> =
        sqlx::query_scalar("SELECT CAST(lead_temperature AS SIGNED) FROM lead_pro_contact_extension WHERE record_id = ?")
            .bind(contact_id.to_string())
            .fetch_optional(connection)
            .await?;
    Ok(value.flatten())
}

pub(crate) async fn set_temperature(
    connection: &mut MySqlConnection,
    contact_id: Uuid,
    temperature: i64,
) -> Result<(), AppError> {
    sqlx::query("INSERT INTO lead_pro_contact_extension (lead_temperature, record_id) VALUES (?, ?) ON DUPLICATE KEY UPDATE lead_temperature = VALUES(lead_temperature)")
        .bind(temperature)
        .bind(contact_id.to_string())
        .execute(connection)
        .await?;
    Ok(())
}

pub(crate) async fn owner(
    connection: &mut MySqlConnection,
    contact_id: Uuid,
) -> Result<Option<String>, AppError> {
    let owner: Option<Option<String>> =
        sqlx::query_scalar("SELECT owner_id FROM contacts WHERE id = ? AND deleted_at IS NULL")
            .bind(contact_id.to_string())
            .fetch_optional(connection)
            .await?;
    Ok(owner.flatten())
}

pub(crate) async fn insert_touch(
    connection: &mut MySqlConnection,
    contact_id: Uuid,
    kind: &str,
    event_id: Option<Uuid>,
    now: DateTime<Utc>,
) -> Result<bool, AppError> {
    let inserted = sqlx::query(
        "INSERT IGNORE INTO lead_pro_touches (id, contact_id, kind, event_id, created_at) \
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(Uuid::now_v7().to_string())
    .bind(contact_id.to_string())
    .bind(kind)
    .bind(event_id.map(|id| id.to_string()))
    .bind(now.naive_utc())
    .execute(connection)
    .await?
    .rows_affected();
    Ok(inserted > 0)
}

pub(crate) async fn touches(
    pool: &MySqlPool,
    contact_id: Uuid,
    limit: u32,
) -> Result<Vec<Touch>, AppError> {
    let rows = sqlx::query(
        "SELECT id, kind, created_at FROM lead_pro_touches WHERE contact_id = ? \
         ORDER BY created_at DESC, id DESC LIMIT ?",
    )
    .bind(contact_id.to_string())
    .bind(limit)
    .fetch_all(pool)
    .await?;
    rows.iter()
        .map(|row| {
            let id: String = row.try_get("id")?;
            Ok(Touch {
                id: Uuid::parse_str(&id)
                    .map_err(|_| AppError::Internal(format!("invalid touch id {id}")))?,
                kind: row.try_get("kind")?,
                created_at: row
                    .try_get::<chrono::NaiveDateTime, _>("created_at")?
                    .and_utc(),
            })
        })
        .collect()
}

pub(crate) async fn cool_down(
    pool: &MySqlPool,
    points: i64,
    idle_since: DateTime<Utc>,
) -> Result<u64, AppError> {
    Ok(sqlx::query(
        "UPDATE lead_pro_contact_extension e JOIN contacts c ON c.id = e.record_id SET e.lead_temperature = GREATEST(CAST(e.lead_temperature AS SIGNED) - ?, 0) \
         WHERE e.lead_temperature > 0 AND c.deleted_at IS NULL AND NOT EXISTS \
         (SELECT 1 FROM lead_pro_touches t WHERE t.contact_id = c.id AND t.created_at >= ?)",
    )
    .bind(points)
    .bind(idle_since.naive_utc())
    .execute(pool)
    .await?
    .rows_affected())
}
