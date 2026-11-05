use chrono::{DateTime, Utc};
use serde_json::{Value, json};
use uuid::Uuid;

use aphel_kernel::domain::UserId;
use aphel_sdk::platform::application::fields::WriteMode;
use aphel_sdk::platform::application::shared::RecordType;
use aphel_sdk::platform::domain::custom_objects::is_custom_record_type;

pub const VALIDATION_RULE_FAILED: &str = "validation_rule_failed";
pub const VALIDATION_RULE_NOT_FOUND: &str = "validation_rule_not_found";

pub const VALIDATION_RULE_CREATED: &str = "validation_rule.created";
pub const VALIDATION_RULE_UPDATED: &str = "validation_rule.updated";
pub const VALIDATION_RULE_DELETED: &str = "validation_rule.deleted";
pub const VALIDATION_RULES_REORDERED: &str = "validation_rule.reordered";

pub const VALIDATION_RULE_RECORD: RecordType = RecordType::from_static("validation_rule");

pub const RECORD_PATH: &str = "record";
pub const MAX_RULE_NAME_LENGTH: usize = 120;
pub const MAX_RULE_MESSAGE_LENGTH: usize = 500;
pub const MAX_RULES_PER_RECORD_TYPE: usize = 100;

pub fn is_validated_record_type(record_type: &str) -> bool {
    is_custom_record_type(record_type)
        || aphel_sdk::platform::modules::try_app().is_some_and(|app| {
            app.record_types()
                .capabilities(record_type)
                .is_some_and(|capabilities| capabilities.has_trait("validation_rules.subject"))
        })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum RuleTiming {
    #[default]
    Always,
    Create,
    Update,
}

impl RuleTiming {
    pub const ALL: [Self; 3] = [Self::Always, Self::Create, Self::Update];

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Always => "always",
            Self::Create => "create",
            Self::Update => "update",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|timing| timing.as_str() == value)
    }

    pub fn applies(&self, mode: WriteMode) -> bool {
        match self {
            Self::Always => true,
            Self::Create => mode == WriteMode::Create,
            Self::Update => mode == WriteMode::Update,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationRule {
    pub id: Uuid,
    pub key: String,
    pub record_type: String,
    pub name: String,
    pub expression: String,
    pub message: String,
    pub field: Option<String>,
    pub applies_on: RuleTiming,
    pub is_active: bool,
    pub position: i32,
    pub created_by: Option<UserId>,
    pub updated_by: Option<UserId>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl ValidationRule {
    pub fn snapshot(&self) -> Value {
        json!({
            "key": self.key,
            "record_type": self.record_type,
            "name": self.name,
            "expression": self.expression,
            "message": self.message,
            "field": self.field,
            "applies_on": self.applies_on.as_str(),
            "is_active": self.is_active,
            "position": self.position,
        })
    }

    pub fn summary(&self) -> Value {
        json!({
            "id": self.id.to_string(),
            "key": self.key,
            "record_type": self.record_type,
            "name": self.name,
        })
    }
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
pub struct UpdateValidationRuleCommand {
    pub name: Option<String>,
    pub expression: Option<String>,
    pub message: Option<String>,
    pub field: Option<Option<String>>,
    pub applies_on: Option<String>,
    pub is_active: Option<bool>,
    pub expected_updated_at: Option<DateTime<Utc>>,
    pub ip: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ReorderValidationRulesCommand {
    pub record_type: String,
    pub ids: Vec<String>,
    pub ip: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleViolation {
    pub rule_id: Uuid,
    pub key: String,
    pub path: String,
    pub message: String,
}
