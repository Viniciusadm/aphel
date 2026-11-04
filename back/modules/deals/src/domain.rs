use chrono::{DateTime, NaiveDate, SecondsFormat, Utc};
use rust_decimal::Decimal;
use serde_json::{Map, Value, json};
use uuid::Uuid;

use aphel_kernel::application::fields::decimal_json;
use aphel_kernel::application::fields::permissions::provided_key;
use aphel_kernel::application::query::RecordQuery;
use aphel_kernel::application::shared::FieldChange;
use aphel_kernel::domain::UserId;
use aphel_kernel::errors::{AppError, FieldViolation};
use aphel_sdk::workflow::{Pipeline, PipelineStage, StageKind};

pub(crate) use aphel_kernel::application::shared::normalize_currency_code as normalize_currency;

pub(crate) const DEAL_NOT_FOUND: &str = "deal_not_found";
pub(crate) const NO_DEFAULT_PIPELINE: &str = "no_default_pipeline";
pub(crate) const DEAL_AMOUNT_MANAGED_BY_ITEMS: &str = "deal_amount_managed_by_items";

pub use aphel_sdk::deals::DEAL_CREATED;
pub const DEAL_UPDATED: &str = "deal.updated";
pub(crate) const DEAL_STAGE_CHANGED: &str = "deal.stage_changed";
pub(crate) const DEAL_PIPELINE_CHANGED: &str = "deal.pipeline_changed";
pub const DEAL_AMOUNT_CHANGED: &str = "deal.amount_changed";
pub(crate) const DEAL_OWNER_CHANGED: &str = "deal.owner_changed";
pub(crate) const DEAL_EXPECTED_CLOSE_DATE_CHANGED: &str = "deal.expected_close_date_changed";
pub use aphel_sdk::deals::DEAL_DELETED;
pub use aphel_sdk::deals::DEAL_LOST;
pub use aphel_sdk::deals::DEAL_REOPENED;
pub use aphel_sdk::deals::DEAL_WON;
pub(crate) const DEAL_ARCHIVED: &str = "deal.archived";
pub(crate) const DEAL_UNARCHIVED: &str = "deal.unarchived";

pub(crate) const DEFAULT_BOARD_PER_COLUMN: u64 = 20;
pub(crate) const MAX_BOARD_PER_COLUMN: u64 = 100;

pub use aphel_sdk::deals::MAX_DEAL_TITLE_LENGTH;
pub(crate) const MAX_DEAL_SOURCE_LENGTH: usize = 140;
pub(crate) const MAX_LOST_REASON_NOTE_LENGTH: usize = 5000;
pub(crate) const MAX_AMOUNT_DIGITS: u32 = 16;
pub(crate) const AMOUNT_DECIMALS: u32 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DealStatus {
    Open,
    Won,
    Lost,
}

impl DealStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Won => "won",
            Self::Lost => "lost",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "open" => Some(Self::Open),
            "won" => Some(Self::Won),
            "lost" => Some(Self::Lost),
            _ => None,
        }
    }

    pub(crate) fn for_stage(kind: StageKind) -> Self {
        match kind {
            StageKind::Open => Self::Open,
            StageKind::Won => Self::Won,
            StageKind::Lost => Self::Lost,
        }
    }
}

pub use aphel_sdk::deals::DealRevenue;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Deal {
    pub id: Uuid,
    pub owner_id: Option<UserId>,
    pub pipeline_id: Uuid,
    pub stage_id: Uuid,
    pub title: String,
    pub amount: Option<Decimal>,
    pub currency: String,
    pub revenue: DealRevenue,
    pub probability: Option<u8>,
    pub expected_close_date: Option<NaiveDate>,
    pub status: DealStatus,
    pub closed_at: Option<DateTime<Utc>>,
    pub lost_reason_id: Option<Uuid>,
    pub lost_reason_note: Option<String>,
    pub source: Option<String>,
    pub contact_id: Option<Uuid>,
    pub company_id: Option<Uuid>,
    pub stage_entered_at: DateTime<Utc>,
    pub last_activity_at: Option<DateTime<Utc>>,
    pub created_by: Option<UserId>,
    pub archived_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LostDetails {
    pub(crate) reason_id: Uuid,
    pub(crate) note: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DealTransitionError {
    LostReasonRequired,
    StageInactive,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StageTransition {
    pub(crate) from_stage_id: Uuid,
    pub(crate) from_status: DealStatus,
    pub(crate) to_status: DealStatus,
    pub(crate) stage_changed: bool,
}

impl StageTransition {
    pub(crate) fn status_changed(&self) -> bool {
        self.from_status != self.to_status
    }

    pub(crate) fn is_noop(&self) -> bool {
        !self.stage_changed && !self.status_changed()
    }
}

impl Deal {
    pub(crate) fn enter_stage(
        &mut self,
        stage: &PipelineStage,
        lost: Option<LostDetails>,
        now: DateTime<Utc>,
    ) -> Result<StageTransition, DealTransitionError> {
        if !stage.is_active && stage.id != self.stage_id {
            return Err(DealTransitionError::StageInactive);
        }
        let to_status = DealStatus::for_stage(stage.kind);
        let transition = StageTransition {
            from_stage_id: self.stage_id,
            from_status: self.status,
            to_status,
            stage_changed: stage.id != self.stage_id,
        };
        if transition.is_noop() && to_status != DealStatus::Lost {
            return Ok(transition);
        }
        match to_status {
            DealStatus::Lost => {
                let lost = lost.ok_or(DealTransitionError::LostReasonRequired)?;
                if transition.is_noop()
                    && self.lost_reason_id == Some(lost.reason_id)
                    && self.lost_reason_note == lost.note
                {
                    return Ok(transition);
                }
                self.lost_reason_id = Some(lost.reason_id);
                self.lost_reason_note = lost.note;
                if transition.status_changed() {
                    self.closed_at = Some(now);
                }
            }
            DealStatus::Won => {
                self.lost_reason_id = None;
                self.lost_reason_note = None;
                if transition.status_changed() {
                    self.closed_at = Some(now);
                }
            }
            DealStatus::Open => {
                self.lost_reason_id = None;
                self.lost_reason_note = None;
                self.closed_at = None;
            }
        }
        if transition.stage_changed {
            self.stage_entered_at = now;
        }
        self.pipeline_id = stage.pipeline_id;
        self.stage_id = stage.id;
        self.status = to_status;
        self.updated_at = now;
        Ok(transition)
    }
}

pub(crate) fn normalize_title(value: &str) -> Option<String> {
    let title = value.trim();
    if title.is_empty() || title.chars().count() > MAX_DEAL_TITLE_LENGTH {
        return None;
    }
    Some(title.to_owned())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TextTooLong;

pub(crate) fn normalize_optional_text(
    value: Option<&str>,
    max: usize,
) -> Result<Option<String>, TextTooLong> {
    match value.map(str::trim) {
        None | Some("") => Ok(None),
        Some(text) if text.chars().count() > max => Err(TextTooLong),
        Some(text) => Ok(Some(text.to_owned())),
    }
}

pub(crate) fn normalize_amount(value: Decimal) -> Option<Decimal> {
    let amount = value.round_dp(AMOUNT_DECIMALS);
    let limit = Decimal::from(10_i64.pow(MAX_AMOUNT_DIGITS));
    (amount >= Decimal::ZERO && amount < limit).then_some(amount)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NamedRef {
    pub(crate) id: Uuid,
    pub(crate) name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OwnerRef {
    pub(crate) id: UserId,
    pub(crate) display_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageSummary {
    pub id: Uuid,
    pub name: String,
    pub kind: StageKind,
    pub probability: u8,
    pub rotting_days: Option<u16>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DealRecord {
    pub deal: Deal,
    pub(crate) stage: StageSummary,
    pub(crate) contact: Option<NamedRef>,
    pub(crate) company: Option<NamedRef>,
    pub(crate) owner: Option<OwnerRef>,
    pub(crate) lost_reason: Option<NamedRef>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StageTotal {
    pub(crate) stage_id: Uuid,
    pub(crate) count: u64,
    pub(crate) total_amount: Decimal,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct DealInput {
    pub title: Option<String>,
    pub amount: Option<Option<String>>,
    pub currency: Option<String>,
    pub pipeline_id: Option<String>,
    pub stage_id: Option<String>,
    pub status: Option<String>,
    pub probability: Option<Option<i64>>,
    pub expected_close_date: Option<Option<String>>,
    pub contact_id: Option<Option<String>>,
    pub company_id: Option<Option<String>>,
    pub owner_id: Option<Option<String>>,
    pub source: Option<Option<String>>,
    pub lost_reason_id: Option<Option<String>>,
    pub lost_reason_note: Option<Option<String>>,
    pub contract_start: Option<Option<String>>,
    pub contract_end: Option<Option<String>>,
    pub custom_fields: Map<String, Value>,
    pub contributed: Map<String, Value>,
    pub expected_updated_at: Option<DateTime<Utc>>,
    pub ip: Option<String>,
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
pub(crate) struct MoveDealPipelineCommand {
    pub(crate) pipeline_id: String,
    pub(crate) stage_id: Option<String>,
    pub(crate) lost_reason_id: Option<String>,
    pub(crate) lost_reason_note: Option<String>,
    pub(crate) fields: Map<String, Value>,
    pub(crate) ip: Option<String>,
}

fn optional_text(value: &Value) -> Option<String> {
    match value {
        Value::Null => None,
        Value::String(text) => Some(text.clone()),
        Value::Number(number) => Some(number.to_string()),
        other => Some(other.to_string()),
    }
}

fn optional_integer(value: &Value) -> Option<i64> {
    match value {
        Value::Null => None,
        Value::Number(number) => Some(
            number
                .as_i64()
                .or_else(|| {
                    number
                        .as_f64()
                        .filter(|value| value.fract() == 0.0)
                        .map(|value| value as i64)
                })
                .unwrap_or(-1),
        ),
        Value::String(text) if text.trim().is_empty() => None,
        Value::String(text) => Some(text.trim().parse().unwrap_or(-1)),
        _ => Some(-1),
    }
}

impl DealInput {
    pub(crate) fn with_record_values(mut self, values: &Map<String, Value>) -> Self {
        let mut fields = Map::new();
        for (key, value) in values {
            match key.as_str() {
                "pipeline_id" => self.pipeline_id = optional_text(value),
                "stage_id" => self.stage_id = optional_text(value),
                "status" => self.status = optional_text(value),
                "lost_reason_id" => self.lost_reason_id = Some(optional_text(value)),
                "lost_reason_note" => self.lost_reason_note = Some(optional_text(value)),
                "title"
                | "amount"
                | "currency"
                | "probability"
                | "expected_close_date"
                | "contact_id"
                | "company_id"
                | "owner_id"
                | "source"
                | "contract_start"
                | "contract_end" => {
                    fields.insert(key.clone(), value.clone());
                }
                _ => {}
            }
        }
        self.with_field_values(&fields)
    }

    pub(crate) fn with_field_values(mut self, fields: &Map<String, Value>) -> Self {
        for (key, value) in fields {
            match key.as_str() {
                "title" => self.title = optional_text(value),
                "amount" => self.amount = Some(optional_text(value)),
                "currency" => self.currency = optional_text(value),
                "probability" => self.probability = Some(optional_integer(value)),
                "expected_close_date" => self.expected_close_date = Some(optional_text(value)),
                "contact_id" => self.contact_id = Some(optional_text(value)),
                "company_id" => self.company_id = Some(optional_text(value)),
                "owner_id" => self.owner_id = Some(optional_text(value)),
                "source" => self.source = Some(optional_text(value)),
                "contract_start" => self.contract_start = Some(optional_text(value)),
                "contract_end" => self.contract_end = Some(optional_text(value)),
                _ => {
                    self.custom_fields.insert(key.clone(), value.clone());
                }
            }
        }
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LoseDealCommand {
    pub lost_reason_id: Option<String>,
    pub lost_reason_note: Option<String>,
    pub ip: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct ReopenDealCommand {
    pub(crate) stage_id: Option<String>,
    pub(crate) ip: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct BoardQuery {
    pub(crate) pipeline_id: Option<String>,
    pub(crate) stage_id: Option<String>,
    pub(crate) per_column: u64,
    pub(crate) page: u64,
    pub(crate) query: RecordQuery,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DealView {
    pub record: DealRecord,
    pub(crate) custom_fields: Map<String, Value>,
    pub(crate) contributed: Map<String, Value>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct BoardColumn {
    pub(crate) stage: PipelineStage,
    pub(crate) count: u64,
    pub(crate) total_amount: Decimal,
    pub(crate) items: Vec<DealView>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Board {
    pub(crate) pipeline: Pipeline,
    pub(crate) columns: Vec<BoardColumn>,
    pub(crate) page: u64,
    pub(crate) per_column: u64,
}

impl DealInput {
    pub(crate) fn provided_keys(&self) -> Vec<&'static str> {
        let mut keys = Vec::new();
        provided_key(&mut keys, "title", self.title.is_some());
        provided_key(&mut keys, "amount", self.amount.is_some());
        provided_key(&mut keys, "currency", self.currency.is_some());
        provided_key(&mut keys, "pipeline_id", self.pipeline_id.is_some());
        provided_key(&mut keys, "stage_id", self.stage_id.is_some());
        provided_key(&mut keys, "status", self.status.is_some());
        provided_key(&mut keys, "probability", self.probability.is_some());
        provided_key(
            &mut keys,
            "expected_close_date",
            self.expected_close_date.is_some(),
        );
        provided_key(&mut keys, "contact_id", self.contact_id.is_some());
        provided_key(&mut keys, "company_id", self.company_id.is_some());
        provided_key(&mut keys, "owner_id", self.owner_id.is_some());
        provided_key(&mut keys, "source", self.source.is_some());
        provided_key(&mut keys, "contract_start", self.contract_start.is_some());
        provided_key(&mut keys, "contract_end", self.contract_end.is_some());
        provided_key(
            &mut keys,
            "lost_reason_id",
            self.lost_reason_id.is_some() || self.lost_reason_note.is_some(),
        );
        keys
    }
}

pub(crate) fn instant(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(SecondsFormat::Secs, true)
}

pub use aphel_sdk::deals::date;

pub(crate) fn amount_json(value: Option<Decimal>) -> Value {
    value.map(decimal_json).unwrap_or(Value::Null)
}

fn uuid_json(value: Option<Uuid>) -> Value {
    value
        .map(|id| Value::String(id.to_string()))
        .unwrap_or(Value::Null)
}

pub(crate) fn deal_snapshot(deal: &Deal) -> Value {
    json!({
        "title": deal.title,
        "amount": amount_json(deal.amount),
        "currency": deal.currency,
        "amount_base": amount_json(deal.revenue.amount_base),
        "exchange_rate": amount_json(deal.revenue.exchange_rate),
        "mrr": amount_json(deal.revenue.mrr),
        "arr": amount_json(deal.revenue.arr),
        "tcv": amount_json(deal.revenue.tcv),
        "contract_start": deal.revenue.contract_start.map(date),
        "contract_end": deal.revenue.contract_end.map(date),
        "renewal_of_deal_id": uuid_json(deal.revenue.renewal_of_deal_id),
        "pipeline_id": deal.pipeline_id.to_string(),
        "stage_id": deal.stage_id.to_string(),
        "status": deal.status.as_str(),
        "probability": deal.probability,
        "expected_close_date": deal.expected_close_date.map(date),
        "contact_id": uuid_json(deal.contact_id),
        "company_id": uuid_json(deal.company_id),
        "owner_id": deal.owner_id.as_ref().map(UserId::as_str),
        "source": deal.source,
        "lost_reason_id": uuid_json(deal.lost_reason_id),
        "lost_reason_note": deal.lost_reason_note,
        "closed_at": deal.closed_at.map(instant),
        "archived_at": deal.archived_at.map(instant),
    })
}

pub fn deal_not_found() -> AppError {
    AppError::not_found_code(DEAL_NOT_FOUND, "deal was not found")
}

fn date_json(value: Option<NaiveDate>) -> Value {
    value.map(date).map(Value::String).unwrap_or(Value::Null)
}

fn revenue_values(revenue: &DealRevenue) -> [(&'static str, Value); 5] {
    [
        ("mrr", amount_json(revenue.mrr)),
        ("arr", amount_json(revenue.arr)),
        ("tcv", amount_json(revenue.tcv)),
        ("contract_start", date_json(revenue.contract_start)),
        ("contract_end", date_json(revenue.contract_end)),
    ]
}

pub fn revenue_changes(old: &DealRevenue, new: &DealRevenue) -> Vec<FieldChange> {
    revenue_values(old)
        .into_iter()
        .zip(revenue_values(new))
        .filter(|((_, old), (_, new))| old != new)
        .map(|((field, old), (_, new))| FieldChange::new(field, old, new))
        .collect()
}

fn parse_date(text: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(text.trim(), "%Y-%m-%d").ok()
}

pub(crate) fn apply_contract_input(
    revenue: &mut DealRevenue,
    contract_start: &Option<Option<String>>,
    contract_end: &Option<Option<String>>,
    violations: &mut Vec<FieldViolation>,
) {
    for (field, input, target) in [
        (
            "contract_start",
            contract_start,
            &mut revenue.contract_start,
        ),
        ("contract_end", contract_end, &mut revenue.contract_end),
    ] {
        let Some(value) = input else {
            continue;
        };
        match value.as_deref().map(str::trim) {
            None | Some("") => *target = None,
            Some(text) => match parse_date(text) {
                Some(value) => *target = Some(value),
                None => violations.push(FieldViolation::new(
                    field,
                    "must be a date in the YYYY-MM-DD format",
                )),
            },
        }
    }
    if (contract_start.is_some() || contract_end.is_some())
        && let (Some(start), Some(end)) = (revenue.contract_start, revenue.contract_end)
        && end < start
    {
        violations.push(FieldViolation::new(
            "contract_end",
            "must be on or after contract_start",
        ));
    }
}

pub(crate) fn needs_contract_refresh(
    previous: Option<&Deal>,
    deal: &Deal,
    contract_end_provided: bool,
) -> bool {
    let became_won = deal.status == DealStatus::Won
        && previous.is_none_or(|previous| previous.status != DealStatus::Won);
    let start_changed = previous
        .is_some_and(|previous| previous.revenue.contract_start != deal.revenue.contract_start);
    previous.is_some() && (became_won || (start_changed && !contract_end_provided))
}

pub(crate) fn transition_violation(error: DealTransitionError) -> FieldViolation {
    match error {
        DealTransitionError::LostReasonRequired => {
            FieldViolation::new("lost_reason_id", "is required to lose a deal")
        }
        DealTransitionError::StageInactive => {
            FieldViolation::new("stage_id", "stage is not active")
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StageTransitionEntry {
    pub(crate) event_id: Uuid,
    pub(crate) deal_id: Uuid,
    pub(crate) from_stage_id: Option<Uuid>,
    pub(crate) stage_id: Uuid,
    pub(crate) entered_at: DateTime<Utc>,
}
