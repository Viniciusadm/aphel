use std::collections::HashMap;
use std::sync::Arc;

use crate::application::{
    CompiledRule, ProposedFieldValues, ProposedRecord, VALIDATION_RULE_FAILED,
    ValidationRuleRepository, aggregate_value, compile_rule, is_validated_record_type,
    json_formula_value, rule_dependencies, stored_formula_value, violation_path,
};
use ::formulas::application::{
    AggregateColumn, AggregateQuery, FormulaSource, FormulaWorkspace, TypeFormulas, local_today,
};
use aphel_kernel::application::fields::{
    FieldDefinitionRepository, RecordTypeRegistry, RecordWrite, SystemField, WriteMode,
};
use aphel_kernel::domain::access::AccessScope;
use aphel_kernel::domain::fields::FieldDefinition;
use aphel_kernel::errors::{AppError, FieldViolation};

pub struct ValidationRuleGuard<R, D, S, V> {
    registry: Arc<RecordTypeRegistry>,
    rules: R,
    definitions: D,
    source: S,
    values: V,
}

impl<R, D, S, V> ValidationRuleGuard<R, D, S, V> {
    pub fn new(
        registry: Arc<RecordTypeRegistry>,
        rules: R,
        definitions: D,
        source: S,
        values: V,
    ) -> Self {
        Self {
            registry,
            rules,
            definitions,
            source,
            values,
        }
    }
}

impl<R, D, S, V> ValidationRuleGuard<R, D, S, V>
where
    R: ValidationRuleRepository,
    D: FieldDefinitionRepository,
    S: FormulaSource,
    V: ProposedFieldValues,
{
    pub async fn check(
        &self,
        tx: &mut V::Tx,
        scope: &AccessScope,
        write: &RecordWrite<'_>,
    ) -> Result<(), AppError> {
        let record_type = &write.record.record_type;
        if !is_validated_record_type(record_type.as_str()) {
            return Ok(());
        }
        let rules: Vec<_> = self
            .rules
            .active_for(record_type.as_str())
            .await?
            .into_iter()
            .filter(|rule| rule.applies_on.applies(write.mode))
            .collect();
        if rules.is_empty() {
            return Ok(());
        }
        let Some(definition) = self.registry.get(record_type) else {
            return Ok(());
        };
        let mut workspace = FormulaWorkspace::new(&self.registry, &self.definitions);
        let context = workspace.context(record_type).await?;
        let compiled: Vec<CompiledRule<'_>> = rules
            .iter()
            .filter_map(|rule| match compile_rule(&rule.expression, &context) {
                Ok(compiled) => Some(CompiledRule { rule, compiled }),
                Err(errors) => {
                    tracing::warn!(
                        rule = %rule.key,
                        record_type = %record_type.as_str(),
                        error = ?errors.first().map(|error| error.describe()),
                        "validation rule skipped because its condition no longer compiles"
                    );
                    None
                }
            })
            .collect();
        if compiled.is_empty() {
            return Ok(());
        }
        let formulas = TypeFormulas::new(context);
        let context = &formulas.context;
        let (dependencies, aggregates) = rule_dependencies(&formulas, &compiled);
        let timezone = self.source.timezone().await?;
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
            let mut rows = self
                .source
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
            let stored = self.values.load(tx, write.record, &custom).await?;
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
                    value = self
                        .source
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
        let violations: Vec<FieldViolation> = compiled
            .iter()
            .filter(|rule| proposed.fails(&rule.compiled))
            .map(|rule| {
                FieldViolation::new(
                    violation_path(context, rule.rule.field.as_deref()),
                    rule.rule.message.clone(),
                )
            })
            .collect();
        if violations.is_empty() {
            return Ok(());
        }
        Err(AppError::invalid_fields_code(
            VALIDATION_RULE_FAILED,
            "the record breaks validation rules",
            violations,
        ))
    }
}
