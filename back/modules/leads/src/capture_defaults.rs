use crate::domain::{LEAD_STAGE, LIFECYCLE_STAGE_FIELD};
use aphel_sdk::{AppError, Ctx, party::PartyCaptureDefaults, shared::RecordType};
use serde_json::{Map, Value, json};
pub(crate) struct CaptureDefaults;
#[async_trait::async_trait]
impl PartyCaptureDefaults for CaptureDefaults {
    async fn values(
        &self,
        ctx: &Ctx<'_>,
        record_type: &RecordType,
    ) -> Result<Map<String, Value>, AppError> {
        let mut values = Map::new();
        if record_type.as_str() != aphel_sdk::people::RECORD_TYPE {
            return Ok(values);
        }
        if crate::store::LeadRecords::of(ctx)
            .stage_options(record_type)
            .await?
            .iter()
            .any(|option| option.is_active && option.value == LEAD_STAGE)
        {
            values.insert(LIFECYCLE_STAGE_FIELD.into(), json!(LEAD_STAGE));
        }
        Ok(values)
    }
}
