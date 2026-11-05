use crate::domain::ListNotesQuery;
use crate::http::NoteResponse;
use aphel_kernel::{Ctx, errors::AppError};
use aphel_sdk::platform::application::organization_admin::ListWindow;
use aphel_sdk::platform::application::shared::RecordRef;
use aphel_sdk::record_context::RelatedRecordSource;
use serde_json::Value;
pub(crate) struct ContextSource;
#[async_trait::async_trait]
impl RelatedRecordSource for ContextSource {
    fn record_type(&self) -> &'static str {
        "notes.note"
    }
    async fn related(
        &self,
        ctx: &Ctx<'_>,
        parent: &RecordRef,
        limit: u64,
    ) -> Result<Vec<Value>, AppError> {
        let listing = crate::service::list(
            ctx,
            ListNotesQuery {
                record_type: parent.record_type.as_str().to_owned(),
                record_id: parent.record_id.to_string(),
            },
            ListWindow { offset: 0, limit },
        )
        .await?;
        listing
            .items
            .into_iter()
            .map(|note| {
                serde_json::to_value(NoteResponse::from(note))
                    .map_err(|error| AppError::Internal(error.to_string()))
            })
            .collect()
    }
}
