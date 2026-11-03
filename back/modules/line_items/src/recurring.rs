use aphel_kernel::records::{InputValidator, Patch};
use aphel_sdk::line_items::{LineData, Recurrence, parse_term_months, term_violation};
pub(crate) fn apply_item_recurrence(
    data: &mut LineData,
    recurrence: Patch<String>,
    term_months: Patch<String>,
    validator: &mut InputValidator,
) {
    if let Some(value) = recurrence {
        match value.as_deref().map(str::trim) {
            None | Some("") => data.recurrence = Recurrence::None,
            Some(text) => match Recurrence::parse(text) {
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
    if data.recurrence == Recurrence::None {
        data.term_months = None;
    }
}
