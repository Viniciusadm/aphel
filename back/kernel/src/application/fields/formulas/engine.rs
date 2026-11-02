use std::collections::{BTreeSet, HashMap, HashSet, VecDeque};

use chrono::{NaiveDate, Utc};
use chrono_tz::Tz;
use rust_decimal::Decimal;
use uuid::Uuid;

use crate::application::fields::formulas::{
    AggregateColumn, AggregateFunction, AggregateQuery, AggregateRef, AggregateSource,
    CompiledFormula, EvalEnv, FormulaContext, FormulaPorts, FormulaSource, FormulaValue,
    FormulaWorkspace, evaluate, order_formulas,
};
use crate::application::fields::{CustomFieldValueRepository, SystemField};
use crate::application::shared::{RecordRef, RecordType, TransactionManager};
use crate::domain::access::AccessScope;
use crate::domain::fields::{
    FieldDefinition, FieldType, FieldValue, MAX_NUMBER_ABS, MAX_TEXT_LENGTH,
};
use crate::errors::AppError;

pub const MAX_PROPAGATION_DEPTH: usize = 3;
pub const RECALCULATION_BATCH: u32 = 200;
const MAX_WRITE_ATTEMPTS: u64 = 4;
const LOCK_RETRY_DELAY_MS: u64 = 50;
const LOCK_CONFLICTS: [&str; 3] = ["1213 (40001)", "Deadlock found", "Lock wait timeout"];

fn is_lock_conflict(error: &AppError) -> bool {
    matches!(error, AppError::Persistence(message)
        if LOCK_CONFLICTS.iter().any(|marker| message.contains(marker)))
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypeFormulas {
    pub context: FormulaContext,
    pub ordered: Vec<(FieldDefinition, Option<CompiledFormula>)>,
}

impl TypeFormulas {
    pub fn new(context: FormulaContext) -> Self {
        let ordered = order_formulas(&context);
        Self { context, ordered }
    }

    pub fn is_empty(&self) -> bool {
        self.ordered.is_empty()
    }

    pub fn aggregates(&self) -> BTreeSet<&AggregateRef> {
        self.ordered
            .iter()
            .filter_map(|(_, compiled)| compiled.as_ref())
            .flat_map(|compiled| compiled.dependencies.aggregates.iter())
            .collect()
    }

    pub fn aggregating_over(&self, child: &RecordType) -> Vec<&AggregateSource> {
        let relations: BTreeSet<&str> = self
            .aggregates()
            .into_iter()
            .map(|aggregate| aggregate.relation.as_str())
            .collect();
        relations
            .into_iter()
            .filter_map(|key| self.context.relation(key))
            .map(|relation| &relation.source)
            .filter(|source| source.target_type() == Some(child))
            .collect()
    }
}

pub fn local_today(timezone: Tz) -> NaiveDate {
    Utc::now().with_timezone(&timezone).date_naive()
}

pub fn formula_field_value(
    definition: &FieldDefinition,
    value: &FormulaValue,
) -> Option<FieldValue> {
    match (definition.field_type.base_type(), value) {
        (FieldType::Number | FieldType::Currency, FormulaValue::Number(number)) => {
            let decimals = definition.config.allowed_decimals(definition.field_type);
            let rounded = number.round_dp(decimals).normalize();
            (rounded.abs() < Decimal::from(MAX_NUMBER_ABS)).then_some(FieldValue::Number(rounded))
        }
        (FieldType::Date, FormulaValue::Date(date)) => Some(FieldValue::Date(*date)),
        (FieldType::Checkbox, FormulaValue::Bool(value)) => Some(FieldValue::Bool(*value)),
        (FieldType::Text, value) if !value.is_empty() => {
            let text: String = value
                .as_text()
                .trim()
                .chars()
                .take(MAX_TEXT_LENGTH as usize)
                .collect();
            (!text.is_empty()).then_some(FieldValue::Text(text))
        }
        _ => None,
    }
}

fn stored_value(value: &FieldValue) -> FormulaValue {
    match value {
        FieldValue::Number(number) => FormulaValue::Number(*number),
        FieldValue::Date(date) => FormulaValue::Date(*date),
        FieldValue::Bool(value) => FormulaValue::Bool(*value),
        FieldValue::Text(text) | FieldValue::LongText(text) => FormulaValue::Text(text.clone()),
        _ => FormulaValue::Null,
    }
}

fn source_value(definition: &FieldDefinition, value: &FieldValue, timezone: Tz) -> FormulaValue {
    match value {
        FieldValue::Text(text) if definition.field_type.base_type() == FieldType::Select => {
            definition.option_by_value(text).map_or_else(
                || FormulaValue::Text(text.clone()),
                |option| FormulaValue::Text(option.label.clone()),
            )
        }
        FieldValue::Text(text) | FieldValue::LongText(text) => FormulaValue::Text(text.clone()),
        FieldValue::Number(number) => FormulaValue::Number(*number),
        FieldValue::Date(date) => FormulaValue::Date(*date),
        FieldValue::DateTime(instant) => {
            FormulaValue::Date(instant.with_timezone(&timezone).date_naive())
        }
        FieldValue::Bool(value) => FormulaValue::Bool(*value),
        FieldValue::Option(id) => definition
            .option_by_id(*id)
            .map_or(FormulaValue::Null, |option| {
                FormulaValue::Text(option.label.clone())
            }),
        FieldValue::Options(_) | FieldValue::User(_) | FieldValue::Record(_) => FormulaValue::Null,
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct EvaluatedRecord {
    pub results: Vec<(FieldDefinition, Option<FieldValue>)>,
    pub previous: HashMap<uuid::Uuid, FieldValue>,
}

pub struct FormulaEngine<'a, P> {
    ports: &'a P,
    scope: &'a AccessScope,
}

impl<'a, P> FormulaEngine<'a, P>
where
    P: FormulaPorts,
{
    pub fn new(ports: &'a P, scope: &'a AccessScope) -> Self {
        Self { ports, scope }
    }

    pub fn workspace(&self) -> FormulaWorkspace<'a, P::Definitions> {
        FormulaWorkspace::new(self.ports.registry(), self.ports.definitions())
    }

    pub async fn evaluate(
        &self,
        formulas: &TypeFormulas,
        record_ids: &[Uuid],
    ) -> Result<HashMap<Uuid, EvaluatedRecord>, AppError> {
        let context = &formulas.context;
        let record_type = &context.record_type;
        let mut dependencies: BTreeSet<&str> = BTreeSet::new();
        for (_, compiled) in &formulas.ordered {
            if let Some(compiled) = compiled {
                dependencies.extend(compiled.dependencies.fields.iter().map(String::as_str));
            }
        }
        let system_fields: Vec<SystemField> = dependencies
            .iter()
            .filter_map(|key| context.field(key))
            .filter_map(|field| field.field.system.clone())
            .collect();
        let mut custom_fields: Vec<FieldDefinition> = dependencies
            .iter()
            .filter_map(|key| context.field(key))
            .filter(|field| !field.field.is_system() && !field.is_formula())
            .map(|field| field.definition().clone())
            .collect();
        custom_fields.extend(
            formulas
                .ordered
                .iter()
                .filter(|(definition, _)| !definition.id.is_nil())
                .map(|(definition, _)| definition.clone()),
        );

        let source = self.ports.source();
        let timezone = source.timezone().await?;
        let today = local_today(timezone);
        let rows = source
            .system_values(self.scope, record_type, &system_fields, record_ids)
            .await?;
        if rows.is_empty() {
            return Ok(HashMap::new());
        }
        let ids: Vec<Uuid> = record_ids
            .iter()
            .filter(|id| rows.contains_key(id))
            .copied()
            .collect();
        let mut stored: HashMap<Uuid, HashMap<Uuid, FieldValue>> = HashMap::new();
        for value in self
            .ports
            .values()
            .load(&record_type.record_type, &custom_fields, &ids)
            .await?
        {
            stored
                .entry(value.record_id)
                .or_default()
                .insert(value.field_id, value.value);
        }

        let mut aggregates: HashMap<AggregateRef, HashMap<Uuid, Decimal>> = HashMap::new();
        for aggregate in formulas.aggregates() {
            let Some(relation) = context.relation(&aggregate.relation) else {
                continue;
            };
            let column: Option<&AggregateColumn> = match &aggregate.field {
                Some(field) => match relation.field(field) {
                    Some(related) => Some(&related.column),
                    None => continue,
                },
                None => None,
            };
            let query = AggregateQuery {
                source: &relation.source,
                function: aggregate.function,
                column,
            };
            let values = source.aggregate(self.scope, &query, &ids).await?;
            aggregates.insert(aggregate.clone(), values);
        }

        let mut evaluated = HashMap::with_capacity(ids.len());
        for id in ids {
            let mut values: HashMap<String, FormulaValue> = HashMap::new();
            if let Some(row) = rows.get(&id) {
                for (key, value) in row {
                    if let Some(field) = context.field(key) {
                        values.insert(
                            key.clone(),
                            source_value(field.definition(), value, timezone),
                        );
                    }
                }
            }
            let previous = stored.remove(&id).unwrap_or_default();
            for field in &custom_fields {
                if field.is_formula() {
                    continue;
                }
                if let Some(value) = previous.get(&field.id) {
                    values.insert(
                        field.key.as_str().to_owned(),
                        source_value(field, value, timezone),
                    );
                }
            }
            let record_aggregates: HashMap<AggregateRef, FormulaValue> = aggregates
                .iter()
                .map(|(aggregate, by_record)| {
                    let value = match by_record.get(&id) {
                        Some(number) => FormulaValue::Number(*number),
                        None if matches!(
                            aggregate.function,
                            AggregateFunction::Count | AggregateFunction::Sum
                        ) =>
                        {
                            FormulaValue::Number(Decimal::ZERO)
                        }
                        None => FormulaValue::Null,
                    };
                    (aggregate.clone(), value)
                })
                .collect();
            let mut results = Vec::with_capacity(formulas.ordered.len());
            for (definition, compiled) in &formulas.ordered {
                let value = compiled.as_ref().and_then(|compiled| {
                    let env = EvalEnv {
                        values: &values,
                        aggregates: &record_aggregates,
                        today,
                    };
                    formula_field_value(definition, &evaluate(&compiled.expression, &env))
                });
                values.insert(
                    definition.key.as_str().to_owned(),
                    value.as_ref().map_or(FormulaValue::Null, stored_value),
                );
                results.push((definition.clone(), value));
            }
            evaluated.insert(id, EvaluatedRecord { results, previous });
        }
        Ok(evaluated)
    }

    pub async fn recalculate(
        &self,
        formulas: &TypeFormulas,
        record_ids: &[Uuid],
        strict: bool,
    ) -> Result<usize, AppError> {
        if formulas.is_empty() || record_ids.is_empty() {
            return Ok(0);
        }
        let mut attempt: u64 = 1;
        loop {
            match self.write_recalculated(formulas, record_ids, strict).await {
                Err(error) if attempt < MAX_WRITE_ATTEMPTS && is_lock_conflict(&error) => {
                    tracing::debug!(attempt, "formula recalculation retried: {error}");
                    tokio::time::sleep(std::time::Duration::from_millis(
                        LOCK_RETRY_DELAY_MS * attempt,
                    ))
                    .await;
                    attempt += 1;
                }
                outcome => return outcome,
            }
        }
    }

    async fn write_recalculated(
        &self,
        formulas: &TypeFormulas,
        record_ids: &[Uuid],
        strict: bool,
    ) -> Result<usize, AppError> {
        let evaluated = self.evaluate(formulas, record_ids).await?;
        let record_type = &formulas.context.record_type.record_type;
        let mut writes = Vec::new();
        for (id, record) in &evaluated {
            for (definition, value) in &record.results {
                let previous = record.previous.get(&definition.id);
                let changed = match value {
                    Some(value) => previous != Some(value),
                    None => previous.is_some() || strict,
                };
                if changed {
                    writes.push((*id, definition, value.as_ref()));
                }
            }
        }
        if writes.is_empty() {
            return Ok(0);
        }
        writes.sort_by_key(|(id, definition, _)| (*id, definition.id));
        let mut tx = self.ports.transactions().begin().await?;
        for (id, definition, value) in &writes {
            let record = RecordRef::new(record_type.clone(), *id);
            match value {
                Some(value) => {
                    self.ports
                        .values()
                        .upsert(&mut tx, &record, definition, value)
                        .await?
                }
                None => {
                    self.ports
                        .values()
                        .delete(&mut tx, &record, definition.id)
                        .await?
                }
            }
        }
        self.ports.transactions().commit(tx).await?;
        Ok(writes.len())
    }

    pub async fn recalculate_all(
        &self,
        formulas: &TypeFormulas,
        strict: bool,
    ) -> Result<usize, AppError> {
        let definition = &formulas.context.record_type;
        let mut after = None;
        let mut written = 0;
        loop {
            let ids = self
                .ports
                .source()
                .record_ids(self.scope, definition, after, RECALCULATION_BATCH)
                .await?;
            let Some(last) = ids.last().copied() else {
                break;
            };
            written += self.recalculate(formulas, &ids, strict).await?;
            if ids.len() < RECALCULATION_BATCH as usize {
                break;
            }
            after = Some(last);
        }
        Ok(written)
    }

    pub async fn linked_dependents(&self, child: &RecordType) -> Result<Vec<RecordType>, AppError> {
        let mut workspace = self.workspace();
        let mut dependents = Vec::new();
        for formula_type in self.ports.source().formula_record_types().await? {
            if self.ports.registry().get(&formula_type).is_none() {
                continue;
            }
            let formulas = TypeFormulas::new(workspace.context(&formula_type).await?);
            if formulas
                .aggregating_over(child)
                .iter()
                .any(|source| matches!(source, AggregateSource::Relationship { .. }))
            {
                dependents.push(formula_type);
            }
        }
        Ok(dependents)
    }

    pub async fn recalculate_with_dependents(
        &self,
        record_type: &RecordType,
        record_ids: &[Uuid],
        previous_parents: &[(String, Uuid)],
    ) -> Result<(), AppError> {
        let source = self.ports.source();
        let formula_types = source.formula_record_types().await?;
        if formula_types.is_empty() {
            return Ok(());
        }
        let mut workspace = self.workspace();
        let mut loaded: HashMap<RecordType, TypeFormulas> = HashMap::new();
        for formula_type in &formula_types {
            if self.ports.registry().get(formula_type).is_none() {
                continue;
            }
            let context = workspace.context(formula_type).await?;
            loaded.insert(formula_type.clone(), TypeFormulas::new(context));
        }
        let mut visited: HashSet<(RecordType, Uuid)> = HashSet::new();
        let mut queue: VecDeque<(RecordType, Vec<Uuid>, usize)> = VecDeque::new();
        queue.push_back((record_type.clone(), record_ids.to_vec(), 0));
        while let Some((current_type, ids, depth)) = queue.pop_front() {
            let ids: Vec<Uuid> = ids
                .into_iter()
                .filter(|id| visited.insert((current_type.clone(), *id)))
                .collect();
            if ids.is_empty() {
                continue;
            }
            if let Some(formulas) = loaded.get(&current_type) {
                self.recalculate(formulas, &ids, false).await?;
            }
            if depth >= MAX_PROPAGATION_DEPTH {
                continue;
            }
            for (parent_type, formulas) in &loaded {
                for aggregate_source in formulas.aggregating_over(&current_type) {
                    let mut parents = source.parents(aggregate_source, &ids).await?;
                    if depth == 0 {
                        parents.extend(previous_parent_ids(aggregate_source, previous_parents));
                    }
                    if !parents.is_empty() {
                        queue.push_back((parent_type.clone(), parents, depth + 1));
                    }
                }
            }
        }
        Ok(())
    }
}

fn previous_parent_ids(source: &AggregateSource, previous: &[(String, Uuid)]) -> Vec<Uuid> {
    let AggregateSource::HasMany { target, column } = source else {
        return Vec::new();
    };
    previous
        .iter()
        .filter(|(key, _)| {
            target
                .system_field_by_key(key)
                .is_some_and(|field| field.column == *column)
        })
        .map(|(_, id)| *id)
        .collect()
}
