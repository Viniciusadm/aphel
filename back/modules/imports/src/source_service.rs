use aphel_kernel::Ctx;
use aphel_sdk::{
    AppError,
    imports::{ImportSources, ParsedSource},
};
use uuid::Uuid;

pub(crate) struct Sources;

#[async_trait::async_trait]
impl ImportSources for Sources {
    fn parse(&self, bytes: &[u8]) -> Result<ParsedSource, AppError> {
        crate::parse_source(bytes)
    }
    async fn record_by_label(
        &self,
        ctx: &Ctx<'_>,
        record_type: &str,
        label: &str,
    ) -> Result<Option<Uuid>, AppError> {
        let Ok(definition) = ctx.registry().require(record_type) else {
            return Ok(None);
        };
        if !aphel_sdk::records::enabled(ctx, &definition.record_type) {
            return Ok(None);
        }
        crate::store::RecordLookups::of(ctx)
            .find_record_by_label(ctx.scope, record_type, label)
            .await
    }
    async fn member_by_email(
        &self,
        ctx: &Ctx<'_>,
        email: &str,
    ) -> Result<Option<String>, AppError> {
        Ok(crate::store::RecordLookups::of(ctx)
            .find_member_by_email(email)
            .await?
            .map(|id| id.as_str().to_owned()))
    }
}
