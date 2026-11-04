use crate::domain::ListActivitiesQuery;
use crate::http::ActivityResponse;
use aphel_kernel::application::organization_admin::ListWindow;
use aphel_kernel::application::query::RecordQuery;
use aphel_kernel::application::shared::{RecordRef, RecordType};
use aphel_kernel::{Ctx, errors::AppError};
use aphel_sdk::record_context::RelatedRecordSource;
use serde_json::Value;
pub(crate) struct ContextSource;
#[async_trait::async_trait]
impl RelatedRecordSource for ContextSource {
    fn record_type(&self) -> &'static str {
        "tasks.task"
    }
    async fn related(
        &self,
        ctx: &Ctx<'_>,
        parent: &RecordRef,
        limit: u64,
    ) -> Result<Vec<Value>, AppError> {
        let listing = crate::service::list(
            ctx,
            ListActivitiesQuery {
                record_type: Some(parent.record_type.as_str().to_owned()),
                record_id: Some(parent.record_id.to_string()),
                my_open: false,
                query: RecordQuery {
                    window: ListWindow { offset: 0, limit },
                    ..RecordQuery::default()
                },
            },
        )
        .await?;
        let items: Vec<ActivityResponse> = listing.items.into_iter().map(Into::into).collect();
        ctx.redacted_items(ctx.scope, &RecordType::from_static("tasks.task"), items)
            .await
    }
}
