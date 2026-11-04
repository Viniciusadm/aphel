use crate::{AppError, Ctx};
pub use aphel_contract_sequences::*;
use aphel_kernel::domain::UserId;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnrollCommand {
    pub sequence_id: String,
    pub contact_id: String,
    pub mailbox_id: Option<String>,
    pub source: EnrollmentSource,
    pub enroll_as: Option<UserId>,
    pub ip: Option<String>,
}

#[async_trait::async_trait]
pub trait SequenceEnrollment: Send + Sync {
    async fn enroll(&self, ctx: &Ctx<'_>, command: EnrollCommand) -> Result<Uuid, AppError>;
}

pub async fn enroll(ctx: &Ctx<'_>, command: EnrollCommand) -> Result<Uuid, AppError> {
    let service = ctx
        .services()
        .get::<dyn SequenceEnrollment>()
        .ok_or_else(unavailable)?;
    service.enroll(ctx, command).await
}

pub fn unavailable() -> AppError {
    AppError::unavailable_code(
        "sequence_enrollment_unavailable",
        "sequence enrollment is not enabled",
    )
}
