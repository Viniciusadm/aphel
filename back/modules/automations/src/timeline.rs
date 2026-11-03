use aphel_kernel::{application::shared::RecordRef, domain::access::AccessScope, errors::AppError};
use aphel_sdk::timeline::{TimelineEntry, TimelineEventExtension, TimelineRequest};
use serde_json::{Value, json};
use sqlx::{MySql, MySqlPool, QueryBuilder, Row};
pub(crate) struct AutomationActors;
#[async_trait::async_trait]
impl TimelineEventExtension for AutomationActors {
    fn restrict(
        &self,
        _builder: &mut QueryBuilder<MySql>,
        _scope: &AccessScope,
        _subject: &RecordRef,
        _request: &TimelineRequest,
    ) {
    }
    async fn enrich(
        &self,
        pool: &MySqlPool,
        _scope: &AccessScope,
        entries: &mut [TimelineEntry],
    ) -> Result<(), AppError> {
        let ids: Vec<String> = entries
            .iter()
            .filter_map(|entry| {
                let actor = entry.metadata.get("service_actor")?;
                (actor.get("kind")?.as_str()? == "automation")
                    .then(|| actor.get("id").and_then(Value::as_str).map(str::to_owned))
                    .flatten()
            })
            .collect();
        if ids.is_empty() {
            return Ok(());
        }
        let mut builder =
            QueryBuilder::<MySql>::new("SELECT id, name FROM automations WHERE id IN (");
        let mut values = builder.separated(", ");
        for id in ids {
            values.push_bind(id);
        }
        builder.push(")");
        let rows = builder.build().fetch_all(pool).await?;
        let names: std::collections::HashMap<String, String> = rows
            .iter()
            .map(|row| Ok((row.try_get("id")?, row.try_get("name")?)))
            .collect::<Result<_, sqlx::Error>>()?;
        for entry in entries {
            if let Some(actor) = entry.metadata.get("service_actor")
                && actor.get("kind").and_then(Value::as_str) == Some("automation")
                && let Some(id) = actor.get("id").and_then(Value::as_str)
                && let Some(name) = names.get(id)
            {
                entry.metadata["automation"] = json!({ "id":id, "name":name });
            }
        }
        Ok(())
    }
}
