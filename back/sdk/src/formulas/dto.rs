use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;

#[derive(Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[schema(as = FormulaResultType)]
pub enum FormulaResultTypeSchema {
    Number,
    Currency,
    Date,
    Text,
    Boolean,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[schema(as = FormulaValueKind)]
pub enum FormulaValueKindSchema {
    Number,
    Text,
    Date,
    Boolean,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct FormulaErrorResponse {
    #[schema(example = "unknown_field")]
    pub code: String,
    pub message: String,
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct FormulaAggregateResponse {
    #[schema(example = "sum")]
    pub function: String,
    #[schema(example = "items")]
    pub relation: String,
    #[schema(example = "total")]
    pub field: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct FormulaDependenciesResponse {
    pub fields: Vec<String>,
    pub aggregates: Vec<FormulaAggregateResponse>,
    pub uses_today: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct FormulaPreviewResponse {
    pub valid: bool,
    pub errors: Vec<FormulaErrorResponse>,
    #[schema(value_type = Option<FormulaValueKindSchema>)]
    pub kind: Option<String>,
    pub dependencies: FormulaDependenciesResponse,
    pub evaluated: bool,
    #[schema(value_type = Option<Object>)]
    pub value: Option<Value>,
}
