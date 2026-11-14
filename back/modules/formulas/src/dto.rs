pub use aphel_sdk::formulas::dto::{
    FormulaAggregateResponse, FormulaDependenciesResponse, FormulaErrorResponse,
    FormulaPreviewResponse, FormulaResultTypeSchema, FormulaValueKindSchema,
};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::application::{FormulaCatalog, FormulaError, FormulaPreview, PreviewFormulaCommand};

#[derive(Debug, Deserialize, ToSchema)]
pub struct FormulaPreviewRequest {
    #[schema(example = "deals.deal")]
    pub record_type: String,
    #[schema(example = "days_until(expected_close_date)")]
    pub expression: String,
    #[schema(value_type = FormulaResultTypeSchema)]
    pub result_type: String,
    pub decimals: Option<u32>,
    pub record_id: Option<String>,
    #[schema(example = "days_to_close")]
    pub field_key: Option<String>,
}

impl From<FormulaPreviewRequest> for PreviewFormulaCommand {
    fn from(request: FormulaPreviewRequest) -> Self {
        Self {
            record_type: request.record_type,
            expression: request.expression,
            result_type: request.result_type,
            decimals: request.decimals,
            record_id: request.record_id,
            field_key: request.field_key,
        }
    }
}

impl From<FormulaError> for FormulaErrorResponse {
    fn from(error: FormulaError) -> Self {
        Self {
            code: error.code.to_owned(),
            message: error.message,
            start: error.span.start,
            end: error.span.end,
        }
    }
}

impl From<FormulaPreview> for FormulaPreviewResponse {
    fn from(preview: FormulaPreview) -> Self {
        Self {
            valid: preview.errors.is_empty(),
            errors: preview.errors.into_iter().map(Into::into).collect(),
            kind: preview.kind.map(|kind| kind.as_str().to_owned()),
            dependencies: FormulaDependenciesResponse {
                fields: preview.dependencies.fields.into_iter().collect(),
                aggregates: preview
                    .dependencies
                    .aggregates
                    .into_iter()
                    .map(|aggregate| FormulaAggregateResponse {
                        function: aggregate.function.as_str().to_owned(),
                        relation: aggregate.relation,
                        field: aggregate.field,
                    })
                    .collect(),
                uses_today: preview.dependencies.uses_today,
            },
            evaluated: preview.evaluated,
            value: preview.value,
        }
    }
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct FormulaCatalogParams {
    pub record_type: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct FormulaCatalogFieldResponse {
    pub key: String,
    pub label: String,
    #[schema(value_type = FormulaValueKindSchema)]
    pub kind: String,
    pub is_formula: bool,
    pub is_currency: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct FormulaRelatedFieldResponse {
    pub key: String,
    pub label: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct FormulaCatalogRelationResponse {
    pub key: String,
    pub label: String,
    pub target: Option<String>,
    pub fields: Vec<FormulaRelatedFieldResponse>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct FormulaFunctionResponse {
    #[schema(example = "days_until")]
    pub name: String,
    #[schema(example = "days_until(date)")]
    pub signature: String,
    #[schema(example = "date")]
    pub category: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct FormulaCatalogResponse {
    pub record_type: String,
    pub fields: Vec<FormulaCatalogFieldResponse>,
    pub relations: Vec<FormulaCatalogRelationResponse>,
    pub functions: Vec<FormulaFunctionResponse>,
}

impl From<FormulaCatalog> for FormulaCatalogResponse {
    fn from(catalog: FormulaCatalog) -> Self {
        Self {
            record_type: catalog.record_type,
            fields: catalog
                .fields
                .into_iter()
                .map(|field| FormulaCatalogFieldResponse {
                    key: field.key,
                    label: field.label,
                    kind: field.kind.as_str().to_owned(),
                    is_formula: field.is_formula,
                    is_currency: field.is_currency,
                })
                .collect(),
            relations: catalog
                .relations
                .into_iter()
                .map(|relation| FormulaCatalogRelationResponse {
                    key: relation.key,
                    label: relation.label,
                    target: relation.target,
                    fields: relation
                        .fields
                        .into_iter()
                        .map(|(key, label)| FormulaRelatedFieldResponse { key, label })
                        .collect(),
                })
                .collect(),
            functions: catalog
                .functions
                .into_iter()
                .map(|function| FormulaFunctionResponse {
                    name: function.name.to_owned(),
                    signature: function.signature.to_owned(),
                    category: function.category.to_owned(),
                })
                .collect(),
        }
    }
}
