use crate::http::DealResponse;
use aphel_kernel::{Ctx, errors::AppError};
use aphel_sdk::platform::application::organization_admin::Listing;
use aphel_sdk::platform::application::query::RecordQuery;
use aphel_sdk::platform::application::shared::RecordType;
use aphel_sdk::record_context::RecordListSource;
use serde_json::Value;
pub(crate) struct ContextSource;
#[async_trait::async_trait]
impl RecordListSource for ContextSource {
    fn record_type(&self) -> &'static str {
        "deals.deal"
    }
    async fn list(&self, ctx: &Ctx<'_>, query: RecordQuery) -> Result<Listing<Value>, AppError> {
        let listing = crate::service::list(ctx, query).await?;
        let items: Vec<DealResponse> = listing.items.into_iter().map(Into::into).collect();
        Ok(Listing {
            items: ctx
                .redacted_items(ctx.scope, &RecordType::from_static("deals.deal"), items)
                .await?,
            total: listing.total,
        })
    }
}
