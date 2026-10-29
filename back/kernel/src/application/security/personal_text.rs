use std::sync::Arc;

use crate::application::fields::is_sql_identifier;
use crate::modules::App;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PersonalTextColumn {
    pub table: &'static str,
    pub column: &'static str,
}

impl PersonalTextColumn {
    pub const fn new(table: &'static str, column: &'static str) -> Self {
        Self { table, column }
    }
}

pub trait PersonalTextSource: Send + Sync {
    fn personal_text_columns(&self) -> Vec<PersonalTextColumn>;
}

pub const PLATFORM_PERSONAL_TEXT_COLUMNS: [PersonalTextColumn; 2] = [
    PersonalTextColumn::new("domain_events", "payload"),
    PersonalTextColumn::new("audit_logs", "changes"),
];

#[derive(Debug, Clone, Copy, Default)]
pub struct PlatformPersonalText;

impl PersonalTextSource for PlatformPersonalText {
    fn personal_text_columns(&self) -> Vec<PersonalTextColumn> {
        PLATFORM_PERSONAL_TEXT_COLUMNS.to_vec()
    }
}

pub fn declared_personal_text_columns(app: &App) -> Vec<PersonalTextColumn> {
    let mut columns: Vec<PersonalTextColumn> = Vec::new();
    let sources = std::iter::once(Arc::new(PlatformPersonalText) as Arc<dyn PersonalTextSource>)
        .chain(
            app.all_contributions::<dyn PersonalTextSource>()
                .into_iter()
                .map(|(_, source)| source),
        );
    for source in sources {
        for column in source.personal_text_columns() {
            if is_sql_identifier(column.table)
                && is_sql_identifier(column.column)
                && !columns.contains(&column)
            {
                columns.push(column);
            }
        }
    }
    columns
}
