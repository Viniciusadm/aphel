use async_trait::async_trait;
use uuid::Uuid;

use crate::Ctx;
use crate::application::automations::TaskSpec;
use crate::application::shared::RecordRef;
use crate::errors::AppError;

#[async_trait]
pub trait StageOperations: Send + Sync {
    async fn move_to_stage(
        &self,
        ctx: &Ctx<'_>,
        record: &RecordRef,
        stage_id: &str,
    ) -> Result<(), AppError>;
}

#[async_trait]
pub trait NoteOperations: Send + Sync {
    async fn create_note(
        &self,
        ctx: &Ctx<'_>,
        record: &RecordRef,
        body: &str,
    ) -> Result<Uuid, AppError>;
}

#[async_trait]
pub trait ActivityTypeOperations: Send + Sync {}

#[async_trait]
pub trait TaskOperations: Send + Sync {
    async fn create_task(&self, ctx: &Ctx<'_>, task: TaskSpec) -> Result<Uuid, AppError>;
}

#[async_trait]
pub trait IncomingWebhookOperations: Send + Sync {}
