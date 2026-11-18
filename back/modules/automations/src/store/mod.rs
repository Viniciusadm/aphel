pub(crate) mod points;
pub(crate) mod timeline;

pub(crate) mod automations;
pub(crate) mod locks;
pub(crate) mod runs;
pub(crate) mod time;

pub(crate) use automations::AutomationStore;
pub(crate) use locks::AutomationLocks;
pub(crate) use runs::{AutomationRunStore, RunInsert};
pub(crate) use time::{AutomationTimeStore, CancelTarget, TimeCursor};

pub(crate) struct EventOrigin {
    pub actor_type: String,
    pub actor_id: Option<String>,
    pub execution: Option<serde_json::Value>,
}

pub(crate) async fn event_origin(
    pool: &aphel_sdk::platform::database::Db,
    event_id: uuid::Uuid,
) -> Result<Option<EventOrigin>, aphel_kernel::errors::AppError> {
    use sqlx::Row;
    let row = sqlx::query("SELECT actor_type, actor_id, execution FROM domain_events WHERE id = ?")
        .bind(event_id.to_string())
        .fetch_optional(pool)
        .await?;
    row.map(|row| {
        let execution: Option<sqlx::types::Json<serde_json::Value>> = row.try_get("execution")?;
        Ok(EventOrigin {
            actor_type: row.try_get("actor_type")?,
            actor_id: row.try_get("actor_id")?,
            execution: execution.map(|json| json.0),
        })
    })
    .transpose()
}
