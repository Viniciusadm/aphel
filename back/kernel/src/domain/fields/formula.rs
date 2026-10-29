use crate::domain::fields::FieldType;

pub const FORMULA_FIELD_TYPE: &str = "formula";
pub const MAX_FORMULA_LENGTH: usize = 2000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum FormulaResultType {
    Number,
    Currency,
    Date,
    Text,
    Boolean,
}

impl FormulaResultType {
    pub const ALL: [Self; 5] = [
        Self::Number,
        Self::Currency,
        Self::Date,
        Self::Text,
        Self::Boolean,
    ];

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Number => "number",
            Self::Currency => "currency",
            Self::Date => "date",
            Self::Text => "text",
            Self::Boolean => "boolean",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|result_type| result_type.as_str() == value)
    }

    pub fn field_type(&self) -> FieldType {
        match self {
            Self::Number => FieldType::Number,
            Self::Currency => FieldType::Currency,
            Self::Date => FieldType::Date,
            Self::Text => FieldType::Text,
            Self::Boolean => FieldType::Checkbox,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormulaSettings {
    pub expression: String,
    pub result_type: FormulaResultType,
}

pub fn resolve_declared_type(
    declared: &str,
    formula: Option<&FormulaSettings>,
) -> Option<FieldType> {
    if declared == FORMULA_FIELD_TYPE {
        return formula.map(|settings| settings.result_type.field_type());
    }
    FieldType::parse(declared)
}
