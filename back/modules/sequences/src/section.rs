use ::config_packages::application::ConfigReferenceKind;
use std::collections::{BTreeSet, HashMap};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::domain::{
    DEFAULT_DAILY_LIMIT, MAX_DAILY_LIMIT, SendWindow, format_clock, normalize_sequence_name,
};
use crate::store::SequenceStore;
use crate::{SequenceStep, SequenceView, StepInput, parse_window, validate_steps};
use ::config_packages::application::sections::common::{check_key, diff};
use ::config_packages::application::{
    ApplyContext, ConfigPackage, ConfigSection, Decisions, PackageIssue, PlanAction, PlanChange,
    PlanItem, parse_section,
};
use aphel_kernel::database::transaction::MySqlTx;
use aphel_kernel::domain::access::AccessScope;
use aphel_kernel::errors::{AppError, FieldViolation};

pub(crate) const SEQUENCES_SECTION: &str = "sequences";
const SEQUENCE_KIND: &str = "sequence";
const EMAIL_TEMPLATES_SECTION: &str = "email_templates";
pub(crate) const MAX_PACKAGE_SEQUENCES: usize = 100;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PackageSequenceStep {
    pub(crate) kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) wait_days: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) template_key: Option<String>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub(crate) subject: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) body_html: Option<String>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub(crate) title: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PackageSequence {
    pub(crate) key: String,
    pub(crate) name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) description: Option<String>,
    #[serde(default = "active_default")]
    pub(crate) is_active: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) window_start: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) window_end: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) daily_limit: Option<u32>,
    #[serde(default)]
    pub(crate) steps: Vec<PackageSequenceStep>,
}

fn active_default() -> bool {
    true
}

struct SequencePlan {
    item: PlanItem,
    sequence: SequenceView,
    is_new: bool,
}

fn template_keys_of_package(package: &ConfigPackage) -> BTreeSet<String> {
    package
        .sections
        .get(EMAIL_TEMPLATES_SECTION)
        .and_then(Value::as_array)
        .map(|templates| {
            templates
                .iter()
                .filter_map(|template| template.get("key").and_then(Value::as_str))
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

fn snapshot(sequence: &SequenceView, keys: &HashMap<Uuid, String>) -> Vec<(String, Value)> {
    vec![
        ("name".to_owned(), json!(sequence.name)),
        ("description".to_owned(), json!(sequence.description)),
        ("is_active".to_owned(), json!(sequence.is_active)),
        (
            "window_start".to_owned(),
            json!(format_clock(sequence.window.start_minute)),
        ),
        (
            "window_end".to_owned(),
            json!(format_clock(sequence.window.end_minute)),
        ),
        ("daily_limit".to_owned(), json!(sequence.daily_limit)),
        (
            "steps".to_owned(),
            json!(
                sequence
                    .steps
                    .iter()
                    .map(|step| package_step(step, keys))
                    .collect::<Vec<_>>()
            ),
        ),
    ]
}

fn package_step(step: &SequenceStep, keys: &HashMap<Uuid, String>) -> PackageSequenceStep {
    PackageSequenceStep {
        kind: step.kind.as_str().to_owned(),
        wait_days: (step.wait_days > 0).then_some(step.wait_days),
        template_key: step.template_id.and_then(|id| keys.get(&id).cloned()),
        subject: step.subject.clone(),
        body_html: step.body_html.clone(),
        title: step.title.clone(),
    }
}

fn step_inputs(
    steps: &[PackageSequenceStep],
    templates: &HashMap<String, Uuid>,
    placeholder: bool,
) -> Vec<StepInput> {
    steps
        .iter()
        .map(|step| StepInput {
            id: None,
            kind: step.kind.clone(),
            wait_days: step.wait_days,
            template_id: step.template_key.as_ref().map(|key| {
                templates
                    .get(key)
                    .copied()
                    .or(placeholder.then(Uuid::nil))
                    .map(|id| id.to_string())
                    .unwrap_or_default()
            }),
            subject: Some(step.subject.clone()),
            body_html: step.body_html.clone(),
            title: Some(step.title.clone()),
        })
        .collect()
}

fn build(
    entry: &PackageSequence,
    current: Option<&SequenceView>,
    templates: &HashMap<String, Uuid>,
    placeholder: bool,
    at: DateTime<Utc>,
    violations: &mut Vec<FieldViolation>,
) -> SequenceView {
    let mut ids: HashMap<Uuid, String> = templates
        .iter()
        .map(|(key, id)| (*id, key.clone()))
        .collect();
    if placeholder {
        ids.insert(Uuid::nil(), String::new());
    }
    let name = normalize_sequence_name(&entry.name).unwrap_or_else(|message| {
        violations.push(FieldViolation::new("name", message));
        entry.name.clone()
    });
    let window = parse_window(
        entry.window_start.as_deref(),
        entry.window_end.as_deref(),
        SendWindow::default(),
        violations,
    );
    let daily_limit = entry.daily_limit.unwrap_or(DEFAULT_DAILY_LIMIT);
    if !(1..=MAX_DAILY_LIMIT).contains(&daily_limit) {
        violations.push(FieldViolation::new(
            "daily_limit",
            format!("must be between 1 and {MAX_DAILY_LIMIT}"),
        ));
    }
    let current_steps = current
        .map(|current| current.steps.clone())
        .unwrap_or_default();
    let mut steps = validate_steps(
        &step_inputs(&entry.steps, templates, placeholder),
        &current_steps,
        &ids,
        violations,
    );
    for (index, step) in steps.iter_mut().enumerate() {
        if let Some(existing) = current_steps.get(index)
            && existing.kind == step.kind
        {
            step.id = existing.id;
        }
    }
    SequenceView {
        id: current.map_or_else(Uuid::now_v7, |current| current.id),
        key: entry.key.clone(),
        name,
        description: entry
            .description
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_owned),
        is_active: entry.is_active,
        window,
        daily_limit,
        steps,
        created_by: current.and_then(|current| current.created_by.clone()),
        updated_by: current.and_then(|current| current.updated_by.clone()),
        created_at: current.map_or(at, |current| current.created_at),
        updated_at: at,
    }
}

pub(crate) struct SequencesSection {
    sequences: SequenceStore,
}

impl SequencesSection {
    pub(crate) fn new(sequences: SequenceStore) -> Self {
        Self { sequences }
    }

    async fn reconcile(
        &self,
        scope: &AccessScope,
        package: &[PackageSequence],
        templates: &HashMap<String, Uuid>,
        placeholder: bool,
        at: DateTime<Utc>,
    ) -> Result<Vec<SequencePlan>, AppError> {
        let existing = self.sequences.all().await?;
        let ids: HashMap<Uuid, String> = self.sequences.template_keys().await?;
        let mut plans = Vec::new();
        for entry in package {
            let item = PlanItem::new(
                SEQUENCES_SECTION,
                SEQUENCE_KIND,
                entry.key.clone(),
                entry.name.clone(),
            );
            let current = existing.iter().find(|sequence| sequence.key == entry.key);
            let mut violations = Vec::new();
            let mut sequence = build(entry, current, templates, placeholder, at, &mut violations);
            if !violations.is_empty() {
                return Err(AppError::invalid_fields(
                    format!("invalid sequence {}", entry.key),
                    violations,
                ));
            }
            sequence.updated_by = Some(scope.user_id().clone());
            match current {
                None => {
                    sequence.created_by = Some(scope.user_id().clone());
                    let changes = snapshot(&sequence, &ids)
                        .into_iter()
                        .map(|(property, value)| PlanChange::new(property, Value::Null, value))
                        .collect();
                    plans.push(SequencePlan {
                        item: item.created(changes),
                        sequence,
                        is_new: true,
                    });
                }
                Some(current) => {
                    let changes = diff(&snapshot(current, &ids), &snapshot(&sequence, &ids));
                    plans.push(SequencePlan {
                        item: item.compared(changes),
                        sequence,
                        is_new: false,
                    });
                }
            }
        }
        Ok(plans)
    }

    async fn template_ids(&self) -> Result<HashMap<String, Uuid>, AppError> {
        Ok(self
            .sequences
            .template_keys()
            .await?
            .into_iter()
            .map(|(id, key)| (key, id))
            .collect())
    }
}

fn parse(content: &Value) -> Result<Vec<PackageSequence>, AppError> {
    parse_section(content).map_err(|issue| AppError::Validation(issue.message))
}

#[async_trait]
impl ConfigSection<MySqlTx> for SequencesSection {
    fn key(&self) -> &'static str {
        SEQUENCES_SECTION
    }

    async fn export(&self, _scope: &AccessScope) -> Result<Option<Value>, AppError> {
        let keys = self.sequences.template_keys().await?;
        let sequences: Vec<PackageSequence> = self
            .sequences
            .all()
            .await?
            .into_iter()
            .map(|sequence| PackageSequence {
                key: sequence.key.clone(),
                name: sequence.name.clone(),
                description: sequence.description.clone(),
                is_active: sequence.is_active,
                window_start: Some(format_clock(sequence.window.start_minute)),
                window_end: Some(format_clock(sequence.window.end_minute)),
                daily_limit: Some(sequence.daily_limit),
                steps: sequence
                    .steps
                    .iter()
                    .map(|step| package_step(step, &keys))
                    .collect(),
            })
            .collect();
        if sequences.is_empty() {
            return Ok(None);
        }
        Ok(Some(json!(sequences)))
    }

    async fn validate(
        &self,
        _scope: &AccessScope,
        package: &ConfigPackage,
        content: &Value,
    ) -> Result<Vec<PackageIssue>, AppError> {
        let sequences: Vec<PackageSequence> = match parse_section(content) {
            Ok(sequences) => sequences,
            Err(issue) => return Ok(vec![issue]),
        };
        let mut issues = Vec::new();
        if sequences.len() > MAX_PACKAGE_SEQUENCES {
            issues.push(PackageIssue::new(
                "",
                "too_many_sequences",
                format!("a package can have at most {MAX_PACKAGE_SEQUENCES} sequences"),
            ));
        }
        let mut known = template_keys_of_package(package);
        known.extend(self.template_ids().await?.into_keys());
        let mut keys = BTreeSet::new();
        for (index, sequence) in sequences.iter().enumerate() {
            check_key(
                &sequence.key,
                &format!("{index}.key"),
                &mut keys,
                &mut issues,
            );
            for (position, step) in sequence.steps.iter().enumerate() {
                if let Some(key) = &step.template_key
                    && !known.contains(key)
                {
                    issues.push(PackageIssue::new(
                        format!("{index}.steps.{position}.template_key"),
                        "unknown_email_template",
                        "must be the key of an e-mail template of the organization or of the package",
                    ));
                }
            }
            let placeholder: HashMap<String, Uuid> =
                known.iter().map(|key| (key.clone(), Uuid::nil())).collect();
            let mut violations = Vec::new();
            build(
                sequence,
                None,
                &placeholder,
                true,
                Utc::now(),
                &mut violations,
            );
            for violation in violations {
                issues.push(PackageIssue::new(
                    format!("{index}.{}", violation.field),
                    "invalid_sequence",
                    violation.message,
                ));
            }
        }
        Ok(issues)
    }

    async fn plan(
        &self,
        scope: &AccessScope,
        _package: &ConfigPackage,
        content: &Value,
    ) -> Result<Vec<PlanItem>, AppError> {
        let templates = self.template_ids().await?;
        Ok(self
            .reconcile(
                scope,
                &parse(content)?,
                &templates,
                true,
                DateTime::<Utc>::UNIX_EPOCH,
            )
            .await?
            .into_iter()
            .map(|plan| plan.item)
            .collect())
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
        let templates = self.sequences.template_ids_in(tx).await?;
        for plan in self
            .reconcile(scope, &parse(content)?, &templates, false, context.at)
            .await?
        {
            if plan.item.action == PlanAction::Unchanged || !decisions.executes(&plan.item) {
                continue;
            }
            if plan.is_new {
                self.sequences.insert(tx, &plan.sequence).await?;
            } else {
                self.sequences.update(tx, &plan.sequence).await?;
            }
            context.resolve(
                ConfigReferenceKind::new("sequences.sequence"),
                &plan.sequence.key,
                plan.sequence.id,
            );
        }
        Ok(())
    }
}
