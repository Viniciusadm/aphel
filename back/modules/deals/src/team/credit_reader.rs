use aphel_kernel::{Ctx, errors::AppError};
use aphel_sdk::deals::{CreditTotals, CreditedUsers, DealCreditReader};
use chrono::NaiveDate;

pub(crate) struct CreditReader;

#[async_trait::async_trait]
impl DealCreditReader for CreditReader {
    async fn won_total(
        &self,
        ctx: &Ctx<'_>,
        credited: &CreditedUsers,
        amount_key: &str,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Result<CreditTotals, AppError> {
        super::service::credited_won_total(ctx, credited, amount_key, from, to).await
    }
}
