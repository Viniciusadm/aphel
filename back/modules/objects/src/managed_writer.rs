use crate::application::CustomObjectAdminUseCases;
use crate::infrastructure::MySqlCustomObjectAdminPorts;
use aphel_kernel::{Ctx, errors::AppError};
use aphel_sdk::objects::{CreateObjectCommand, ManagedObject, ManagedObjectWriter};

pub(crate) struct Writer;

#[async_trait::async_trait]
impl ManagedObjectWriter for Writer {
    async fn create(
        &self,
        ctx: &Ctx<'_>,
        command: CreateObjectCommand,
        managed: ManagedObject,
    ) -> Result<(), AppError> {
        CustomObjectAdminUseCases::new(MySqlCustomObjectAdminPorts::new(
            ctx.pool().clone(),
            ctx.registry().clone(),
            ctx.events().clone(),
            ctx.audit().clone(),
        ))
        .create_managed(ctx.scope, command, managed)
        .await?;
        Ok(())
    }
}
