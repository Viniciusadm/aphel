use aphel_kernel::database::transaction::MySqlTx;
use aphel_sdk::{
    AppError, Ctx,
    people::{PersonCreation, PersonWriter},
    records::PendingCommit,
};
pub(crate) struct Creator;
#[async_trait::async_trait]
impl PersonWriter for Creator {
    async fn create_in_tx(
        &self,
        ctx: &Ctx<'_>,
        tx: &mut MySqlTx,
        creation: PersonCreation,
    ) -> Result<PendingCommit, AppError> {
        let created = crate::service::create_in_tx(
            ctx,
            tx,
            crate::domain::SaveContactCommand {
                input: crate::domain::ContactInput::from_values(&creation.values),
                custom_fields: creation.custom_fields,
                ip: creation.ip,
                expected_updated_at: None,
            },
        )
        .await?;
        Ok(created.into_pending())
    }
}
