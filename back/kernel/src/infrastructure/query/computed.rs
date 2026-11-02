use crate::application::fields::{COMPUTED_RECORD_ALIAS, ComputedColumn, SystemField};
use crate::infrastructure::query::compiler::aliased;

const NOW: &str = "UTC_TIMESTAMP(6)";
pub(crate) fn computed_expression(computed: &ComputedColumn, alias: &str) -> String {
    match computed {
        ComputedColumn::DaysSince { column } => {
            format!("TIMESTAMPDIFF(DAY, {}, {NOW})", aliased(alias, column))
        }
        ComputedColumn::Expression { template } => {
            format!("({})", template.replace(COMPUTED_RECORD_ALIAS, alias))
        }
    }
}

pub fn system_expression(field: &SystemField, alias: &str) -> String {
    if let Some(extension) = &field.extension {
        return format!(
            "(SELECT e.`{}` FROM `{}` e WHERE e.`{}` = {alias}.id)",
            field.column, extension.table, extension.record_column
        );
    }
    match &field.computed {
        Some(computed) => computed_expression(computed, alias),
        None => aliased(alias, &field.column),
    }
}
