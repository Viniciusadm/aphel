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
    ctx: &aphel_kernel::Ctx<'_>,
    points: i64,
    idle_since: DateTime<Utc>,
) -> Result<u64, AppError> {
    let record_type = aphel_sdk::shared::RecordType::parse(aphel_sdk::people::RECORD_TYPE)?;
    if !aphel_sdk::records::enabled(ctx, &record_type) {
        return Ok(0);
    }
    let Some(definition) = ctx.registry().get(&record_type) else {
        return Ok(0);
    };
    let mut after = String::new();
    let mut cooled = 0;
    loop {
        let ids: Vec<String> = sqlx::query_scalar("SELECT record_id FROM lead_pro_contact_extension e WHERE record_id > ? AND lead_temperature > 0 AND NOT EXISTS (SELECT 1 FROM lead_pro_touches t WHERE t.contact_id = e.record_id AND t.created_at >= ?) ORDER BY record_id LIMIT 200")
            .bind(&after).bind(idle_since.naive_utc()).fetch_all(ctx.pool()).await?;
        if ids.is_empty() {
            break;
        }
        after = ids.last().cloned().unwrap_or_default();
        let mut tx = ctx.begin().await?;
        let mut live = Vec::new();
        for id in &ids {
            let id = Uuid::parse_str(id)
                .map_err(|_| AppError::Internal("invalid lead temperature identity".into()))?;
            if aphel_sdk::records::lock_entity_version(&mut tx, definition, id)
                .await?
                .is_some()
            {
                live.push(id);
            }
        }
        if !live.is_empty() {
            let mut query = sqlx::QueryBuilder::<sqlx::MySql>::new(
                "UPDATE lead_pro_contact_extension e SET lead_temperature = GREATEST(CAST(lead_temperature AS SIGNED) - ",
            );
            query
                .push_bind(points)
                .push(", 0) WHERE lead_temperature > 0 AND record_id IN (");
            let mut separated = query.separated(", ");
            for id in &live {
                separated.push_bind(id.to_string());
            }
            query.push(") AND NOT EXISTS (SELECT 1 FROM lead_pro_touches t WHERE t.contact_id = e.record_id AND t.created_at >= ").push_bind(idle_since.naive_utc()).push(")");
            cooled += query.build().execute(&mut *tx).await?.rows_affected();
        }
        tx.commit().await?;
        if ids.len() < 200 {
            break;
        }
    }
    Ok(cooled)
}
