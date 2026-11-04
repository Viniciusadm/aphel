use crate::{AppError, Ctx};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CreateNoteCommand {
    pub record_type: String,
    pub record_id: String,
    pub body: String,
    pub visibility: Option<String>,
    pub pinned: Option<bool>,
    pub ip: Option<String>,
}

#[async_trait::async_trait]
pub trait NoteWriter: Send + Sync {
    async fn create(&self, ctx: &Ctx<'_>, command: CreateNoteCommand) -> Result<Uuid, AppError>;
}

pub fn writer() -> Result<std::sync::Arc<dyn NoteWriter>, AppError> {
    aphel_kernel::modules::app()
        .services()
        .get::<dyn NoteWriter>()
        .ok_or_else(|| {
            AppError::unavailable_code("note_writer_unavailable", "note creation is not enabled")
        })
}
