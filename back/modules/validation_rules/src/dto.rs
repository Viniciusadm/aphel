use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::application::{
    CreateValidationRuleCommand, ReorderValidationRulesCommand, UpdateValidationRuleCommand,
    ValidationRule, ValidationRulePreview,
};
use aphel_kernel::api::dto::common::double_option;
use aphel_kernel::api::dto::common::timestamp;
use aphel_sdk::formulas::dto::FormulaPreviewResponse;

#[derive(Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[schema(as = ValidationRuleTiming)]
pub enum ValidationRuleTimingSchema {
    Always,
    Create,
    Update,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ValidationRuleResponse {
    pub id: String,
    #[schema(example = "discount_needs_reason")]
    pub key: String,
    #[schema(example = "deals.deal")]
    pub record_type: String,
    #[schema(example = "Discount requires justification")]
    pub name: String,
    #[schema(example = "amount > 10000 and is_blank(source)")]
    pub expression: String,
    #[schema(example = "Provide the source for deals above 10k.")]
    pub message: String,
    #[schema(example = "source")]
    pub field: Option<String>,
    #[schema(value_type = ValidationRuleTimingSchema)]
    pub applies_on: String,
    pub is_active: bool,
    pub position: i32,
    pub created_by: Option<String>,
    pub updated_by: Option<String>,
    #[schema(example = "2026-10-20T13:45:00Z")]
    pub created_at: String,
    #[schema(example = "2026-10-20T13:45:00Z")]
    pub updated_at: String,
}

impl From<ValidationRule> for ValidationRuleResponse {
    fn from(rule: ValidationRule) -> Self {
        Self {
            id: rule.id.to_string(),
            key: rule.key,
            record_type: rule.record_type,
            name: rule.name,
            expression: rule.expression,
            message: rule.message,
            field: rule.field,
            applies_on: rule.applies_on.as_str().to_owned(),
            is_active: rule.is_active,
            position: rule.position,
            created_by: rule.created_by.map(|id| id.into_inner()),
            updated_by: rule.updated_by.map(|id| id.into_inner()),
            created_at: timestamp(rule.created_at),
            updated_at: timestamp(rule.updated_at),
        }
    }
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct ValidationRuleListParams {
    pub record_type: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateValidationRuleRequest {
    #[schema(example = "deals.deal")]
    pub record_type: String,
    #[schema(example = "discount_needs_reason")]
    pub key: Option<String>,
    pub name: String,
    pub expression: String,
    pub message: String,
    pub field: Option<String>,
    #[schema(value_type = Option<ValidationRuleTimingSchema>)]
    pub applies_on: Option<String>,
    pub is_active: Option<bool>,
}

impl CreateValidationRuleRequest {
    pub fn into_command(self, ip: Option<String>) -> CreateValidationRuleCommand {
        CreateValidationRuleCommand {
            record_type: self.record_type,
            key: self.key,
            name: self.name,
            expression: self.expression,
            message: self.message,
            field: self.field,
            applies_on: self.applies_on,
            is_active: self.is_active,
            ip,
        }
    }
}

#[derive(Debug, Default, Deserialize, ToSchema)]
pub struct UpdateValidationRuleRequest {
    pub name: Option<String>,
    pub expression: Option<String>,
    pub message: Option<String>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>)]
    pub field: Option<Option<String>>,
    #[schema(value_type = Option<ValidationRuleTimingSchema>)]
    pub applies_on: Option<String>,
    pub is_active: Option<bool>,
}

impl UpdateValidationRuleRequest {
    pub fn into_command(
        self,
        ip: Option<String>,
        expected_updated_at: Option<DateTime<Utc>>,
    ) -> UpdateValidationRuleCommand {
        UpdateValidationRuleCommand {
            name: self.name,
            expression: self.expression,
            message: self.message,
            field: self.field,
            applies_on: self.applies_on,
            is_active: self.is_active,
            expected_updated_at,
            ip,
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ReorderValidationRulesRequest {
    #[schema(example = "deals.deal")]
    pub record_type: String,
    pub ids: Vec<String>,
}

impl ReorderValidationRulesRequest {
    pub fn into_command(self, ip: Option<String>) -> ReorderValidationRulesCommand {
        ReorderValidationRulesCommand {
            record_type: self.record_type,
            ids: self.ids,
            ip,
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ValidationRulePreviewRequest {
    #[schema(example = "deals.deal")]
    pub record_type: String,
    #[schema(example = "amount > 10000 and is_blank(source)")]
    pub expression: String,
    pub record_id: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ValidationRulePreviewResponse {
    pub fails: Option<bool>,
    pub formula: FormulaPreviewResponse,
}

impl From<ValidationRulePreview> for ValidationRulePreviewResponse {
    fn from(preview: ValidationRulePreview) -> Self {
        Self {
            fails: preview.matched,
            formula: preview.formula,
        }
    }
}
