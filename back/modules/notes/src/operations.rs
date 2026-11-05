use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::CreateNoteCommand;
use aphel_contract_automation::application::NoteOperations;
use aphel_kernel::Ctx;
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::application::shared::RecordRef;

pub(crate) struct AutomationNotes;

#[async_trait]
impl NoteOperations for AutomationNotes {
    async fn create_note(
        &self,
        ctx: &Ctx<'_>,
        record: &RecordRef,
        body: &str,
    ) -> Result<Uuid, AppError> {
        Ok(crate::service::create(
            ctx,
            CreateNoteCommand {
                record_type: record.record_type.as_str().to_owned(),
                record_id: record.record_id.to_string(),
                body: body.to_owned(),
                visibility: None,
                pinned: None,
                ip: None,
            },
        )
        .await?
        .id)
    }
}
