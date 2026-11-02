use regex::RegexBuilder;
use rust_decimal::Decimal;
use uuid::Uuid;

use crate::domain::fields::{
    FORMULA_FIELD_TYPE, FieldKey, FieldType, FormulaSettings, MAX_FORMULA_LENGTH, RawValue,
};

pub const MAX_TEXT_LENGTH: u32 = 1024;
pub const MAX_LONG_TEXT_LENGTH: u32 = 65_535;
pub const MAX_DECIMALS: u32 = 6;
pub const DEFAULT_CURRENCY_DECIMALS: u32 = 2;
pub const MAX_PATTERN_LENGTH: usize = 500;
pub const PATTERN_SIZE_LIMIT: usize = 1 << 16;
pub const DEFAULT_PHONE_COUNTRY: &str = "BR";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldOption {
    pub id: Uuid,
    pub value: String,
    pub label: String,
    pub color: Option<String>,
    pub position: i32,
    pub is_active: bool,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct FieldConfig {
    pub min: Option<Decimal>,
    pub max: Option<Decimal>,
    pub decimals: Option<u32>,
    pub currency: Option<String>,
    pub max_length: Option<u32>,
    pub pattern: Option<String>,
    pub default_country: Option<String>,
    pub target_record_type: Option<String>,
    pub default_value: Option<RawValue>,
    pub formula: Option<FormulaSettings>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigProperty {
    Min,
    Max,
    Decimals,
    Currency,
    MaxLength,
    Pattern,
    DefaultCountry,
    TargetRecordType,
    DefaultValue,
    Expression,
    ResultType,
}

impl ConfigProperty {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Min => "min",
            Self::Max => "max",
            Self::Decimals => "decimals",
            Self::Currency => "currency",
            Self::MaxLength => "max_length",
            Self::Pattern => "pattern",
            Self::DefaultCountry => "default_country",
            Self::TargetRecordType => "target_record_type",
            Self::DefaultValue => "default_value",
            Self::Expression => "expression",
            Self::ResultType => "result_type",
        }
    }

    pub fn supported_by(&self, field_type: FieldType) -> bool {
        let field_type = field_type.base_type();
        use FieldType::*;
        match self {
            Self::Min | Self::Max | Self::Decimals => {
                matches!(field_type, Number | Currency)
            }
            Self::Currency => field_type == Currency,
            Self::MaxLength => matches!(field_type, Text | LongText),
            Self::Pattern => field_type == Text,
            Self::DefaultCountry => field_type == Phone,
            Self::TargetRecordType => field_type == Record,
            Self::DefaultValue => !matches!(field_type, User | Record),
            Self::Expression | Self::ResultType => {
                matches!(field_type, Number | Currency | Date | Text | Checkbox)
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigViolation {
    pub property: ConfigProperty,
    pub message: String,
}

impl ConfigViolation {
    fn new(property: ConfigProperty, message: impl Into<String>) -> Self {
        Self {
            property,
            message: message.into(),
        }
    }
}

impl FieldConfig {
    pub fn present_properties(&self) -> Vec<ConfigProperty> {
        [
            (self.min.is_some(), ConfigProperty::Min),
            (self.max.is_some(), ConfigProperty::Max),
            (self.decimals.is_some(), ConfigProperty::Decimals),
            (self.currency.is_some(), ConfigProperty::Currency),
            (self.max_length.is_some(), ConfigProperty::MaxLength),
            (self.pattern.is_some(), ConfigProperty::Pattern),
            (
                self.default_country.is_some(),
                ConfigProperty::DefaultCountry,
            ),
            (
                self.target_record_type.is_some(),
                ConfigProperty::TargetRecordType,
            ),
            (self.default_value.is_some(), ConfigProperty::DefaultValue),
            (self.formula.is_some(), ConfigProperty::Expression),
            (self.formula.is_some(), ConfigProperty::ResultType),
        ]
        .into_iter()
        .filter_map(|(present, property)| present.then_some(property))
        .collect()
    }

    pub fn allowed_decimals(&self, field_type: FieldType) -> u32 {
        let default = if field_type.base_type() == FieldType::Currency {
            DEFAULT_CURRENCY_DECIMALS
        } else {
            MAX_DECIMALS
        };
        self.decimals.unwrap_or(default).min(MAX_DECIMALS)
    }

    pub fn max_text_length(&self, field_type: FieldType) -> u32 {
        let limit = if field_type.base_type() == FieldType::LongText {
            MAX_LONG_TEXT_LENGTH
        } else {
            MAX_TEXT_LENGTH
        };
        self.max_length.unwrap_or(limit).min(limit)
    }

    pub fn phone_country(&self) -> &str {
        self.default_country
            .as_deref()
            .unwrap_or(DEFAULT_PHONE_COUNTRY)
    }

    pub fn validate(&self, field_type: FieldType) -> Vec<ConfigViolation> {
        let mut violations: Vec<ConfigViolation> = self
            .present_properties()
            .into_iter()
            .filter(|property| !property.supported_by(field_type))
            .map(|property| {
                ConfigViolation::new(
                    property,
                    format!("is not supported by {} fields", field_type.as_str()),
                )
            })
            .collect();
        if !violations.is_empty() {
            return violations;
        }
        if let Some(formula) = &self.formula {
            violations.extend(self.formula_violations(formula, field_type));
            if !violations.is_empty() {
                return violations;
            }
        }

        if let (Some(min), Some(max)) = (self.min, self.max)
            && min > max
        {
            violations.push(ConfigViolation::new(
                ConfigProperty::Min,
                "must be less than or equal to max",
            ));
        }
        if self
            .decimals
            .is_some_and(|decimals| decimals > MAX_DECIMALS)
        {
            violations.push(ConfigViolation::new(
                ConfigProperty::Decimals,
                format!("must be between 0 and {MAX_DECIMALS}"),
            ));
        }
        if let Some(currency) = &self.currency
            && !(currency.len() == 3 && currency.chars().all(|c| c.is_ascii_uppercase()))
        {
            violations.push(ConfigViolation::new(
                ConfigProperty::Currency,
                "must be an ISO 4217 code such as BRL",
            ));
        }
        if let Some(max_length) = self.max_length {
            let limit = if field_type.base_type() == FieldType::LongText {
                MAX_LONG_TEXT_LENGTH
            } else {
                MAX_TEXT_LENGTH
            };
            if max_length == 0 || max_length > limit {
                violations.push(ConfigViolation::new(
                    ConfigProperty::MaxLength,
                    format!("must be between 1 and {limit}"),
                ));
            }
        }
        if let Some(pattern) = &self.pattern
            && (pattern.len() > MAX_PATTERN_LENGTH
                || RegexBuilder::new(pattern)
                    .size_limit(PATTERN_SIZE_LIMIT)
                    .build()
                    .is_err())
        {
            violations.push(ConfigViolation::new(
                ConfigProperty::Pattern,
                "must be a valid regular expression",
            ));
        }
        if let Some(country) = &self.default_country
            && country.parse::<phonenumber::country::Id>().is_err()
        {
            violations.push(ConfigViolation::new(
                ConfigProperty::DefaultCountry,
                "must be an ISO 3166 country code such as BR",
            ));
        }
        if field_type.base_type() == FieldType::Record && self.target_record_type.is_none() {
            violations.push(ConfigViolation::new(
                ConfigProperty::TargetRecordType,
                "is required for record fields",
            ));
        }
        violations
    }
}

impl FieldConfig {
    fn formula_violations(
        &self,
        formula: &FormulaSettings,
        field_type: FieldType,
    ) -> Vec<ConfigViolation> {
        let mut violations: Vec<ConfigViolation> = self
            .present_properties()
            .into_iter()
            .filter(|property| {
                !matches!(
                    property,
                    ConfigProperty::Expression
                        | ConfigProperty::ResultType
                        | ConfigProperty::Decimals
                        | ConfigProperty::Currency
                )
            })
            .map(|property| ConfigViolation::new(property, "is not supported by formula fields"))
            .collect();
        let length = formula.expression.chars().count();
        if formula.expression.trim().is_empty() || length > MAX_FORMULA_LENGTH {
            violations.push(ConfigViolation::new(
                ConfigProperty::Expression,
                format!("must have between 1 and {MAX_FORMULA_LENGTH} characters"),
            ));
        }
        if formula.result_type.field_type() != field_type {
            violations.push(ConfigViolation::new(
                ConfigProperty::ResultType,
                "does not match the field type",
            ));
        }
        violations
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FieldDefinition {
    pub id: Uuid,
    pub record_type: String,
    pub key: FieldKey,
    pub label: String,
    pub field_type: FieldType,
    pub is_system: bool,
    pub column_name: Option<String>,
    pub is_required: bool,
    pub is_unique: bool,
    pub is_searchable: bool,
    pub is_personal: bool,
    pub is_sensitive: bool,
    pub module_key: Option<String>,
    pub is_active: bool,
    pub position: i32,
    pub config: FieldConfig,
    pub options: Vec<FieldOption>,
}

impl FieldDefinition {
    pub fn is_managed(&self) -> bool {
        self.module_key.is_some()
    }

    pub fn option_by_value(&self, value: &str) -> Option<&FieldOption> {
        self.options.iter().find(|option| option.value == value)
    }

    pub fn option_by_id(&self, id: Uuid) -> Option<&FieldOption> {
        self.options.iter().find(|option| option.id == id)
    }

    pub fn is_custom(&self) -> bool {
        !self.is_system
    }

    pub fn formula(&self) -> Option<&FormulaSettings> {
        self.config.formula.as_ref()
    }

    pub fn is_formula(&self) -> bool {
        self.config.formula.is_some()
    }

    pub fn type_name(&self) -> &'static str {
        if self.is_formula() {
            FORMULA_FIELD_TYPE
        } else {
            self.field_type.as_str()
        }
    }
}
