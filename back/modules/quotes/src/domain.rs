use aphel_sdk::line_items::line_discount;
use std::collections::BTreeMap;

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde_json::{Map, Value};
use uuid::Uuid;

use aphel_kernel::domain::UserId;
use aphel_kernel::records::Patch;
use aphel_sdk::email::EmailAddress;
use aphel_sdk::line_items::{DiscountKind, Recurrence, money_limit, round_money};
use aphel_sdk::platform::application::fields::permissions::provided_key;

pub(crate) const MAX_QUOTE_TITLE_LENGTH: usize = 255;
pub(crate) const MAX_QUOTE_ITEMS: usize = 200;
pub(crate) const MAX_QUOTE_TEXT_LENGTH: usize = 20_000;
pub(crate) const MAX_ITEM_DESCRIPTION_LENGTH: usize = 2_000;
pub(crate) const MAX_REJECTION_REASON_LENGTH: usize = 2_000;
pub(crate) const MAX_ACCEPTANCE_NAME_LENGTH: usize = 160;
pub(crate) const TAX_DECIMALS: u32 = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub(crate) enum QuoteStatus {
    #[default]
    Draft,
    PendingApproval,
    Approved,
    Sent,
    Accepted,
    Rejected,
    Expired,
}

impl QuoteStatus {
    pub(crate) const ALL: [Self; 7] = [
        Self::Draft,
        Self::PendingApproval,
        Self::Approved,
        Self::Sent,
        Self::Accepted,
        Self::Rejected,
        Self::Expired,
    ];

    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::PendingApproval => "pending_approval",
            Self::Approved => "approved",
            Self::Sent => "sent",
            Self::Accepted => "accepted",
            Self::Rejected => "rejected",
            Self::Expired => "expired",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|status| status.as_str() == value)
    }

    pub(crate) fn is_editable(&self) -> bool {
        matches!(self, Self::Draft | Self::Approved)
    }

    pub(crate) fn can_submit(&self) -> bool {
        matches!(self, Self::Draft)
    }

    pub(crate) fn can_send(&self) -> bool {
        matches!(self, Self::Draft | Self::Approved | Self::Sent)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct QuoteItemComponent {
    pub(crate) product_id: Option<Uuid>,
    pub(crate) name: String,
    pub(crate) code: Option<String>,
    pub(crate) quantity: Decimal,
    pub(crate) unit_price: Decimal,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct QuoteItemData {
    pub(crate) product_id: Option<Uuid>,
    pub(crate) name: String,
    pub(crate) description: Option<String>,
    pub(crate) unit: Option<String>,
    pub(crate) quantity: Decimal,
    pub(crate) unit_price: Decimal,
    pub(crate) discount_kind: DiscountKind,
    pub(crate) discount_value: Decimal,
    pub(crate) tax_percent: Decimal,
    pub(crate) recurrence: Recurrence,
    pub(crate) term_months: Option<u16>,
    pub(crate) components: Vec<QuoteItemComponent>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct QuoteDiscount {
    pub(crate) kind: DiscountKind,
    pub(crate) value: Decimal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct LineBreakdown {
    pub(crate) gross: Decimal,
    pub(crate) item_discount: Decimal,
    pub(crate) net: Decimal,
    pub(crate) total_discount_share: Decimal,
    pub(crate) taxable: Decimal,
    pub(crate) tax: Decimal,
    pub(crate) total: Decimal,
}

impl LineBreakdown {
    pub(crate) fn discount(&self) -> Decimal {
        self.item_discount + self.total_discount_share
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct TotalsBreakdown {
    pub(crate) lines: Vec<LineBreakdown>,
    pub(crate) subtotal: Decimal,
    pub(crate) items_discount: Decimal,
    pub(crate) net_subtotal: Decimal,
    pub(crate) total_discount: Decimal,
    pub(crate) discount_total: Decimal,
    pub(crate) tax_total: Decimal,
    pub(crate) total: Decimal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TotalsError {
    ItemPercentOutOfRange(usize),
    ItemTaxOutOfRange(usize),
    ItemTooLarge(usize),
    TotalPercentOutOfRange,
    TooLarge,
}

fn percent_of(base: Decimal, percent: Decimal) -> Decimal {
    round_money(base * percent / Decimal::ONE_HUNDRED)
}

fn valid_percent(value: Decimal) -> bool {
    value >= Decimal::ZERO && value <= Decimal::ONE_HUNDRED
}

fn allocate(total: Decimal, nets: &[Decimal]) -> Vec<Decimal> {
    let base: Decimal = nets.iter().copied().sum();
    let mut shares = vec![Decimal::ZERO; nets.len()];
    if total <= Decimal::ZERO || base <= Decimal::ZERO {
        return shares;
    }
    let Some(last) = nets.iter().rposition(|net| *net > Decimal::ZERO) else {
        return shares;
    };
    let mut allocated = Decimal::ZERO;
    for (index, net) in nets.iter().enumerate().take(last) {
        let share = round_money(total * *net / base).min(*net);
        shares[index] = share;
        allocated += share;
    }
    shares[last] = (total - allocated).clamp(Decimal::ZERO, nets[last]);
    shares
}

pub(crate) fn compute_totals(
    items: &[QuoteItemData],
    discount: QuoteDiscount,
) -> Result<TotalsBreakdown, TotalsError> {
    let mut lines = Vec::with_capacity(items.len());
    for (index, item) in items.iter().enumerate() {
        if !valid_percent(item.tax_percent) {
            return Err(TotalsError::ItemTaxOutOfRange(index));
        }
        let amounts = aphel_sdk::line_items::LineData {
            quantity: item.quantity,
            unit_price: item.unit_price,
            discount_kind: item.discount_kind,
            discount_value: item.discount_value,
            ..Default::default()
        }
        .amounts()
        .map_err(|error| match error {
            aphel_sdk::line_items::LineTotalError::PercentOutOfRange => {
                TotalsError::ItemPercentOutOfRange(index)
            }
            _ => TotalsError::ItemTooLarge(index),
        })?;
        let gross = amounts.gross;
        let item_discount = amounts.discount;
        lines.push(LineBreakdown {
            gross,
            item_discount,
            net: gross - item_discount,
            ..LineBreakdown::default()
        });
    }
    let subtotal: Decimal = lines.iter().map(|line| line.gross).sum();
    let items_discount: Decimal = lines.iter().map(|line| line.item_discount).sum();
    let net_subtotal = subtotal - items_discount;
    let total_discount = line_discount(net_subtotal, discount.kind, discount.value)
        .ok_or(TotalsError::TotalPercentOutOfRange)?;
    let nets: Vec<Decimal> = lines.iter().map(|line| line.net).collect();
    for ((line, share), item) in lines
        .iter_mut()
        .zip(allocate(total_discount, &nets))
        .zip(items)
    {
        line.total_discount_share = share;
        line.taxable = line.net - share;
        line.tax = percent_of(line.taxable, item.tax_percent);
        line.total = line.taxable + line.tax;
    }
    let tax_total: Decimal = lines.iter().map(|line| line.tax).sum();
    let total: Decimal = lines.iter().map(|line| line.total).sum();
    if subtotal >= money_limit() || total >= money_limit() {
        return Err(TotalsError::TooLarge);
    }
    Ok(TotalsBreakdown {
        lines,
        subtotal,
        items_discount,
        net_subtotal,
        total_discount,
        discount_total: items_discount + total_discount,
        tax_total,
        total,
    })
}

fn share_percent(part: Decimal, whole: Decimal) -> Decimal {
    if whole <= Decimal::ZERO {
        return Decimal::ZERO;
    }
    part * Decimal::ONE_HUNDRED / whole
}

pub(crate) fn max_discount_percent(totals: &TotalsBreakdown) -> Decimal {
    let items = totals
        .lines
        .iter()
        .map(|line| share_percent(line.item_discount, line.gross));
    let total = share_percent(totals.total_discount, totals.net_subtotal);
    items
        .chain(std::iter::once(total))
        .fold(Decimal::ZERO, Decimal::max)
}

pub(crate) fn exceeds_discount_limit(totals: &TotalsBreakdown, limit: Option<Decimal>) -> bool {
    limit.is_some_and(|limit| max_discount_percent(totals) > limit)
}

pub(crate) fn normalize_tax_percent(value: Decimal) -> Option<Decimal> {
    let percent = value.round_dp(TAX_DECIMALS);
    valid_percent(percent).then(|| percent.normalize())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct QuoteAcceptance {
    pub(crate) name: String,
    pub(crate) email: String,
    pub(crate) ip: Option<String>,
    pub(crate) user_agent: Option<String>,
    pub(crate) recorded_by: Option<UserId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Quote {
    pub id: Uuid,
    pub deal_id: Uuid,
    pub(crate) number: u32,
    pub(crate) version: u32,
    pub(crate) is_current: bool,
    pub(crate) title: String,
    pub(crate) status: QuoteStatus,
    pub(crate) owner_id: Option<UserId>,
    pub(crate) contact_id: Option<Uuid>,
    pub(crate) company_id: Option<Uuid>,
    pub(crate) currency: String,
    pub(crate) price_list_id: Option<Uuid>,
    pub(crate) valid_until: Option<NaiveDate>,
    pub(crate) terms: Option<String>,
    pub(crate) notes: Option<String>,
    pub(crate) discount: QuoteDiscount,
    pub(crate) subtotal: Decimal,
    pub(crate) discount_total: Decimal,
    pub(crate) tax_total: Decimal,
    pub(crate) total: Decimal,
    pub(crate) exceeds_discount_limit: bool,
    pub(crate) public_key: Option<String>,
    pub(crate) sent_at: Option<DateTime<Utc>>,
    pub(crate) sent_by: Option<UserId>,
    pub(crate) viewed_at: Option<DateTime<Utc>>,
    pub(crate) accepted_at: Option<DateTime<Utc>>,
    pub(crate) rejected_at: Option<DateTime<Utc>>,
    pub(crate) expired_at: Option<DateTime<Utc>>,
    pub(crate) acceptance: Option<QuoteAcceptance>,
    pub(crate) rejection_reason: Option<String>,
    pub(crate) deal_synced_at: Option<DateTime<Utc>>,
    pub(crate) deal_sync_error: Option<String>,
    pub(crate) created_by: Option<UserId>,
    pub(crate) created_at: DateTime<Utc>,
    pub(crate) updated_at: DateTime<Utc>,
}

impl Quote {
    pub(crate) fn apply_totals(&mut self, totals: &TotalsBreakdown) {
        self.subtotal = totals.subtotal;
        self.discount_total = totals.discount_total;
        self.tax_total = totals.tax_total;
        self.total = totals.total;
    }

    pub(crate) fn is_expired_on(&self, today: NaiveDate) -> bool {
        self.valid_until.is_some_and(|until| until < today)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct QuoteItem {
    pub(crate) id: Uuid,
    pub(crate) quote_id: Uuid,
    pub(crate) position: i32,
    pub(crate) data: QuoteItemData,
    pub(crate) amounts: LineBreakdown,
}

pub(crate) const QUOTE_NOT_FOUND: &str = "quote_not_found";
pub(crate) const QUOTE_NOT_EDITABLE: &str = "quote_not_editable";
pub(crate) const QUOTE_INVALID_TRANSITION: &str = "quote_invalid_transition";
pub(crate) const QUOTE_NOT_CURRENT: &str = "quote_not_current";
pub(crate) const QUOTE_ALREADY_ACCEPTED: &str = "quote_already_accepted";
pub(crate) const QUOTE_EXPIRED: &str = "quote_expired";
pub(crate) const QUOTE_SUPERSEDED: &str = "quote_superseded";
pub(crate) const QUOTE_CLOSED: &str = "quote_closed";
pub(crate) const QUOTE_HAS_NO_ITEMS: &str = "quote_has_no_items";
pub(crate) const QUOTE_NOT_SENT: &str = "quote_not_sent";
pub(crate) const QUOTE_ITEMS_LIMIT_REACHED: &str = "quote_items_limit_reached";
pub(crate) const QUOTE_PRICE_LIST_NOT_FOUND: &str = "quote_price_list_not_found";

pub(crate) const QUOTE_CREATED: &str = "quote.created";
pub(crate) const QUOTE_UPDATED: &str = "quote.updated";
pub(crate) const QUOTE_DELETED: &str = "quote.deleted";
pub(crate) const QUOTE_SUBMITTED: &str = "quote.submitted";
pub(crate) const QUOTE_APPROVED: &str = "quote.approved";
pub(crate) const QUOTE_RETURNED_TO_DRAFT: &str = "quote.returned_to_draft";
pub(crate) const QUOTE_SENT: &str = "quote.sent";
pub(crate) const QUOTE_VIEWED: &str = "quote.viewed";
pub(crate) const QUOTE_ACCEPTED: &str = "quote.accepted";
pub(crate) const QUOTE_REJECTED: &str = "quote.rejected";
pub(crate) const QUOTE_EXPIRED_EVENT: &str = "quote.expired";
pub(crate) const QUOTE_VERSION_CREATED: &str = "quote.version_created";
pub(crate) const QUOTE_DEAL_SYNCED: &str = "quote.deal_synced";
pub(crate) const QUOTE_DEAL_SYNC_FAILED: &str = "quote.deal_sync_failed";
pub(crate) const QUOTE_SETTINGS_UPDATED: &str = "quote_settings.updated";

pub(crate) const DEAL_QUOTE_SENT: &str = "deal.quote_sent";
pub(crate) const DEAL_QUOTE_ACCEPTED: &str = "deal.quote_accepted";
pub(crate) const DEAL_QUOTE_REJECTED: &str = "deal.quote_rejected";

pub(crate) const QUOTE_DEAL_SYNC_JOB: &str = "quote.deal_sync";
pub(crate) const QUOTES_EXPIRE_JOB: &str = "quotes.expire";
pub(crate) const QUOTES_EXPIRE_CRON: &str = "0 7 * * * *";
pub(crate) const QUOTE_APPROVALS_CONSUMER: &str = "quotes.approvals";

pub(crate) const QUOTE_LINK_PATH: &str = "/quote";
pub(crate) const QUOTE_LINK_PLACEHOLDER: &str = "{{quote_link}}";
pub(crate) const TOKEN_KEY_OWNER: &str = "quote_links";
pub(crate) const TOKEN_KEY_NAME: &str = "token_key";
pub(crate) const MAX_DEFAULT_VALID_DAYS: u16 = 3_650;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct QuoteItemInput {
    pub(crate) product_id: Option<String>,
    pub(crate) name: Option<String>,
    pub(crate) description: Option<String>,
    pub(crate) unit: Option<String>,
    pub(crate) quantity: Option<String>,
    pub(crate) unit_price: Option<String>,
    pub(crate) discount_type: Option<String>,
    pub(crate) discount_value: Option<String>,
    pub(crate) tax_percent: Option<String>,
    pub(crate) recurrence: Option<String>,
    pub(crate) term_months: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct QuoteInput {
    pub title: Patch<String>,
    pub owner_id: Patch<String>,
    pub price_list_id: Patch<String>,
    pub valid_until: Patch<String>,
    pub terms: Patch<String>,
    pub notes: Patch<String>,
    pub discount_type: Patch<String>,
    pub discount_value: Patch<String>,
}

impl QuoteInput {
    pub(crate) fn provided_keys(&self) -> Vec<&'static str> {
        let mut keys = Vec::new();
        provided_key(&mut keys, "title", self.title.is_some());
        provided_key(&mut keys, "owner_id", self.owner_id.is_some());
        provided_key(&mut keys, "valid_until", self.valid_until.is_some());
        keys
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct CreateQuoteCommand {
    pub deal_id: String,
    pub from_line_items: bool,
    pub input: QuoteInput,
    pub items: Option<Vec<QuoteItemInput>>,
    pub custom_fields: Map<String, Value>,
    pub ip: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct UpdateQuoteCommand {
    pub(crate) input: QuoteInput,
    pub(crate) items: Option<Vec<QuoteItemInput>>,
    pub(crate) reprice: bool,
    pub(crate) custom_fields: Map<String, Value>,
    pub(crate) ip: Option<String>,
    pub(crate) expected_updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct SubmitQuoteCommand {
    pub(crate) comment: Option<String>,
    pub(crate) ip: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct SendQuoteCommand {
    pub(crate) to: Vec<EmailAddress>,
    pub(crate) cc: Vec<EmailAddress>,
    pub(crate) subject: Option<String>,
    pub(crate) body_html: Option<String>,
    pub(crate) mailbox_id: Option<String>,
    pub(crate) ip: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct AcceptQuoteCommand {
    pub(crate) name: String,
    pub(crate) email: String,
    pub(crate) ip: Option<String>,
    pub(crate) user_agent: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ManualAcceptQuoteCommand {
    pub(crate) name: Option<String>,
    pub(crate) email: Option<String>,
    pub(crate) note: Option<String>,
    pub(crate) ip: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct RejectQuoteCommand {
    pub(crate) reason: Option<String>,
    pub(crate) ip: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct QuoteDetails {
    pub quote: Quote,
    pub(crate) items: Vec<QuoteItem>,
    pub(crate) items_discount: Decimal,
    pub(crate) total_discount: Decimal,
    pub(crate) max_discount_percent: Decimal,
    pub(crate) discount_limit: Option<Decimal>,
    pub(crate) versions: Vec<QuoteVersionSummary>,
    pub(crate) custom_fields: Map<String, Value>,
    pub(crate) public_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct QuoteSummary {
    pub(crate) quote: Quote,
    pub(crate) custom_fields: Map<String, Value>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct QuoteVersionSummary {
    pub(crate) id: Uuid,
    pub(crate) version: u32,
    pub(crate) status: QuoteStatus,
    pub(crate) total: Decimal,
    pub(crate) is_current: bool,
    pub(crate) created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct QuoteSettings {
    pub(crate) default_valid_days: Option<u16>,
    pub(crate) default_terms: Option<String>,
    pub(crate) accept_stages: BTreeMap<Uuid, Uuid>,
    pub(crate) updated_by: Option<UserId>,
    pub(crate) updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct UpdateQuoteSettingsCommand {
    pub(crate) default_valid_days: Patch<u16>,
    pub(crate) default_terms: Patch<String>,
    pub(crate) accept_stages: Option<BTreeMap<String, Option<String>>>,
    pub(crate) ip: Option<String>,
    pub(crate) expected_updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct QuoteOrganization {
    pub(crate) name: String,
    pub(crate) timezone: String,
    pub(crate) currency: String,
    pub(crate) locale: String,
    pub(crate) logo_file_id: Option<Uuid>,
    pub(crate) primary_color: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PublicQuoteState {
    Open,
    Accepted,
    Rejected,
    Expired,
    Superseded,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PublicQuote {
    pub(crate) organization: QuoteOrganization,
    pub(crate) quote: Quote,
    pub(crate) items: Vec<QuoteItem>,
    pub(crate) items_discount: Decimal,
    pub(crate) total_discount: Decimal,
    pub(crate) state: PublicQuoteState,
    pub(crate) owner_name: Option<String>,
    pub(crate) owner_email: Option<String>,
    pub(crate) contact_name: Option<String>,
    pub(crate) company_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct QuotePeople {
    pub(crate) owner_name: Option<String>,
    pub(crate) owner_email: Option<String>,
    pub(crate) contact_name: Option<String>,
    pub(crate) company_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct QuoteDeal {
    pub(crate) id: Uuid,
    pub(crate) title: String,
    pub(crate) currency: String,
    pub(crate) owner_id: Option<UserId>,
    pub(crate) contact_id: Option<Uuid>,
    pub(crate) company_id: Option<Uuid>,
    pub(crate) pipeline_id: Uuid,
    pub(crate) stage_id: Uuid,
    pub(crate) is_open: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct QuoteEmail {
    pub(crate) deal_id: Uuid,
    pub(crate) mailbox_id: Option<String>,
    pub(crate) to: Vec<EmailAddress>,
    pub(crate) cc: Vec<EmailAddress>,
    pub(crate) subject: String,
    pub(crate) body_html: String,
    pub(crate) ip: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineItemsOutcome {
    pub amount_managed: bool,
}
