use crate::application::{
    FORMULA_CYCLE, FormulaNode, FormulaWorkspace, ValueKind, compile_formula, find_cycle,
};
use aphel_kernel::errors::{AppError, FieldViolation};
use aphel_sdk::platform::application::fields::{FieldDefinitionReader, RecordTypeRegistry};
use aphel_sdk::platform::application::shared::RecordType;
use aphel_sdk::platform::domain::fields::FieldDefinition;

pub const MAX_REPORTED_ERRORS: usize = 5;
pub const EXPRESSION_PATH: &str = "config.expression";

pub fn cycle_error(path: &[FormulaNode]) -> AppError {
    let description = path
        .iter()
        .map(FormulaNode::label)
        .collect::<Vec<_>>()
        .join(" → ");
    AppError::invalid_fields_code(
        FORMULA_CYCLE,
        "the formula depends on itself through other formulas",
        vec![FieldViolation::new(EXPRESSION_PATH, description)],
    )
}

pub async fn formula_definition_violations<D>(
    registry: &RecordTypeRegistry,
    definitions: &D,
    field: &FieldDefinition,
) -> Result<Vec<FieldViolation>, AppError>
where
    D: FieldDefinitionReader + ?Sized,
{
    let Some(settings) = field.formula() else {
        return Ok(Vec::new());
    };
    let record_type = RecordType::parse(field.record_type.clone())?;
    let mut workspace = FormulaWorkspace::new(registry, definitions).with_override(field.clone());
    let context = workspace.context(&record_type).await?;
    let compiled = match compile_formula(&settings.expression, &context) {
        Ok(compiled) => compiled,
        Err(errors) => {
            return Ok(errors
                .iter()
                .take(MAX_REPORTED_ERRORS)
                .map(|error| FieldViolation::new(EXPRESSION_PATH, error.describe()))
                .collect());
        }
    };
    if !ValueKind::fits(compiled.kind, settings.result_type) {
        return Ok(vec![FieldViolation::new(
            "config.result_type",
            format!(
                "the expression results in {} and cannot be stored as {}",
                compiled.kind.map_or("null", |kind| kind.as_str()),
                settings.result_type.as_str()
            ),
        )]);
    }
    let start = FormulaNode {
        record_type,
        key: field.key.as_str().to_owned(),
    };
    if let Some(path) = find_cycle(&mut workspace, start).await? {
        return Err(cycle_error(&path));
    }
    Ok(Vec::new())
}
