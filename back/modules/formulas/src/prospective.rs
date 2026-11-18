use crate::application::{
    AggregateColumn, AggregateQuery, FormulaSource, FormulaUseCases, FormulaWorkspace,
    MAX_REPORTED_ERRORS, TypeFormulas, local_today,
};
use crate::infrastructure::{MySqlFormulaPorts, MySqlFormulaSource};
use crate::prospective_values::{
    CompiledRule, ProposedRecord, aggregate_value, compile_rule, json_formula_value,
    rule_dependencies, rule_preview, stored_formula_value, violation_path,
};
use aphel_sdk::platform::application::fields::{
    FieldDefinitionReader, RecordTypeRegistry, SystemField, WriteMode,
};
use aphel_sdk::platform::application::shared::RecordType;
use aphel_sdk::platform::database::Db;
use aphel_sdk::platform::database::transaction::Tx;
use aphel_sdk::platform::domain::access::AccessScope;
use aphel_sdk::platform::domain::fields::FieldDefinition;
use aphel_sdk::platform::infrastructure::fields::MySqlFieldDefinitionRepository;
use aphel_sdk::{
    AppError,
    formulas::{
        BooleanFormulaPreview, PredicateCheck, PredicateOutcome, PreviewFormulaCommand,
        ProposedFieldValues, ProspectivePredicates, ProspectiveRequest,
    },
};
use std::collections::HashMap;
use std::sync::Arc;

pub(crate) struct Predicates;

#[async_trait::async_trait]
impl ProspectivePredicates for Predicates {
    async fn check(
        &self,
        registry: &RecordTypeRegistry,
        definitions: &dyn FieldDefinitionReader,
        record_type: &RecordType,
        expression: &str,
        field: Option<&str>,
    ) -> Result<PredicateCheck, AppError> {
        let context = FormulaWorkspace::new(registry, definitions)
            .context(record_type)
            .await?;
        Ok(PredicateCheck {
            errors: if expression.chars().count()
                > aphel_sdk::platform::domain::fields::MAX_FORMULA_LENGTH
            {
                Vec::new()
            } else {
                compile_rule(expression, &context)
                    .err()
                    .unwrap_or_default()
                    .iter()
                    .take(MAX_REPORTED_ERRORS)
                    .map(|error| error.describe())
                    .collect()
            },
            field_exists: field.is_none_or(|key| context.field(key).is_some()),
        })
    }
    async fn evaluate(
        &self,
        request: ProspectiveRequest<'_, '_>,
        tx: &mut Tx,
        values_source: &dyn ProposedFieldValues<Tx = Tx>,
    ) -> Result<Vec<PredicateOutcome>, AppError> {
        let ProspectiveRequest {
            pool,
            registry,
            scope,
            write,
            predicates,
        } = request;
        let record_type = &write.record.record_type;
        let Some(definition) = registry.get(record_type) else {
            return Ok(Vec::new());
        };
        let definitions = MySqlFieldDefinitionRepository::new(pool.clone());
        let source = MySqlFormulaSource::new(pool.clone());
        let context = FormulaWorkspace::new(registry, &definitions)
            .context(record_type)
            .await?;
        let mut outcomes = Vec::new();
        let compiled: Vec<CompiledRule<'_>> = predicates
            .iter()
            .filter_map(|rule| match compile_rule(&rule.expression, &context) {
                Ok(compiled) => Some(CompiledRule { rule, compiled }),
                Err(errors) => {
                    outcomes.push(PredicateOutcome {
                        id: rule.id,
                        matched: None,
                        path: violation_path(&context, rule.field.as_deref()),
                        error: errors.first().map(|error| error.describe()),
                    });
                    None
                }
            })
            .collect();
        if compiled.is_empty() {
            return Ok(outcomes);
        }
        let formulas = TypeFormulas::new(context);
        let context = &formulas.context;
        let (dependencies, aggregates) = rule_dependencies(&formulas, &compiled);
        let timezone = source.timezone().await?;
        let record_id = write.record.record_id;

        let mut values = HashMap::new();
        let mut missing_system: Vec<SystemField> = Vec::new();
        let mut custom: Vec<FieldDefinition> = Vec::new();
        for key in &dependencies {
            let Some(field) = context.field(key) else {
                continue;
            };
            if field.is_formula() {
                continue;
            }
            match &field.field.system {
                Some(system) => match write.values.get(key.as_str()) {
                    Some(value) => {
                        values.insert(
                            key.clone(),
                            json_formula_value(field.definition(), value, timezone),
                        );
                    }
                    None => missing_system.push(system.clone()),
                },
                None => custom.push(field.definition().clone()),
            }
        }
        if write.mode == WriteMode::Update && !missing_system.is_empty() {
            let mut rows = source
                .system_values(scope, definition, &missing_system, &[record_id])
                .await?;
            if let Some(row) = rows.remove(&record_id) {
                for (key, value) in row {
                    if let Some(field) = context.field(&key) {
                        let value = stored_formula_value(field.definition(), &value, timezone);
                        values.insert(key, value);
                    }
                }
            }
        }
        if !custom.is_empty() {
            let stored = values_source.load(tx, write.record, &custom).await?;
            for field in &custom {
                if let Some(value) = stored.get(&field.id) {
                    values.insert(
                        field.key.as_str().to_owned(),
                        stored_formula_value(field, value, timezone),
                    );
                }
            }
        }

        let mut record_aggregates = HashMap::new();
        for aggregate in &aggregates {
            let mut value = None;
            if write.mode == WriteMode::Update
                && let Some(relation) = context.relation(&aggregate.relation)
            {
                let column: Option<&AggregateColumn> = match &aggregate.field {
                    Some(field) => relation.field(field).map(|related| &related.column),
                    None => None,
                };
                if aggregate.field.is_none() || column.is_some() {
                    let query = AggregateQuery {
                        source: &relation.source,
                        function: aggregate.function,
                        column,
                    };
                    value = source
                        .aggregate(scope, &query, &[record_id])
                        .await?
                        .remove(&record_id);
                }
            }
            record_aggregates.insert(aggregate.clone(), aggregate_value(aggregate, value));
        }

        let mut proposed = ProposedRecord {
            values,
            aggregates: record_aggregates,
            today: local_today(timezone),
        };
        proposed.compute_formulas(&formulas);

        outcomes.extend(compiled.iter().map(|rule| PredicateOutcome {
            id: rule.rule.id,
            matched: Some(proposed.fails(&rule.compiled)),
            path: violation_path(context, rule.rule.field.as_deref()),
            error: None,
        }));
        Ok(outcomes)
    }
    async fn preview(
        &self,
        pool: &Db,
        registry: &Arc<RecordTypeRegistry>,
        scope: &AccessScope,
        mut command: PreviewFormulaCommand,
    ) -> Result<BooleanFormulaPreview, AppError> {
        command.result_type = "boolean".to_owned();
        let expression = command.expression.clone();
        let formula = FormulaUseCases::new(MySqlFormulaPorts::new(pool.clone(), registry.clone()))
            .preview(scope, command)
            .await?;
        let preview = rule_preview(&expression, formula);
        Ok(BooleanFormulaPreview {
            formula: preview.formula.into(),
            matched: preview.fails,
        })
    }
}
