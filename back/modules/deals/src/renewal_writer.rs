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
