use crate::{AppError, Ctx};
use aphel_kernel::application::organization_admin::Listing;
use aphel_kernel::application::query::RecordQuery;
use aphel_kernel::application::shared::RecordRef;
use serde_json::Value;

#[async_trait::async_trait]
pub trait RecordListSource: Send + Sync {
    fn record_type(&self) -> &'static str;
    async fn list(&self, ctx: &Ctx<'_>, query: RecordQuery) -> Result<Listing<Value>, AppError>;
}

#[async_trait::async_trait]
pub trait RelatedRecordSource: Send + Sync {
    fn record_type(&self) -> &'static str;
    async fn related(
        &self,
        ctx: &Ctx<'_>,
        parent: &RecordRef,
        limit: u64,
    ) -> Result<Vec<Value>, AppError>;
}

pub async fn list(
    ctx: &Ctx<'_>,
    record_type: &str,
    query: RecordQuery,
) -> Result<Listing<Value>, AppError> {
    bound(query.window.limit)?;
    let sources = ctx.app().all_contributions::<dyn RecordListSource>();
    let mut matching = sources.into_iter().filter(|(owner, source)| {
        ctx.app().is_enabled(owner) && source.record_type() == record_type
    });
    let Some((_, source)) = matching.next() else {
        if ctx.registry().require(record_type).is_err() {
            return Ok(Listing {
                items: Vec::new(),
                total: 0,
            });
        }
        return ctx.list_records(record_type, query).await;
    };
    if matching.next().is_some() {
        return Err(AppError::Configuration(format!(
            "multiple record listing sources for {record_type}"
        )));
    }
    source.list(ctx, query).await
}

pub async fn related(
    ctx: &Ctx<'_>,
    record_type: &str,
    parent: &RecordRef,
    limit: u64,
) -> Result<Vec<Value>, AppError> {
    bound(limit)?;
    let sources = ctx.app().all_contributions::<dyn RelatedRecordSource>();
    let mut matching = sources.into_iter().filter(|(owner, source)| {
        ctx.app().is_enabled(owner) && source.record_type() == record_type
    });
    let Some((_, source)) = matching.next() else {
        return Ok(Vec::new());
    };
    if matching.next().is_some() {
        return Err(AppError::Configuration(format!(
            "multiple related record sources for {record_type}"
        )));
    }
    source.related(ctx, parent, limit).await
}

fn bound(limit: u64) -> Result<(), AppError> {
    if !(1..=1000).contains(&limit) {
        return Err(AppError::Validation(
            "record context limit must be between 1 and 1000".into(),
        ));
    }
    Ok(())
}
