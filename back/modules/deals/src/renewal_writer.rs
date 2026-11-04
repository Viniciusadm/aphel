use crate::domain::DealInput;
use aphel_kernel::application::shared::money_text;
use aphel_sdk::{
    AppError, Ctx,
    deals::{DealRevenue, RenewalDealCreation, RenewalDealWriter, date},
};
use uuid::Uuid;
pub(crate) struct Writer;
#[async_trait::async_trait]
impl RenewalDealWriter for Writer {
    async fn create(&self, ctx: &Ctx<'_>, creation: RenewalDealCreation) -> Result<Uuid, AppError> {
        let record = crate::service::create_with_revenue(
            ctx,
            DealInput {
                title: Some(creation.title),
                amount: Some(creation.amount.map(money_text)),
                currency: Some(creation.currency),
                pipeline_id: Some(creation.pipeline_id.to_string()),
                stage_id: Some(creation.stage_id.to_string()),
                expected_close_date: Some(creation.expected_close_date.map(date)),
                contact_id: Some(creation.contact_id.map(|id| id.to_string())),
                company_id: Some(creation.company_id.map(|id| id.to_string())),
                owner_id: Some(creation.owner_id),
                source: Some(creation.source),
                contract_start: Some(creation.contract_start.map(date)),
                ..DealInput::default()
            },
            DealRevenue {
                renewal_of_deal_id: Some(creation.original_id),
                ..DealRevenue::default()
            },
        )
        .await?;
        Ok(record.record.deal.id)
    }
}

#[async_trait::async_trait]
impl aphel_sdk::deals::DealRenewalReader for Writer {
    async fn due(
        &self,
        ctx: &Ctx<'_>,
        query: aphel_sdk::deals::RenewalDueQuery,
    ) -> Result<Vec<aphel_sdk::deals::RenewalCandidate>, AppError> {
        use aphel_kernel::application::query::{
            ArchivedFilter, Filter, FilterOperator, RecordQuery, SortSpec,
        };
        let until = query
            .today
            .checked_add_days(chrono::Days::new(u64::from(query.days_before)))
            .ok_or_else(|| AppError::Validation("renewal date is out of range".into()))?;
        let mut filters = vec![
            Filter::condition(
                "pipeline_id",
                FilterOperator::Eq,
                serde_json::json!(query.pipeline_id),
            ),
            Filter::condition("status", FilterOperator::Eq, serde_json::json!("won")),
            Filter::condition(
                "contract_end",
                FilterOperator::Gte,
                serde_json::json!(date(query.today)),
            ),
            Filter::condition(
                "contract_end",
                FilterOperator::Lte,
                serde_json::json!(date(until)),
            ),
        ];
        if let Some(after) = query.after {
            filters.push(Filter::or(vec![
                Filter::condition(
                    "contract_end",
                    FilterOperator::Gt,
                    serde_json::json!(date(after.contract_end)),
                ),
                Filter::and(vec![
                    Filter::condition(
                        "contract_end",
                        FilterOperator::Eq,
                        serde_json::json!(date(after.contract_end)),
                    ),
                    Filter::condition("id", FilterOperator::Gt, serde_json::json!(after.id)),
                ]),
            ]));
        }
        let rows = aphel_sdk::record_context::list(
            ctx,
            aphel_sdk::deals::RECORD_TYPE,
            RecordQuery {
                filter: Some(Filter::and(filters)),
                sort: vec![SortSpec::asc("contract_end"), SortSpec::asc("id")],
                archived: ArchivedFilter::Include,
                window: aphel_kernel::application::organization_admin::ListWindow {
                    offset: 0,
                    limit: 100,
                },
                ..RecordQuery::default()
            },
        )
        .await?;
        rows.items
            .into_iter()
            .map(|value| {
                let id = value
                    .get("id")
                    .and_then(|value| value.as_str())
                    .and_then(|value| Uuid::parse_str(value).ok())
                    .ok_or_else(|| AppError::Internal("deal snapshot has no identity".into()))?;
                let contract_end = value
                    .get("contract_end")
                    .and_then(|value| value.as_str())
                    .and_then(|value| chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d").ok())
                    .ok_or_else(|| {
                        AppError::Forbidden("deal contract end is not readable".into())
                    })?;
                Ok(aphel_sdk::deals::RenewalCandidate { id, contract_end })
            })
            .collect()
    }
    async fn existing(
        &self,
        ctx: &Ctx<'_>,
        original_id: Uuid,
    ) -> Result<Option<aphel_sdk::deals::ExistingDealRenewal>, AppError> {
        if ctx
            .load_record(aphel_sdk::deals::RECORD_TYPE, original_id)
            .await?
            .is_none()
        {
            return Ok(None);
        }
        let row: Option<(String, Option<chrono::NaiveDateTime>)> =
            sqlx::query_as("SELECT id, deleted_at FROM deals WHERE renewal_of_deal_id = ?")
                .bind(original_id.to_string())
                .fetch_optional(ctx.pool())
                .await?;
        row.map(|(id, deleted)| {
            Ok(aphel_sdk::deals::ExistingDealRenewal {
                deal_id: aphel_kernel::infrastructure::repositories::mapper::parse_uuid(
                    &id, "deals.id",
                )?,
                deleted: deleted.is_some(),
            })
        })
        .transpose()
    }
}
