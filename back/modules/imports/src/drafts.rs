use aphel_kernel::{Ctx, domain::access::Permission};
use aphel_sdk::{
    AppError,
    imports::{
        CreateImportCommand, ImportDrafts,
        dto::{DuplicateKeyDto, ImportJobResponse},
    },
};

pub(crate) struct Drafts;

#[async_trait::async_trait]
impl ImportDrafts for Drafts {
    fn create_permission(&self, ctx: &Ctx<'_>, record_type: &str) -> Option<Permission> {
        crate::create_permission(ctx, record_type)
    }
    async fn create(
        &self,
        ctx: &Ctx<'_>,
        command: CreateImportCommand,
    ) -> Result<ImportJobResponse, AppError> {
        Ok(crate::service::create(ctx, command).await?.into())
    }
    async fn configure(
        &self,
        ctx: &Ctx<'_>,
        id: &str,
        mapping: Vec<Option<String>>,
        preferred_duplicate_key: DuplicateKeyDto,
    ) -> Result<ImportJobResponse, AppError> {
        let job = crate::service::get(ctx, id).await?;
        let preferred: crate::DuplicateKey = preferred_duplicate_key.into();
        let definition = ctx.registry().require(&job.job.record_type)?;
        let duplicate_key = if preferred.supported_by(definition) {
            preferred
        } else {
            crate::ImportOptions::default_for_definition(Some(definition)).duplicate_key
        };
        Ok(crate::service::configure(
            ctx,
            id,
            crate::ConfigureImportCommand {
                mapping,
                options: crate::ImportOptions {
                    duplicate_key,
                    duplicate_strategy: crate::DuplicateStrategy::Skip,
                    create_missing_references: true,
                    distribution_rule_id: None,
                },
            },
        )
        .await?
        .into())
    }
}
