use aphel_sdk::records::RecordIncludeSource;
use aphel_sdk::{AppError, Ctx};
use serde_json::Value;
use std::collections::HashMap;
pub(crate) struct Tags;
#[async_trait::async_trait]
impl RecordIncludeSource for Tags {
    fn key(&self) -> &'static str {
        "tags"
    }
    fn supports(&self, ctx: &Ctx<'_>, record_type: &str) -> bool {
        ctx.registry().require(record_type).is_ok_and(|definition| {
            definition
                .capabilities
                .has_trait(aphel_sdk::tagging::TAGGABLE)
                || aphel_sdk::platform::domain::custom_objects::is_custom_record_type(record_type)
        })
    }
    async fn load(
        &self,
        ctx: &Ctx<'_>,
        record_type: &str,
        ids: &[String],
    ) -> Result<HashMap<String, Value>, AppError> {
        let mut result = HashMap::new();
        for chunk in ids.chunks(aphel_sdk::platform::application::query::MAX_LIST_VALUES) {
            let mut tags = crate::service::tags_for_records(ctx, record_type, chunk).await?;
            for id in chunk {
                let key = uuid::Uuid::parse_str(id)
                    .map_err(|_| AppError::Validation("invalid included record id".to_owned()))?;
                let summaries = tags
                    .remove(&key)
                    .unwrap_or_default()
                    .into_iter()
                    .map(crate::dto::TagSummary::from)
                    .collect::<Vec<_>>();
                result.insert(
                    id.clone(),
                    serde_json::to_value(summaries)
                        .map_err(|error| AppError::Internal(error.to_string()))?,
                );
            }
        }
        Ok(result)
    }
}
