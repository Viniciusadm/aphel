#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ComputedColumn {
    DaysSince { column: String },
    Expression { template: String },
}

pub const COMPUTED_RECORD_ALIAS: &str = "{record}";

impl ComputedColumn {
    pub fn days_since(column: &str) -> Self {
        Self::DaysSince {
            column: column.to_owned(),
        }
    }

    pub fn expression(template: impl Into<String>) -> Self {
        Self::Expression {
            template: template.into(),
        }
    }

    pub fn identifiers(&self) -> Vec<&str> {
        match self {
            Self::DaysSince { column } => vec![column.as_str()],
            Self::Expression { .. } => Vec::new(),
        }
    }
}
