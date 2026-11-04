use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{Map, Value, json};
use uuid::Uuid;

use crate::application::sections::custom_objects::package_registry;
use crate::application::{
    ApplyContext, ConfigPackage, ConfigSection, Decisions, PackageIssue, PlanChange, PlanItem,
};
use ::formulas::application::{
    FormulaNode, FormulaWorkspace, MAX_REPORTED_ERRORS, ValueKind, compile_formula, find_cycle,
    recalculate_job,
};
use aphel_kernel::application::fields::{
    CustomFieldValueRepository, EntityDefinition, FieldDefinitionRepository, MAX_CUSTOM_FIELDS,
    RecordTypeRegistry, config_to_json, field_definition_violations,
};
use aphel_kernel::application::shared::{JobQueue, RecordType};
use aphel_kernel::domain::access::AccessScope;
use aphel_kernel::domain::fields::{FieldDefinition, FieldOption};
use aphel_kernel::errors::AppError;

pub use aphel_sdk::configuration::sections::fields::*;
const FIELD_KIND: &str = "field";

enum FieldOperation {
    Create(FieldDefinition),
    Update {
        updated: FieldDefinition,
        new_options: Vec<FieldOption>,
        changed_options: Vec<FieldOption>,
    },
    Nothing,
}

pub struct FieldsSection<D, V, J> {
    registry: Arc<RecordTypeRegistry>,
    definitions: D,
    values: V,
    jobs: J,
}

impl<D, V, J> FieldsSection<D, V, J>
where
    D: FieldDefinitionRepository,
    V: CustomFieldValueRepository,
{
    pub fn new(registry: Arc<RecordTypeRegistry>, definitions: D, values: V, jobs: J) -> Self {
        Self {
            registry,
            definitions,
            values,
            jobs,
        }
    }

    async fn formula_issues(
        &self,
        registry: &RecordTypeRegistry,
        fields: &[PackageField],
        existing: &BTreeMap<String, Vec<FieldDefinition>>,
    ) -> Result<Vec<PackageIssue>, AppError> {
        let mut projected = Vec::new();
        for (index, field) in fields.iter().enumerate() {
            let id = existing
                .get(&field.record_type)
                .and_then(|definitions| {
                    definitions
                        .iter()
                        .find(|definition| definition.key.as_str() == field.key)
                })
                .map_or_else(Uuid::now_v7, |definition| definition.id);
            if let Ok(definition) = field_definition_from_package(field, id, 0) {
                projected.push((index, definition));
            }
        }
        let mut workspace = FormulaWorkspace::new(registry, &self.definitions);
        for (_, definition) in &projected {
            workspace = workspace.with_override(definition.clone());
        }
        let mut issues = Vec::new();
        for (index, definition) in &projected {
            let Some(settings) = definition.formula() else {
                continue;
            };
            let path = |property: &str| format!("{index}.config.{property}");
            let record_type = RecordType::parse(definition.record_type.clone())?;
            let context = workspace.context(&record_type).await?;
            let compiled = match compile_formula(&settings.expression, &context) {
                Ok(compiled) => compiled,
                Err(errors) => {
                    issues.extend(errors.iter().take(MAX_REPORTED_ERRORS).map(|error| {
                        PackageIssue::new(path("expression"), "invalid_formula", error.describe())
                    }));
                    continue;
                }
            };
            if !ValueKind::fits(compiled.kind, settings.result_type) {
                issues.push(PackageIssue::new(
                    path("result_type"),
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
                issues.push(PackageIssue::new(
                    path("expression"),
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

    fn record_types(&self) -> Vec<RecordType> {
        let mut types: Vec<RecordType> = self
            .registry
            .definitions()
            .map(|definition| definition.record_type.clone())
            .collect();
        types.sort();
        types
    }

    async fn reconcile(
        &self,
        registry: &RecordTypeRegistry,
        fields: &[PackageField],
    ) -> Result<Vec<(PlanItem, FieldOperation)>, AppError> {
        let mut existing: BTreeMap<String, Vec<FieldDefinition>> = BTreeMap::new();
        let mut positions: BTreeMap<String, i32> = BTreeMap::new();
        let mut operations = Vec::new();
        for field in fields {
            let record_type = registry.require(&field.record_type)?;
            if !existing.contains_key(&field.record_type) {
                let definitions = self.definitions.list(&record_type.record_type).await?;
                positions.insert(
                    field.record_type.clone(),
                    definitions
                        .iter()
                        .map(|definition| definition.position)
                        .max()
                        .unwrap_or(-1),
                );
                existing.insert(field.record_type.clone(), definitions);
            }
            let position = positions
                .get_mut(&field.record_type)
                .expect("positions are loaded with the definitions");
            let item = PlanItem::new(
                FIELDS_SECTION,
                FIELD_KIND,
                format!("{}.{}", field.record_type, field.key),
                field.label.clone(),
            );
            let current = existing
                .get(&field.record_type)
                .and_then(|definitions| {
                    definitions
                        .iter()
                        .find(|definition| definition.key.as_str() == field.key)
                })
                .cloned();
            let proposed = field_definition_from_package(field, Uuid::now_v7(), *position + 1)
                .map_err(|_| AppError::Validation(format!("invalid field {}", field.key)))?;
            let Some(current) = current else {
                *position += 1;
                let mut changes: Vec<PlanChange> = field_snapshot(&proposed)
                    .into_iter()
                    .map(|(property, value)| PlanChange::new(property, Value::Null, value))
                    .collect();
                changes.extend(proposed.options.iter().map(|option| {
                    PlanChange::new(
                        format!("options.{}", option.value),
                        Value::Null,
                        option_snapshot(option),
                    )
                }));
                operations.push((item.created(changes), FieldOperation::Create(proposed)));
                continue;
            };
            operations.push(self.compare(record_type, item, current, proposed).await?);
        }
        Ok(operations)
    }

    async fn compare(
        &self,
        record_type: &EntityDefinition,
        item: PlanItem,
        current: FieldDefinition,
        proposed: FieldDefinition,
    ) -> Result<(PlanItem, FieldOperation), AppError> {
        let mut updated = current.clone();
        updated.label = proposed.label.clone();
        updated.field_type = proposed.field_type;
        updated.is_required = proposed.is_required;
        updated.is_unique = proposed.is_unique;
        updated.is_searchable = proposed.is_searchable;
        updated.is_active = true;
        updated.config = proposed.config.clone();
        let mut changes = diff_snapshots(&field_snapshot(&current), &field_snapshot(&updated));

        let mut next_position = current
            .options
            .iter()
            .map(|option| option.position)
            .max()
            .unwrap_or(-1);
        let mut new_options = Vec::new();
        let mut changed_options = Vec::new();
        for option in &proposed.options {
            match current.option_by_value(&option.value) {
                None => {
                    next_position += 1;
                    changes.push(PlanChange::new(
                        format!("options.{}", option.value),
                        Value::Null,
                        option_snapshot(option),
                    ));
                    new_options.push(FieldOption {
                        position: next_position,
                        ..option.clone()
                    });
                }
                Some(existing) => {
                    let changed = FieldOption {
                        label: option.label.clone(),
                        color: option.color.clone(),
                        is_active: true,
                        ..existing.clone()
                    };
                    if &changed != existing {
                        changes.push(PlanChange::new(
                            format!("options.{}", option.value),
                            option_snapshot(existing),
                            option_snapshot(&changed),
                        ));
                        changed_options.push(changed);
                    }
                }
            }
        }

        let operation = FieldOperation::Update {
            updated: updated.clone(),
            new_options,
            changed_options,
        };
        if current.field_type != proposed.field_type {
            let resolvable = current.field_type.can_change_to(proposed.field_type);
            let message = format!(
                "the organization already has this field as {} and the package defines it as {}",
                current.field_type.as_str(),
                proposed.field_type.as_str()
            );
            return Ok((
                item.conflicting(changes, FIELD_TYPE_MISMATCH, message, resolvable),
                operation,
            ));
        }
        if updated.is_unique
            && !current.is_unique
            && self.values.has_duplicates(record_type, &updated).await?
        {
            return Ok((
                item.conflicting(
                    changes,
                    FIELD_DUPLICATE_VALUES,
                    "existing records share values of this field, so it cannot become unique",
                    false,
                ),
                operation,
            ));
        }
        if changes.is_empty() {
            return Ok((item.compared(changes), FieldOperation::Nothing));
        }
        Ok((item.compared(changes), operation))
    }
}

#[async_trait]
impl<D, V, J, Tx> ConfigSection<Tx> for FieldsSection<D, V, J>
where
    Tx: Send,
    D: FieldDefinitionRepository<Tx = Tx>,
    V: CustomFieldValueRepository<Tx = Tx>,
    J: JobQueue<Tx>,
{
    fn key(&self) -> &'static str {
        FIELDS_SECTION
    }

    async fn export(&self, _scope: &AccessScope) -> Result<Option<Value>, AppError> {
        let mut fields = Vec::new();
        for record_type in self.record_types() {
            let mut definitions: Vec<FieldDefinition> = self
                .definitions
                .list(&record_type)
                .await?
                .into_iter()
                .filter(|definition| !definition.is_system && definition.is_active)
                .collect();
            definitions.sort_by_key(|definition| (definition.position, definition.id));
            fields.extend(definitions.iter().map(export_field));
        }
        Ok(Some(
            serde_json::to_value(fields).map_err(|error| AppError::Internal(error.to_string()))?,
        ))
    }

    async fn validate(
        &self,
        _scope: &AccessScope,
        package: &ConfigPackage,
        content: &Value,
    ) -> Result<Vec<PackageIssue>, AppError> {
        let fields = match parse_fields(content) {
            Ok(fields) => fields,
            Err(issue) => return Ok(vec![issue]),
        };
        let registry = package_registry(&self.registry, package);
        let mut issues = Vec::new();
        let mut seen = BTreeSet::new();
        let mut created: BTreeMap<String, usize> = BTreeMap::new();
        let mut existing: BTreeMap<String, Vec<FieldDefinition>> = BTreeMap::new();
        for (index, field) in fields.iter().enumerate() {
            let path = index.to_string();
            let Ok(record_type) = registry.require(&field.record_type) else {
                issues.push(PackageIssue::new(
                    format!("{path}.record_type"),
                    "unknown_record_type",
                    "is not a registered record type",
                ));
                continue;
            };
            if !existing.contains_key(&field.record_type) {
                existing.insert(
                    field.record_type.clone(),
                    self.definitions.list(&record_type.record_type).await?,
                );
            }
            if !seen.insert((field.record_type.clone(), field.key.clone())) {
                issues.push(PackageIssue::new(
                    format!("{path}.key"),
                    "duplicated_key",
                    "is declared more than once for this record type",
                ));
            }
            if record_type.system_field_by_key(&field.key).is_some() {
                issues.push(PackageIssue::new(
                    format!("{path}.key"),
                    "reserved_key",
                    "is reserved by a system field",
                ));
                continue;
            }
            match field_definition_from_package(field, Uuid::now_v7(), 0) {
                Err(field_issues) => {
                    issues.extend(field_issues.into_iter().map(|issue| issue.under(&path)))
                }
                Ok(definition) => {
                    issues.extend(
                        field_definition_violations(&registry, &definition)
                            .into_iter()
                            .map(|violation| {
                                PackageIssue::new(
                                    format!("{path}.{}", package_property(&violation.field)),
                                    "invalid_field",
                                    violation.message,
                                )
                            }),
                    );
                }
            }
            let is_new = existing.get(&field.record_type).is_none_or(|definitions| {
                !definitions
                    .iter()
                    .any(|definition| definition.key.as_str() == field.key)
            });
            if is_new {
                *created.entry(field.record_type.clone()).or_default() += 1;
            }
        }
        if issues.is_empty() {
            issues.extend(self.formula_issues(&registry, &fields, &existing).await?);
        }
        for (record_type, count) in created {
            let current = existing.get(&record_type).map_or(0, |definitions| {
                definitions
                    .iter()
                    .filter(|definition| !definition.is_system)
                    .count()
            });
            if current + count > MAX_CUSTOM_FIELDS {
                issues.push(PackageIssue::new(
                    "",
                    "too_many_fields",
                    format!("{record_type} would have more than {MAX_CUSTOM_FIELDS} custom fields"),
                ));
            }
        }
        Ok(issues)
    }

    async fn plan(
        &self,
        _scope: &AccessScope,
        package: &ConfigPackage,
        content: &Value,
    ) -> Result<Vec<PlanItem>, AppError> {
        let fields = parse_fields(content).map_err(|issue| AppError::Validation(issue.message))?;
        let registry = package_registry(&self.registry, package);
        Ok(self
            .reconcile(&registry, &fields)
            .await?
            .into_iter()
            .map(|(item, _)| item)
            .collect())
    }

    async fn apply(
        &self,
        tx: &mut Tx,
        _scope: &AccessScope,
        package: &ConfigPackage,
        content: &Value,
        decisions: &Decisions,
        _context: &mut ApplyContext,
    ) -> Result<(), AppError> {
        let fields = parse_fields(content).map_err(|issue| AppError::Validation(issue.message))?;
        let registry = package_registry(&self.registry, package);
        let mut recalculate: BTreeSet<String> = BTreeSet::new();
        for (item, operation) in self.reconcile(&registry, &fields).await? {
            if !decisions.executes(&item) {
                continue;
            }
            match operation {
                FieldOperation::Create(field) => {
                    if field.is_formula() {
                        recalculate.insert(field.record_type.clone());
                    }
                    self.definitions.insert(tx, &field).await?;
                }
                FieldOperation::Update {
                    updated,
                    new_options,
                    changed_options,
                } => {
                    if updated.is_formula() {
                        recalculate.insert(updated.record_type.clone());
                    }
                    self.definitions.update(tx, &updated).await?;
                    if updated.field_type.supports_unique() {
                        self.values
                            .rebuild_unique(tx, registry.require(&updated.record_type)?, &updated)
                            .await?;
                    }
                    for option in &new_options {
                        self.definitions
                            .insert_option(tx, updated.id, option)
                            .await?;
                    }
                    for option in &changed_options {
                        self.definitions
                            .update_option(tx, updated.id, option)
                            .await?;
                    }
                }
                FieldOperation::Nothing => {}
            }
        }
        for record_type in recalculate {
            self.jobs
                .enqueue(
                    tx,
                    recalculate_job(&RecordType::parse(record_type.as_str())?, true, 0),
                )
                .await?;
        }
        Ok(())
    }
}

fn package_property(property: &str) -> &str {
    match property {
        "is_unique" => "unique",
        "is_searchable" => "searchable",
        "is_required" => "required",
        other => other,
    }
}

fn field_snapshot(field: &FieldDefinition) -> Vec<(String, Value)> {
    vec![
        ("label".to_owned(), json!(field.label)),
        ("type".to_owned(), json!(field.field_type.as_str())),
        ("required".to_owned(), json!(field.is_required)),
        ("unique".to_owned(), json!(field.is_unique)),
        ("searchable".to_owned(), json!(field.is_searchable)),
        ("active".to_owned(), json!(field.is_active)),
        ("config".to_owned(), config_to_json(&field.config)),
    ]
}

fn diff_snapshots(current: &[(String, Value)], proposed: &[(String, Value)]) -> Vec<PlanChange> {
    let current: Map<String, Value> = current.iter().cloned().collect();
    proposed
        .iter()
        .filter_map(|(property, value)| {
            let old = current.get(property).cloned().unwrap_or(Value::Null);
            (&old != value).then(|| PlanChange::new(property.clone(), old, value.clone()))
        })
        .collect()
}

fn option_snapshot(option: &FieldOption) -> Value {
    json!({
        "label": option.label,
        "color": option.color,
        "active": option.is_active,
    })
}
