use aphel_sdk::{
    AppError, Ctx,
    notes::{CreateNoteCommand, NoteWriter},
};
use uuid::Uuid;

pub(crate) struct Writer;

#[async_trait::async_trait]
impl NoteWriter for Writer {
    async fn create(&self, ctx: &Ctx<'_>, command: CreateNoteCommand) -> Result<Uuid, AppError> {
        Ok(crate::service::create(ctx, command).await?.id)
    }
}
