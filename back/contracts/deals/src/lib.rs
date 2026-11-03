use chrono::NaiveDate;
use rust_decimal::Decimal;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DealRevenue {
    pub amount_base: Option<Decimal>,
    pub exchange_rate: Option<Decimal>,
    pub mrr: Option<Decimal>,
    pub arr: Option<Decimal>,
    pub tcv: Option<Decimal>,
    pub contract_start: Option<NaiveDate>,
    pub contract_end: Option<NaiveDate>,
    pub renewal_of_deal_id: Option<Uuid>,
}
