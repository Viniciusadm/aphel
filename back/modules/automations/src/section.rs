use aphel_sdk::configuration::ConfigReferenceKind;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use uuid::Uuid;

use crate::model::{Automation, MAX_AUTOMATIONS, MAX_DESCRIPTION_LENGTH, MAX_NAME_LENGTH};
use crate::service::valid_key;
use crate::store::AutomationStore;
use aphel_contract_automation::application::actions::{ActionRegistry, configured_flag};
use aphel_contract_automation::application::definition::{
    AutomationDefinition, MAX_DEFINITION_BYTES, TriggerSpec, parse_definition, parse_trigger,
};
use aphel_contract_automation::application::triggers::{TriggerRegistry, is_automatable};
use aphel_contract_automation::domain::AutomationStatus;
use aphel_kernel::application::fields::RecordTypeRegistry;
use aphel_kernel::database::transaction::MySqlTx;
use aphel_kernel::domain::access::AccessScope;
use aphel_kernel::errors::AppError;
use aphel_sdk::configuration::sections::common::diff;
use aphel_sdk::configuration::sections::custom_objects::package_registry;
use aphel_sdk::configuration::{
    ApplyContext, ConfigKeyRepository, ConfigPackage, ConfigSection, Decisions, PackageIssue,
    PlanChange, PlanItem, assign_keys, parse_section,
};
use tagging::TagStore;
use tagging::section::{PackageTag, TAGS_SECTION};
use tasks::section::{ACTIVITY_TYPES_SECTION, PackageActivityType};
use tasks::store::ActivityTypeStore;
use workflow::section::{PIPELINES_SECTION, keyed_pipelines, parse_pipelines, stage_item_key};
use workflow::store::PipelineStore;

pub(crate) const AUTOMATIONS_SECTION: &str = "automations";
const AUTOMATION_KIND: &str = "automation";
pub(crate) const MAX_PACKAGE_AUTOMATIONS: usize = 200;
pub(crate) const AUTOMATION_RECORD_TYPE_MISMATCH: &str = "automation_record_type_mismatch";
pub(crate) const NON_PORTABLE_REFERENCE: &str = "non_portable_reference";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum ReferenceKind {
    Tag,
    Stage,
    ActivityType,
    Sequence,
    Segment,
}

impl ReferenceKind {
    fn label(&self) -> &'static str {
        match self {
            Self::Tag => "tag",
            Self::Stage => "stage",
            Self::ActivityType => "activity type",
            Self::Sequence => "sequence",
            Self::Segment => "segment",
        }
    }
}

const ACTION_REFERENCES: [(&str, &str, ReferenceKind); 4] = [
    ("tag_id", "tag", ReferenceKind::Tag),
    ("stage_id", "stage", ReferenceKind::Stage),
    (
        "activity_type_id",
        "activity_type",
        ReferenceKind::ActivityType,
    ),
    ("sequence_id", "sequence", ReferenceKind::Sequence),
];

const TRIGGER_REFERENCES: [(&str, &str, ReferenceKind); 3] = [
    ("from_stage_id", "from_stage", ReferenceKind::Stage),
    ("to_stage_id", "to_stage", ReferenceKind::Stage),
    ("segment_id", "segment", ReferenceKind::Segment),
];

const SEQUENCES_SECTION: &str = "sequences";
const IN_SEGMENT_OPERATOR: &str = "in_saved_filter";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PackageAutomation {
    pub(crate) key: String,
    pub(crate) name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) description: Option<String>,
    pub(crate) record_type: String,
    pub(crate) trigger: Value,
    pub(crate) definition: Value,
}

struct StageEntry {
    id: Uuid,
    pipeline_id: Uuid,
    pipeline_key: String,
    key: String,
}

#[derive(Default)]
struct KeyIndex {
    tags: Vec<(Uuid, String)>,
    task_types: Vec<(Uuid, String)>,
    workflow: Vec<(Uuid, String)>,
    stages: Vec<StageEntry>,
    sequences: Vec<(Uuid, String)>,
    segments: Vec<(Uuid, String)>,
}

impl KeyIndex {
    fn key_of(&self, kind: ReferenceKind, id: &str) -> Option<String> {
        let id = Uuid::parse_str(id.trim()).ok()?;
        match kind {
            ReferenceKind::Tag => find_key(&self.tags, id),
            ReferenceKind::ActivityType => find_key(&self.task_types, id),
            ReferenceKind::Sequence => find_key(&self.sequences, id),
            ReferenceKind::Segment => find_key(&self.segments, id),
            ReferenceKind::Stage => self
                .stages
                .iter()
                .find(|stage| stage.id == id)
                .map(|stage| format!("{}.{}", stage.pipeline_key, stage.key)),
        }
    }

    fn id_of(items: &[(Uuid, String)], key: &str) -> Option<Uuid> {
        items
            .iter()
            .find(|(_, known)| known == key)
            .map(|(id, _)| *id)
    }

    fn pipeline_id(&self, key: &str) -> Option<Uuid> {
        self.workflow
            .iter()
            .find(|(_, known)| known == key)
            .map(|(id, _)| *id)
    }

    fn stage_id(&self, pipeline_id: Uuid, key: &str) -> Option<Uuid> {
        self.stages
            .iter()
            .find(|stage| stage.pipeline_id == pipeline_id && stage.key == key)
            .map(|stage| stage.id)
    }
}

fn find_key(items: &[(Uuid, String)], id: Uuid) -> Option<String> {
    items
        .iter()
        .find(|(known, _)| *known == id)
        .map(|(_, key)| key.clone())
}

#[derive(Debug, Clone, PartialEq)]
struct PortableAutomation {
    name: String,
    description: Option<String>,
    record_type: String,
    trigger: Value,
    definition: Value,
}

impl PortableAutomation {
    fn snapshot(&self) -> Vec<(String, Value)> {
        vec![
            ("name".to_owned(), json!(self.name)),
            ("description".to_owned(), json!(self.description)),
            ("record_type".to_owned(), json!(self.record_type)),
            ("trigger".to_owned(), self.trigger.clone()),
            ("definition".to_owned(), self.definition.clone()),
        ]
    }
}

struct AutomationPlan {
    item: PlanItem,
    key: String,
    proposed: PortableAutomation,
    current: Option<Automation>,
}

pub(crate) struct AutomationsSection<K> {
    registry: Arc<RecordTypeRegistry>,
    automations: AutomationStore,
    tags: TagStore,
    workflow: PipelineStore,
    keys: K,
    task_types: ActivityTypeStore,
    triggers: TriggerRegistry,
    actions: ActionRegistry,
}

impl<K> AutomationsSection<K>
where
    K: ConfigKeyRepository,
{
    pub(crate) fn new(
        registry: Arc<RecordTypeRegistry>,
        automations: AutomationStore,
        tags: TagStore,
        workflow: PipelineStore,
        keys: K,
        task_types: ActivityTypeStore,
    ) -> Self {
        Self {
            registry,
            automations,
            tags,
            workflow,
            keys,
            task_types,
            triggers: TriggerRegistry::builtin(),
            actions: ActionRegistry::builtin(),
        }
    }

    async fn organization_index(&self) -> Result<KeyIndex, AppError> {
        let mut tags = self.tags.list().await?;
        tags.sort_by(|left, right| {
            left.created_at
                .cmp(&right.created_at)
                .then(left.id.cmp(&right.id))
        });
        let stored_tags = self
            .keys
            .keys(ConfigReferenceKind::new("tagging.tag"))
            .await?;
        let tags = assign_keys(
            tags.iter().map(|tag| (tag.id, tag.name.as_str())),
            &stored_tags,
        )
        .into_iter()
        .map(|keyed| (keyed.id, keyed.key))
        .collect();
        let task_types = self
            .task_types
            .list(true)
            .await?
            .into_iter()
            .map(|activity_type| (activity_type.id, activity_type.key))
            .collect();
        let stored_stages = self
            .keys
            .keys(ConfigReferenceKind::new("workflow.stage"))
            .await?;
        let mut workflow = Vec::new();
        let mut stages = Vec::new();
        for (pipeline, keyed) in keyed_pipelines(&self.workflow, &self.keys).await? {
            let workflow_stages = self.workflow.list_stages(pipeline.id).await?;
            let ids: BTreeSet<Uuid> = workflow_stages.iter().map(|stage| stage.id).collect();
            let stored: HashMap<Uuid, String> = stored_stages
                .iter()
                .filter(|(id, _)| ids.contains(id))
                .map(|(id, key)| (*id, key.clone()))
                .collect();
            for stage in assign_keys(
                workflow_stages
                    .iter()
                    .map(|stage| (stage.id, stage.name.as_str())),
                &stored,
            ) {
                stages.push(StageEntry {
                    id: stage.id,
                    pipeline_id: pipeline.id,
                    pipeline_key: keyed.key.clone(),
                    key: stage.key,
                });
            }
            workflow.push((pipeline.id, keyed.key));
        }
        Ok(KeyIndex {
            tags,
            task_types,
            workflow,
            stages,
            sequences: self.automations.sequence_keys().await?,
            segments: self.automations.segment_keys().await?,
        })
    }

    fn secret_fields(&self, action: &str) -> &'static [&'static str] {
        self.actions
            .get(action)
            .map(|action| action.secret_fields())
            .unwrap_or(&[])
    }

    fn portable(&self, automation: &Automation, index: &KeyIndex) -> PortableAutomation {
        let mut trigger = serde_json::to_value(&automation.trigger).unwrap_or(Value::Null);
        let mut definition = serde_json::to_value(&automation.definition).unwrap_or(Value::Null);
        let to_key = |kind: ReferenceKind, id: &str| index.key_of(kind, id);
        rewrite_trigger(&mut trigger, false, &to_key);
        rewrite_segment_conditions(&mut trigger, &to_key, &mut Vec::new());
        rewrite_segment_conditions(&mut definition, &to_key, &mut Vec::new());
        for_each_action(&mut definition, |action, config| {
            for field in self.secret_fields(action) {
                config.remove(*field);
                config.remove(&configured_flag(field));
            }
            rewrite_config(config, false, &to_key);
        });
        PortableAutomation {
            name: automation.name.clone(),
            description: automation.description.clone(),
            record_type: automation.record_type.clone(),
            trigger,
            definition,
        }
    }

    async fn reconcile(
        &self,
        package: &[PackageAutomation],
    ) -> Result<(Vec<AutomationPlan>, KeyIndex), AppError> {
        let index = self.organization_index().await?;
        let existing = self.automations.list_all().await?;
        let mut plans = Vec::new();
        for entry in package {
            let proposed = proposed(entry);
            let item = PlanItem::new(
                AUTOMATIONS_SECTION,
                AUTOMATION_KIND,
                entry.key.clone(),
                proposed.name.clone(),
            );
            let current = existing
                .iter()
                .find(|automation| automation.key == entry.key)
                .cloned();
            let item = match &current {
                None => {
                    let changes: Vec<PlanChange> = proposed
                        .snapshot()
                        .into_iter()
                        .map(|(property, value)| PlanChange::new(property, Value::Null, value))
                        .collect();
                    let unknown = non_portable_paths(&proposed, &BTreeSet::new());
                    if unknown.is_empty() {
                        item.created(changes)
                    } else {
                        item.conflicting(
                            changes,
                            NON_PORTABLE_REFERENCE,
                            non_portable_message(&unknown),
                            true,
                        )
                    }
                }
                Some(current) => {
                    let portable = self.portable(current, &index);
                    let changes = diff(&portable.snapshot(), &proposed.snapshot());
                    let known = uuid_strings(&portable);
                    let unknown = non_portable_paths(&proposed, &known);
                    if current.record_type != proposed.record_type {
                        item.conflicting(
                            changes,
                            AUTOMATION_RECORD_TYPE_MISMATCH,
                            format!(
                                "the organization already has this automation for {} and the package defines it for {}",
                                current.record_type, proposed.record_type
                            ),
                            false,
                        )
                    } else if !changes.is_empty() && !unknown.is_empty() {
                        item.conflicting(
                            changes,
                            NON_PORTABLE_REFERENCE,
                            non_portable_message(&unknown),
                            true,
                        )
                    } else {
                        item.compared(changes)
                    }
                }
            };
            plans.push(AutomationPlan {
                item,
                key: entry.key.clone(),
                proposed,
                current,
            });
        }
        Ok((plans, index))
    }

    fn resolve_stages(
        &self,
        references: BTreeSet<String>,
        index: &KeyIndex,
        context: &ApplyContext,
    ) -> HashMap<String, Uuid> {
        let mut resolved = HashMap::new();
        for reference in references {
            let Some((pipeline_key, stage_key)) = reference.split_once('.') else {
                continue;
            };
            let stage_id = context
                .resolved(
                    ConfigReferenceKind::new("workflow.stage"),
                    &stage_item_key(pipeline_key, stage_key),
                )
                .or_else(|| {
                    context
                        .resolved(ConfigReferenceKind::new("workflow.pipeline"), pipeline_key)
                        .or_else(|| index.pipeline_id(pipeline_key))
                        .and_then(|pipeline_id| index.stage_id(pipeline_id, stage_key))
                });
            if let Some(stage_id) = stage_id {
                resolved.insert(reference, stage_id);
            }
        }
        resolved
    }

    fn materialize(
        &self,
        proposed: &PortableAutomation,
        previous: Option<&AutomationDefinition>,
        lookup: &dyn Fn(ReferenceKind, &str) -> Option<String>,
    ) -> Result<(TriggerSpec, AutomationDefinition), AppError> {
        let mut trigger = proposed.trigger.clone();
        let mut definition = proposed.definition.clone();
        let mut missing: Vec<String> = Vec::new();
        collect_missing(&trigger, &TRIGGER_REFERENCES, lookup, &mut missing);
        rewrite_trigger(&mut trigger, true, lookup);
        for_each_action(&mut definition, |_, config| {
            collect_missing(
                &Value::Object(config.clone()),
                &ACTION_REFERENCES,
                lookup,
                &mut missing,
            );
            rewrite_config(config, true, lookup);
        });
        rewrite_segment_conditions(&mut trigger, lookup, &mut missing);
        rewrite_segment_conditions(&mut definition, lookup, &mut missing);
        if !missing.is_empty() {
            missing.sort();
            missing.dedup();
            return Err(AppError::Validation(format!(
                "the package references items that were not installed: {}",
                missing.join(", ")
            )));
        }
        self.restore_secret_flags(&mut definition, previous);
        let trigger =
            parse_trigger(&trigger).map_err(|violation| AppError::Validation(violation.message))?;
        let definition = parse_definition(&definition)
            .map_err(|violation| AppError::Validation(violation.message))?;
        Ok((trigger, definition))
    }

    fn restore_secret_flags(
        &self,
        definition: &mut Value,
        previous: Option<&AutomationDefinition>,
    ) {
        let Some(steps) = definition.get_mut("steps").and_then(Value::as_array_mut) else {
            return;
        };
        for step in steps {
            let Some(step_id) = step.get("id").and_then(Value::as_str).map(str::to_owned) else {
                continue;
            };
            let Some(action) = step
                .get("action")
                .and_then(Value::as_str)
                .map(str::to_owned)
            else {
                continue;
            };
            let fields = self.secret_fields(&action);
            if fields.is_empty() {
                continue;
            }
            let previous_config = previous
                .and_then(|definition| definition.step(&step_id))
                .and_then(|step| serde_json::to_value(&step.body).ok())
                .and_then(|body| body.get("config").cloned());
            let Some(config) = step.get_mut("config") else {
                continue;
            };
            if config.is_null() {
                *config = Value::Object(Map::new());
            }
            let Some(config) = config.as_object_mut() else {
                continue;
            };
            for field in fields {
                let flag = configured_flag(field);
                let configured = previous_config
                    .as_ref()
                    .and_then(|config| config.get(&flag))
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                config.insert(flag, Value::Bool(configured));
            }
        }
    }

    fn validate_entry(
        &self,
        entry: &PackageAutomation,
        registry: &RecordTypeRegistry,
        available: &BTreeMap<ReferenceKind, BTreeSet<String>>,
        issues: &mut Vec<PackageIssue>,
    ) {
        let name_length = entry.name.trim().chars().count();
        if name_length == 0 || name_length > MAX_NAME_LENGTH {
            issues.push(PackageIssue::new(
                "name",
                "invalid_name",
                format!("must have between 1 and {MAX_NAME_LENGTH} characters"),
            ));
        }
        if entry
            .description
            .as_deref()
            .is_some_and(|text| text.trim().chars().count() > MAX_DESCRIPTION_LENGTH)
        {
            issues.push(PackageIssue::new(
                "description",
                "invalid_description",
                format!("must have at most {MAX_DESCRIPTION_LENGTH} characters"),
            ));
        }
        let record_type = entry.record_type.as_str();
        let supported_type = is_automatable(record_type) && registry.require(record_type).is_ok();
        if !supported_type {
            issues.push(PackageIssue::new(
                "record_type",
                "unknown_record_type",
                "must be a record type that supports automations or a custom object of the package or the organization",
            ));
        }
        match parse_trigger(&entry.trigger) {
            Err(violation) => issues.push(PackageIssue::new(
                violation.field,
                "invalid_trigger",
                violation.message,
            )),
            Ok(trigger) => {
                match self.triggers.get(&trigger.kind) {
                    None => issues.push(PackageIssue::new(
                        "trigger.type",
                        "unknown_trigger",
                        format!("{} is not a known trigger", trigger.kind),
                    )),
                    Some(known) if supported_type && !known.supports(record_type) => {
                        issues.push(PackageIssue::new(
                            "trigger.type",
                            "unsupported_trigger",
                            format!("{} is not available for {record_type}", trigger.kind),
                        ))
                    }
                    Some(_) => {}
                }
                check_references(
                    &entry.trigger,
                    "trigger",
                    &TRIGGER_REFERENCES,
                    available,
                    issues,
                );
                check_segment_conditions(&entry.trigger, "trigger", available, issues);
            }
        }
        check_segment_conditions(&entry.definition, "definition", available, issues);
        if entry.definition.to_string().len() > MAX_DEFINITION_BYTES {
            issues.push(PackageIssue::new(
                "definition",
                "definition_too_large",
                format!("must have at most {MAX_DEFINITION_BYTES} bytes"),
            ));
            return;
        }
        let definition = match parse_definition(&entry.definition) {
            Ok(definition) => definition,
            Err(violation) => {
                issues.push(PackageIssue::new(
                    violation.field,
                    "invalid_definition",
                    violation.message,
                ));
                return;
            }
        };
        for violation in definition.structure_violations() {
            issues.push(PackageIssue::new(
                violation.field,
                "invalid_definition",
                violation.message,
            ));
        }
        for (step_index, _, action, config) in definition.actions() {
            let path = format!("definition.steps.{step_index}");
            match self.actions.get(action) {
                None => issues.push(PackageIssue::new(
                    format!("{path}.action"),
                    "unknown_action",
                    format!("{action} is not a known action"),
                )),
                Some(known) => {
                    if supported_type && !known.supports(record_type) {
                        issues.push(PackageIssue::new(
                            format!("{path}.action"),
                            "unsupported_action",
                            format!("{action} is not available for {record_type}"),
                        ));
                    }
                    for field in known.secret_fields() {
                        for property in [(*field).to_owned(), configured_flag(field)] {
                            if config.get(&property).is_some() {
                                issues.push(PackageIssue::new(
                                    format!("{path}.config.{property}"),
                                    "secret_not_portable",
                                    "secrets are never part of a package; configure it after installing",
                                ));
                            }
                        }
                    }
                }
            }
            check_references(
                config,
                &format!("{path}.config"),
                &ACTION_REFERENCES,
                available,
                issues,
            );
        }
    }
}

fn proposed(entry: &PackageAutomation) -> PortableAutomation {
    let trigger = parse_trigger(&entry.trigger)
        .ok()
        .and_then(|trigger| serde_json::to_value(trigger).ok())
        .unwrap_or_else(|| entry.trigger.clone());
    let definition = parse_definition(&entry.definition)
        .ok()
        .and_then(|definition| serde_json::to_value(definition).ok())
        .unwrap_or_else(|| entry.definition.clone());
    PortableAutomation {
        name: entry.name.trim().to_owned(),
        description: entry
            .description
            .as_deref()
            .map(str::trim)
            .filter(|text| !text.is_empty())
            .map(str::to_owned),
        record_type: entry.record_type.trim().to_owned(),
        trigger,
        definition,
    }
}

fn for_each_action(definition: &mut Value, mut visit: impl FnMut(&str, &mut Map<String, Value>)) {
    let Some(steps) = definition.get_mut("steps").and_then(Value::as_array_mut) else {
        return;
    };
    for step in steps {
        if step.get("type").and_then(Value::as_str) != Some("action") {
            continue;
        }
        let action = step
            .get("action")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned();
        if let Some(config) = step.get_mut("config").and_then(Value::as_object_mut) {
            visit(&action, config);
        }
    }
}

fn swap_reference(
    map: &mut Map<String, Value>,
    from: &str,
    to: &str,
    kind: ReferenceKind,
    lookup: &dyn Fn(ReferenceKind, &str) -> Option<String>,
) {
    let Some(value) = map.get(from).and_then(Value::as_str) else {
        return;
    };
    if let Some(translated) = lookup(kind, value) {
        map.remove(from);
        map.insert(to.to_owned(), Value::String(translated));
    }
}

fn rewrite_map(
    map: &mut Map<String, Value>,
    table: &[(&str, &str, ReferenceKind)],
    to_ids: bool,
    lookup: &dyn Fn(ReferenceKind, &str) -> Option<String>,
) {
    for (id_field, key_field, kind) in table {
        if to_ids {
            swap_reference(map, key_field, id_field, *kind, lookup);
        } else {
            swap_reference(map, id_field, key_field, *kind, lookup);
        }
    }
}

fn rewrite_trigger(
    trigger: &mut Value,
    to_ids: bool,
    lookup: &dyn Fn(ReferenceKind, &str) -> Option<String>,
) {
    if let Some(map) = trigger.as_object_mut() {
        rewrite_map(map, &TRIGGER_REFERENCES, to_ids, lookup);
    }
}

fn rewrite_config(
    config: &mut Map<String, Value>,
    to_ids: bool,
    lookup: &dyn Fn(ReferenceKind, &str) -> Option<String>,
) {
    rewrite_map(config, &ACTION_REFERENCES, to_ids, lookup);
}

fn collect_missing(
    value: &Value,
    table: &[(&str, &str, ReferenceKind)],
    lookup: &dyn Fn(ReferenceKind, &str) -> Option<String>,
    missing: &mut Vec<String>,
) {
    for (_, key_field, kind) in table {
        if let Some(key) = value.get(*key_field).and_then(Value::as_str)
            && lookup(*kind, key).is_none()
        {
            missing.push(format!("{} {key}", kind.label()));
        }
    }
}

fn check_references(
    value: &Value,
    path: &str,
    table: &[(&str, &str, ReferenceKind)],
    available: &BTreeMap<ReferenceKind, BTreeSet<String>>,
    issues: &mut Vec<PackageIssue>,
) {
    for (_, key_field, kind) in table {
        match value.get(*key_field) {
            None => {}
            Some(Value::String(key))
                if available.get(kind).is_some_and(|known| known.contains(key)) => {}
            Some(Value::String(key)) => issues.push(PackageIssue::new(
                format!("{path}.{key_field}"),
                "unknown_reference",
                format!(
                    "{} {key} exists neither in the package nor in the organization",
                    kind.label()
                ),
            )),
            Some(_) => issues.push(PackageIssue::new(
                format!("{path}.{key_field}"),
                "invalid_reference",
                "must be a key",
            )),
        }
    }
}

fn rewrite_segment_conditions(
    value: &mut Value,
    lookup: &dyn Fn(ReferenceKind, &str) -> Option<String>,
    missing: &mut Vec<String>,
) {
    match value {
        Value::Array(items) => {
            for item in items {
                rewrite_segment_conditions(item, lookup, missing);
            }
        }
        Value::Object(map) => {
            if map.get("operator").and_then(Value::as_str) == Some(IN_SEGMENT_OPERATOR)
                && let Some(reference) = map.get("value").and_then(Value::as_str)
            {
                match lookup(ReferenceKind::Segment, reference) {
                    Some(translated) => {
                        map.insert("value".to_owned(), Value::String(translated));
                    }
                    None if !is_uuid(reference.trim()) => {
                        missing.push(format!("{} {reference}", ReferenceKind::Segment.label()));
                    }
                    None => {}
                }
            }
            for item in map.values_mut() {
                rewrite_segment_conditions(item, lookup, missing);
            }
        }
        _ => {}
    }
}

fn check_segment_conditions(
    value: &Value,
    path: &str,
    available: &BTreeMap<ReferenceKind, BTreeSet<String>>,
    issues: &mut Vec<PackageIssue>,
) {
    match value {
        Value::Array(items) => {
            for (index, item) in items.iter().enumerate() {
                check_segment_conditions(item, &format!("{path}.{index}"), available, issues);
            }
        }
        Value::Object(map) => {
            if map.get("operator").and_then(Value::as_str) == Some(IN_SEGMENT_OPERATOR)
                && let Some(reference) = map.get("value").and_then(Value::as_str)
                && !is_uuid(reference.trim())
                && !available
                    .get(&ReferenceKind::Segment)
                    .is_some_and(|known| known.contains(reference))
            {
                issues.push(PackageIssue::new(
                    format!("{path}.value"),
                    "unknown_reference",
                    format!(
                        "segment {reference} does not exist in the organization; segments are not part of packages and must be created before installing"
                    ),
                ));
            }
            for (key, item) in map {
                check_segment_conditions(item, &format!("{path}.{key}"), available, issues);
            }
        }
        _ => {}
    }
}

fn is_uuid(value: &str) -> bool {
    value.len() == 36 && Uuid::parse_str(value).is_ok()
}

fn walk_uuids(value: &Value, path: String, found: &mut Vec<(String, String)>) {
    match value {
        Value::String(text) if is_uuid(text.trim()) => {
            found.push((path, text.trim().to_owned()));
        }
        Value::Array(items) => {
            for (index, item) in items.iter().enumerate() {
                walk_uuids(item, format!("{path}.{index}"), found);
            }
        }
        Value::Object(map) => {
            for (key, item) in map {
                walk_uuids(item, format!("{path}.{key}"), found);
            }
        }
        _ => {}
    }
}

fn automation_uuids(automation: &PortableAutomation) -> Vec<(String, String)> {
    let mut found = Vec::new();
    walk_uuids(&automation.trigger, "trigger".to_owned(), &mut found);
    walk_uuids(&automation.definition, "definition".to_owned(), &mut found);
    found
}

fn uuid_strings(automation: &PortableAutomation) -> BTreeSet<String> {
    automation_uuids(automation)
        .into_iter()
        .map(|(_, value)| value)
        .collect()
}

fn non_portable_paths(automation: &PortableAutomation, known: &BTreeSet<String>) -> Vec<String> {
    automation_uuids(automation)
        .into_iter()
        .filter(|(_, value)| !known.contains(value))
        .map(|(path, _)| path)
        .collect()
}

fn non_portable_message(paths: &[String]) -> String {
    format!(
        "references by id that may not exist in this organization (users, incoming webhooks, records): {}; it is installed as a draft and must be reviewed before publishing",
        paths.join(", ")
    )
}

fn parse(content: &Value) -> Result<Vec<PackageAutomation>, AppError> {
    parse_section(content).map_err(|issue| AppError::Validation(issue.message))
}

fn package_available(package: &ConfigPackage) -> BTreeMap<ReferenceKind, BTreeSet<String>> {
    let mut available: BTreeMap<ReferenceKind, BTreeSet<String>> = BTreeMap::new();
    if let Some(Ok(tags)) = package
        .section(TAGS_SECTION)
        .map(parse_section::<Vec<PackageTag>>)
    {
        available
            .entry(ReferenceKind::Tag)
            .or_default()
            .extend(tags.into_iter().map(|tag| tag.key));
    }
    if let Some(Ok(types)) = package
        .section(ACTIVITY_TYPES_SECTION)
        .map(parse_section::<Vec<PackageActivityType>>)
    {
        available
            .entry(ReferenceKind::ActivityType)
            .or_default()
            .extend(types.into_iter().map(|activity_type| activity_type.key));
    }
    if let Some(Value::Array(sequences)) = package.section(SEQUENCES_SECTION) {
        available
            .entry(ReferenceKind::Sequence)
            .or_default()
            .extend(
                sequences
                    .iter()
                    .filter_map(|sequence| sequence.get("key").and_then(Value::as_str))
                    .map(str::to_owned),
            );
    }
    if let Some(Ok(workflow)) = package.section(PIPELINES_SECTION).map(parse_pipelines) {
        let stages = available.entry(ReferenceKind::Stage).or_default();
        for pipeline in workflow {
            for stage in pipeline.stages {
                stages.insert(format!("{}.{}", pipeline.key, stage.key));
            }
        }
    }
    available
}

#[async_trait]
impl<K> ConfigSection<MySqlTx> for AutomationsSection<K>
where
    K: ConfigKeyRepository,
{
    fn key(&self) -> &'static str {
        AUTOMATIONS_SECTION
    }

    async fn export(&self, _scope: &AccessScope) -> Result<Option<Value>, AppError> {
        let index = self.organization_index().await?;
        let mut automations = self.automations.list_all().await?;
        automations.sort_by(|left, right| {
            left.created_at
                .cmp(&right.created_at)
                .then(left.id.cmp(&right.id))
        });
        let exported: Vec<PackageAutomation> = automations
            .iter()
            .map(|automation| {
                let portable = self.portable(automation, &index);
                PackageAutomation {
                    key: automation.key.clone(),
                    name: portable.name,
                    description: portable.description,
                    record_type: portable.record_type,
                    trigger: portable.trigger,
                    definition: portable.definition,
                }
            })
            .collect();
        Ok(Some(json!(exported)))
    }

    async fn validate(
        &self,
        _scope: &AccessScope,
        package: &ConfigPackage,
        content: &Value,
    ) -> Result<Vec<PackageIssue>, AppError> {
        let automations: Vec<PackageAutomation> = match parse_section(content) {
            Ok(automations) => automations,
            Err(issue) => return Ok(vec![issue]),
        };
        let mut issues = Vec::new();
        if automations.len() > MAX_PACKAGE_AUTOMATIONS {
            issues.push(PackageIssue::new(
                "",
                "too_many_automations",
                format!("a package can have at most {MAX_PACKAGE_AUTOMATIONS} automations"),
            ));
        }
        let index = self.organization_index().await?;
        let mut available = package_available(package);
        available
            .entry(ReferenceKind::Tag)
            .or_default()
            .extend(index.tags.iter().map(|(_, key)| key.clone()));
        available
            .entry(ReferenceKind::ActivityType)
            .or_default()
            .extend(index.task_types.iter().map(|(_, key)| key.clone()));
        available
            .entry(ReferenceKind::Sequence)
            .or_default()
            .extend(index.sequences.iter().map(|(_, key)| key.clone()));
        available
            .entry(ReferenceKind::Segment)
            .or_default()
            .extend(index.segments.iter().map(|(_, key)| key.clone()));
        available.entry(ReferenceKind::Stage).or_default().extend(
            index
                .stages
                .iter()
                .map(|stage| format!("{}.{}", stage.pipeline_key, stage.key)),
        );
        let registry = package_registry(&self.registry, package);
        let mut keys = BTreeSet::new();
        for (index_in_package, entry) in automations.iter().enumerate() {
            let path = index_in_package.to_string();
            if !valid_key(&entry.key) {
                issues.push(PackageIssue::new(
                    format!("{path}.key"),
                    "invalid_key",
                    "must start with a letter and have up to 64 lowercase letters, digits or '_'",
                ));
            } else if !keys.insert(entry.key.clone()) {
                issues.push(PackageIssue::new(
                    format!("{path}.key"),
                    "duplicated_key",
                    "is declared more than once",
                ));
            }
            let mut entry_issues = Vec::new();
            self.validate_entry(entry, &registry, &available, &mut entry_issues);
            issues.extend(entry_issues.into_iter().map(|issue| issue.under(&path)));
        }
        let existing = self.automations.list_all().await?;
        let created = automations
            .iter()
            .filter(|entry| !existing.iter().any(|current| current.key == entry.key))
            .count();
        if (existing.len() + created) as u64 > MAX_AUTOMATIONS {
            issues.push(PackageIssue::new(
                "",
                "too_many_automations",
                format!("the organization would have more than {MAX_AUTOMATIONS} automations"),
            ));
        }
        Ok(issues)
    }

    async fn plan(
        &self,
        _scope: &AccessScope,
        _package: &ConfigPackage,
        content: &Value,
    ) -> Result<Vec<PlanItem>, AppError> {
        let (plans, _) = self.reconcile(&parse(content)?).await?;
        Ok(plans.into_iter().map(|plan| plan.item).collect())
    }

    async fn apply(
        &self,
        tx: &mut MySqlTx,
        scope: &AccessScope,
        _package: &ConfigPackage,
        content: &Value,
        decisions: &Decisions,
        context: &mut ApplyContext,
    ) -> Result<(), AppError> {
        let (plans, index) = self.reconcile(&parse(content)?).await?;
        for plan in plans {
            if let Some(current) = &plan.current {
                context.resolve(
                    ConfigReferenceKind::new("automations.automation"),
                    &plan.key,
                    current.id,
                );
            }
            if !decisions.executes(&plan.item) {
                continue;
            }
            let mut stage_references = BTreeSet::new();
            let mut collect = |value: &Value, table: &[(&str, &str, ReferenceKind)]| {
                for (_, key_field, kind) in table {
                    if *kind == ReferenceKind::Stage
                        && let Some(key) = value.get(*key_field).and_then(Value::as_str)
                    {
                        stage_references.insert(key.to_owned());
                    }
                }
            };
            collect(&plan.proposed.trigger, &TRIGGER_REFERENCES);
            let mut definition = plan.proposed.definition.clone();
            for_each_action(&mut definition, |_, config| {
                collect(&Value::Object(config.clone()), &ACTION_REFERENCES);
            });
            let stages = self.resolve_stages(stage_references, &index, context);
            let lookup = |kind: ReferenceKind, key: &str| -> Option<String> {
                let id = match kind {
                    ReferenceKind::Tag => context
                        .resolved(ConfigReferenceKind::new("tagging.tag"), key)
                        .or_else(|| {
                            index
                                .tags
                                .iter()
                                .find(|(_, known)| known == key)
                                .map(|(id, _)| *id)
                        }),
                    ReferenceKind::ActivityType => context
                        .resolved(ConfigReferenceKind::new("tasks.activity_type"), key)
                        .or_else(|| {
                            index
                                .task_types
                                .iter()
                                .find(|(_, known)| known == key)
                                .map(|(id, _)| *id)
                        }),
                    ReferenceKind::Stage => stages.get(key).copied(),
                    ReferenceKind::Sequence => context
                        .resolved(ConfigReferenceKind::new("sequences.sequence"), key)
                        .or_else(|| KeyIndex::id_of(&index.sequences, key)),
                    ReferenceKind::Segment => KeyIndex::id_of(&index.segments, key),
                };
                id.map(|id| id.to_string())
            };
            let is_new = plan.current.is_none();
            let previous = plan.current.as_ref().map(|current| &current.definition);
            let (trigger, definition) = self.materialize(&plan.proposed, previous, &lookup)?;
            let automation = match plan.current {
                None => Automation {
                    id: Uuid::now_v7(),
                    key: plan.key.clone(),
                    name: plan.proposed.name,
                    description: plan.proposed.description,
                    record_type: plan.proposed.record_type,
                    trigger,
                    definition,
                    status: AutomationStatus::Draft,
                    version: 0,
                    created_by: Some(scope.user_id().clone()),
                    updated_by: Some(scope.user_id().clone()),
                    created_at: context.at,
                    updated_at: context.at,
                },
                Some(current) => Automation {
                    name: plan.proposed.name,
                    description: plan.proposed.description,
                    trigger,
                    definition,
                    updated_by: Some(scope.user_id().clone()),
                    updated_at: context.at,
                    ..current
                },
            };
            if is_new {
                self.automations.insert(tx, &automation).await?;
            } else {
                self.automations.update(tx, &automation).await?;
            }
            context.resolve(
                ConfigReferenceKind::new("automations.automation"),
                &plan.key,
                automation.id,
            );
        }
        Ok(())
    }
}
