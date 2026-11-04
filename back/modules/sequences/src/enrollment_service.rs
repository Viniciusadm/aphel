use aphel_kernel::{Ctx, errors::AppError};
use aphel_sdk::sequences::{EnrollCommand, SequenceEnrollment};
use uuid::Uuid;
pub(crate) struct Enrollment;
#[async_trait::async_trait]
impl SequenceEnrollment for Enrollment {
    async fn enroll(&self, ctx: &Ctx<'_>, command: EnrollCommand) -> Result<Uuid, AppError> {
        Ok(crate::service::enroll(ctx, command).await?.id)
    }
}
