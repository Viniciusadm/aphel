use chrono::{DateTime, Days, Months, NaiveDate, Utc};
use chrono_tz::Tz;
use rust_decimal::Decimal;

use aphel_kernel::records::{InputValidator, Patch};
use deals::domain::{Deal, DealRevenue, DealStatus};
use products::domain::{
    DealItem, DealItemData, ProductRecurrence, normalize_money, parse_term_months, term_violation,
};

pub(crate) const DEFAULT_TERM_MONTHS: u16 = 12;
pub(crate) const MONTHS_PER_YEAR: u32 = 12;

pub(crate) fn period_months(recurrence: ProductRecurrence) -> Option<u32> {
    match recurrence {
        ProductRecurrence::None => None,
        ProductRecurrence::Monthly => Some(1),
        ProductRecurrence::Quarterly => Some(3),
        ProductRecurrence::Yearly => Some(MONTHS_PER_YEAR),
    }
}

pub(crate) fn effective_term(term_months: Option<u16>) -> u32 {
    u32::from(term_months.unwrap_or(DEFAULT_TERM_MONTHS).max(1))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct RecurringTotals {
    pub(crate) mrr: Option<Decimal>,
    pub(crate) arr: Option<Decimal>,
    pub(crate) tcv: Option<Decimal>,
    pub(crate) term_months: Option<u32>,
}

pub(crate) fn monthly_amount(item: &DealItem) -> Option<Decimal> {
    period_months(item.data.recurrence).map(|period| item.total / Decimal::from(period))
}

pub(crate) fn recurring_totals(items: &[DealItem]) -> RecurringTotals {
    if items.is_empty() {
        return RecurringTotals::default();
    }
    let mut monthly = Decimal::ZERO;
    let mut contract = Decimal::ZERO;
    let mut term: Option<u32> = None;
    for item in items {
        match period_months(item.data.recurrence) {
            None => contract += item.total,
            Some(period) => {
                let months = effective_term(item.data.term_months);
                monthly += item.total / Decimal::from(period);
                contract += item.total * Decimal::from(months) / Decimal::from(period);
                term = Some(term.map_or(months, |current| current.max(months)));
            }
        }
    }
    let mrr = term.and(normalize_money(monthly));
    RecurringTotals {
        mrr,
        arr: mrr.and_then(|mrr| normalize_money(mrr * Decimal::from(MONTHS_PER_YEAR))),
        tcv: normalize_money(contract),
        term_months: term,
    }
}

pub(crate) fn contract_end(start: NaiveDate, term_months: u32) -> Option<NaiveDate> {
    start
        .checked_add_months(Months::new(term_months))?
        .checked_sub_days(Days::new(1))
}

pub(crate) fn won_on(deal: &Deal, timezone: Tz) -> Option<NaiveDate> {
    won_date(deal.status, deal.closed_at, timezone)
}

pub(crate) fn won_date(
    status: DealStatus,
    closed_at: Option<DateTime<Utc>>,
    timezone: Tz,
) -> Option<NaiveDate> {
    match (status, closed_at) {
        (DealStatus::Won, Some(at)) => Some(at.with_timezone(&timezone).date_naive()),
        _ => None,
    }
}

pub(crate) fn apply_revenue(
    revenue: &mut DealRevenue,
    items: &[DealItem],
    won_on: Option<NaiveDate>,
) {
    let totals = recurring_totals(items);
    revenue.mrr = totals.mrr;
    revenue.arr = totals.arr;
    revenue.tcv = totals.tcv;
    refresh_contract_end(revenue, totals.term_months, won_on);
}

pub(crate) fn refresh_contract_end(
    revenue: &mut DealRevenue,
    term_months: Option<u32>,
    won_on: Option<NaiveDate>,
) {
    let Some(term) = term_months else {
        return;
    };
    if revenue.contract_start.is_none() {
        revenue.contract_start = won_on;
    }
    if let Some(start) = revenue.contract_start {
        revenue.contract_end = contract_end(start, term);
    }
}

pub(crate) fn apply_item_recurrence(
    data: &mut DealItemData,
    recurrence: Patch<String>,
    term_months: Patch<String>,
    validator: &mut InputValidator,
) {
    if let Some(value) = recurrence {
        match value.as_deref().map(str::trim) {
            None | Some("") => data.recurrence = ProductRecurrence::None,
            Some(text) => match ProductRecurrence::parse(text) {
                Some(recurrence) => data.recurrence = recurrence,
                None => {
                    validator.violation("recurrence", "must be none, monthly, quarterly or yearly")
                }
            },
        }
    }
    if let Some(value) = term_months {
        match value.as_deref().map(str::trim) {
            None | Some("") => data.term_months = None,
            Some(text) => match parse_term_months(text) {
                Some(months) => data.term_months = Some(months),
                None => validator.violation("term_months", term_violation()),
            },
        }
    }
    if data.recurrence == ProductRecurrence::None {
        data.term_months = None;
    }
}
