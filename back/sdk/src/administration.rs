use crate::{AppError, Ctx};
use aphel_kernel::records::Patch;
use chrono::{DateTime, Utc};
use serde_json::{Map, Value};
use uuid::Uuid;
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SetCurrenciesCommand {
    pub currencies: Vec<String>,
    pub ip: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CreateExchangeRateCommand {
    pub currency: String,
    pub effective_date: Option<String>,
    pub rate: String,
    pub ip: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SavePriceListCommand {
    pub key: Option<String>,
    pub name: Option<String>,
    pub description: Patch<String>,
    pub currency: Option<String>,
    pub applies_to: Option<String>,
    pub segment_id: Patch<String>,
    pub company_ids: Option<Vec<String>>,
    pub valid_from: Patch<String>,
    pub valid_to: Patch<String>,
    pub is_active: Option<bool>,
    pub is_default: Option<bool>,
    pub expected_updated_at: Option<DateTime<Utc>>,
    pub ip: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PriceListItemInput {
    pub product_id: String,
    pub unit_price: String,
    pub min_quantity: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ReplacePriceListItemsCommand {
    pub items: Vec<PriceListItemInput>,
    pub expected_updated_at: Option<DateTime<Utc>>,
    pub ip: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StepInput {
    pub name: Option<String>,
    pub approver_kind: String,
    pub approver_user_id: Option<String>,
    pub approver_role_id: Option<String>,
    pub rule: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PolicyCommand {
    pub key: Option<String>,
    pub name: Option<String>,
    pub description: Option<Option<String>>,
    pub record_type: Option<String>,
    pub moment: Option<String>,
    pub target_id: Option<Option<String>>,
    pub condition_kind: Option<String>,
    pub condition_filter: Option<Value>,
    pub condition_formula: Option<Option<String>>,
    pub locked_fields: Option<Vec<String>>,
    pub due_hours: Option<Option<i64>>,
    pub is_active: Option<bool>,
    pub steps: Option<Vec<StepInput>>,
    pub ip: Option<String>,
    pub expected_updated_at: Option<DateTime<Utc>>,
}
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MoveDealCommand {
    pub stage_id: String,
    pub lost_reason_id: Option<String>,
    pub lost_reason_note: Option<String>,
    pub fields: Map<String, Value>,
    pub ip: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LoseDealCommand {
    pub lost_reason_id: Option<String>,
    pub lost_reason_note: Option<String>,
    pub ip: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AddDealMemberCommand {
    pub user_id: String,
    pub role_id: Option<String>,
    pub credit_percent: Option<String>,
    pub ip: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CreateValidationRuleCommand {
    pub record_type: String,
    pub key: Option<String>,
    pub name: String,
    pub expression: String,
    pub message: String,
    pub field: Option<String>,
    pub applies_on: Option<String>,
    pub is_active: Option<bool>,
    pub ip: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CompleteOnboardingCommand {
    pub template_key: Option<String>,
    pub install_id: Option<String>,
    pub ip: Option<String>,
}

pub struct CurrencyConfiguration {
    pub base: String,
    pub currencies: Vec<CurrencyEntry>,
}
pub struct CurrencyEntry {
    pub code: String,
    pub is_base: bool,
}
#[async_trait::async_trait]
pub trait MoneyAdministration: Send + Sync {
    async fn settings(&self, ctx: &Ctx<'_>) -> Result<CurrencyConfiguration, AppError>;
    async fn set_currencies(
        &self,
        ctx: &Ctx<'_>,
        command: SetCurrenciesCommand,
    ) -> Result<(), AppError>;
    async fn create_rate(
        &self,
        ctx: &Ctx<'_>,
        command: CreateExchangeRateCommand,
    ) -> Result<Uuid, AppError>;
}
#[async_trait::async_trait]
pub trait PricingAdministration: Send + Sync {
    async fn create(&self, ctx: &Ctx<'_>, command: SavePriceListCommand) -> Result<Uuid, AppError>;
    async fn replace_items(
        &self,
        ctx: &Ctx<'_>,
        id: &str,
        command: ReplacePriceListItemsCommand,
    ) -> Result<(), AppError>;
}
#[async_trait::async_trait]
pub trait ApprovalAdministration: Send + Sync {
    async fn create_policy(&self, ctx: &Ctx<'_>, command: PolicyCommand) -> Result<Uuid, AppError>;
}
#[async_trait::async_trait]
pub trait ValidationAdministration: Send + Sync {
    async fn create(
        &self,
        ctx: &Ctx<'_>,
        command: CreateValidationRuleCommand,
    ) -> Result<Uuid, AppError>;
}
#[async_trait::async_trait]
pub trait OnboardingCompletion: Send + Sync {
    async fn complete(
        &self,
        ctx: &Ctx<'_>,
        command: CompleteOnboardingCommand,
    ) -> Result<(), AppError>;
}
#[async_trait::async_trait]
pub trait StageAdministration: Send + Sync {
    async fn set_rotting_if_unset(
        &self,
        ctx: &Ctx<'_>,
        stage_id: &str,
        days: i64,
    ) -> Result<(), AppError>;
}
pub struct QuoteDraft {
    pub deal_id: String,
    pub valid_until: Option<String>,
    pub terms: Option<String>,
}
#[async_trait::async_trait]
pub trait QuoteCreation: Send + Sync {
    async fn create_from_line_items(
        &self,
        ctx: &Ctx<'_>,
        draft: QuoteDraft,
    ) -> Result<Uuid, AppError>;
}
#[async_trait::async_trait]
pub trait DealActions: Send + Sync {
    async fn move_to_stage(
        &self,
        ctx: &Ctx<'_>,
        id: &str,
        command: MoveDealCommand,
    ) -> Result<(), AppError>;
    async fn win(&self, ctx: &Ctx<'_>, id: &str, ip: Option<String>) -> Result<(), AppError>;
    async fn lose(&self, ctx: &Ctx<'_>, id: &str, command: LoseDealCommand)
    -> Result<(), AppError>;
    async fn member_roles(&self, ctx: &Ctx<'_>, active_only: bool) -> Result<Vec<Uuid>, AppError>;
    async fn add_member(
        &self,
        ctx: &Ctx<'_>,
        id: &str,
        command: AddDealMemberCommand,
    ) -> Result<(), AppError>;
}
