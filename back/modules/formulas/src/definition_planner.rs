use crate::application::{
    FormulaNode, FormulaWorkspace, MAX_REPORTED_ERRORS, ValueKind, compile_formula, find_cycle,
};
use aphel_sdk::platform::application::fields::{FieldDefinitionReader, RecordTypeRegistry};
use aphel_sdk::platform::application::shared::{Job, RecordType};
use aphel_sdk::platform::domain::fields::FieldDefinition;
use aphel_sdk::{
    AppError,
    formulas::{FormulaDefinitionIssue, FormulaDefinitionPlanner},
};
pub(crate) struct DefinitionPlanner;
#[async_trait::async_trait]
impl FormulaDefinitionPlanner for DefinitionPlanner {
    async fn projected_issues(
        &self,
        registry: &RecordTypeRegistry,
        definitions: &dyn FieldDefinitionReader,
        projected: &[(usize, FieldDefinition)],
    ) -> Result<Vec<FormulaDefinitionIssue>, AppError> {
        let mut workspace = FormulaWorkspace::new(registry, definitions);
        for (_, definition) in projected {
            workspace = workspace.with_override(definition.clone());
        }
        let mut issues = Vec::new();
        for (index, definition) in projected {
            let Some(settings) = definition.formula() else {
                continue;
            };
            let issue = |property: &str, code: &str, message: String| FormulaDefinitionIssue {
                index: *index,
                property: property.into(),
                code: code.into(),
                message,
            };
            let record_type = RecordType::parse(definition.record_type.clone())?;
            let context = workspace.context(&record_type).await?;
            let compiled = match compile_formula(&settings.expression, &context) {
                Ok(compiled) => compiled,
                Err(errors) => {
                    issues.extend(
                        errors
                            .iter()
                            .take(MAX_REPORTED_ERRORS)
                            .map(|error| issue("expression", "invalid_formula", error.describe())),
                    );
                    continue;
                }
            };
            if !ValueKind::fits(compiled.kind, settings.result_type) {
                issues.push(issue(
                    "result_type",
                    "invalid_formula",
                    format!(
                        "the expression results in {} and cannot be stored as {}",
                        compiled.kind.map_or("null", |kind| kind.as_str()),
                        settings.result_type.as_str()
                    ),
                ));
                continue;
            }
            let start = FormulaNode {
                record_type,
                key: definition.key.as_str().to_owned(),
            };
            if let Some(cycle) = find_cycle(&mut workspace, start).await? {
                issues.push(issue(
                    "expression",
                    "formula_cycle",
                    format!(
                        "the formula depends on itself: {}",
                        cycle
                            .iter()
                            .map(FormulaNode::label)
                            .collect::<Vec<_>>()
                            .join(" → ")
                    ),
                ));
            }
        }
        Ok(issues)
    }
    fn recalculate_job(&self, record_type: &RecordType) -> Job {
        crate::application::recalculate_job(record_type, true, 0)
    }
}
