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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DealParties {
    pub contact_id: Option<Uuid>,
    pub company_id: Option<Uuid>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DealOutcomes {
    pub open: u64,
    pub won: u64,
    pub lost: u64,
}

pub const DEAL_CREATED: &str = "deal.created";
pub const DEAL_REOPENED: &str = "deal.reopened";
pub const DEAL_WON: &str = "deal.won";
pub const DEAL_LOST: &str = "deal.lost";
pub const DEAL_DELETED: &str = "deal.deleted";

pub const KEY: &str = "deals";
pub const RECORD_TYPE: &str = "deals.deal";
pub const DESCRIPTOR: aphel_contract::ContractDescriptor = aphel_contract::ContractDescriptor {
    contract: aphel_contract::contract!("deals", "1.0.0"),
    entries: &[aphel_contract::KeyEntry::new(
        RECORD_TYPE,
        aphel_contract::KeyKind::Reference,
        "A deal record",
    )],
};
