use aphel_sdk::configuration::ConfigReferenceKind;
use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::domain::{
    ApprovalMoment, ApprovalPolicy, ApprovalRule, ApproverKind, ConditionKind, PolicyCondition,
    PolicyListFilter, PolicyStep, StepInput,
};
use crate::engine::acting_user;
use crate::service::{PolicyDraft, check_shape};
use crate::store::{ApprovalDirectory, ApprovalStore};
use aphel_kernel::domain::UserId;
use aphel_kernel::errors::AppError;
use aphel_sdk::configuration::sections::common::{check_key, diff};
use aphel_sdk::configuration::{
    ApplyContext, ConfigPackage, ConfigSection, Decisions, PackageIssue, PlanAction, PlanChange,
    PlanItem, parse_section,
};
use aphel_sdk::configuration::{package_reference_keys, reference_keys};
use aphel_sdk::platform::application::fields::RecordTypeRegistry;
use aphel_sdk::platform::application::shared::RecordType;
use aphel_sdk::platform::database::transaction::MySqlTx;
use aphel_sdk::platform::domain::access::AccessScope;
use sqlx::MySqlPool;

pub(crate) const APPROVAL_POLICIES_SECTION: &str = "approval_policies";
const APPROVAL_POLICY_KIND: &str = "approval_policy";
const CUSTOM_OBJECTS_SECTION: &str = "custom_objects";
pub(crate) const MAX_PACKAGE_APPROVAL_POLICIES: usize = 100;

fn always() -> String {
    ConditionKind::Always.as_str().to_owned()
}

fn active() -> bool {
    true
}

fn any_rule() -> String {
    ApprovalRule::Any.as_str().to_owned()
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PackageApprovalStep {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) name: Option<String>,
    pub(crate) approver: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) user_email: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) role_key: Option<String>,
    #[serde(default = "any_rule")]
    pub(crate) rule: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PackageApprovalPolicy {
    pub(crate) key: String,
    pub(crate) name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) description: Option<String>,
    pub(crate) record_type: String,
    pub(crate) moment: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) target_stage: Option<String>,
    #[serde(default = "always")]
    pub(crate) condition_kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) condition_filter: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) condition_formula: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) locked_fields: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) due_hours: Option<u32>,
    #[serde(default = "active")]
    pub(crate) is_active: bool,
    pub(crate) steps: Vec<PackageApprovalStep>,
}

impl PackageApprovalPolicy {
    fn draft(&self) -> PolicyDraft {
        PolicyDraft {
            key: self.key.clone(),
            name: self.name.clone(),
            description: self.description.clone(),
            record_type: self.record_type.clone(),
            moment: self.moment.clone(),
            target_id: None,
            condition_kind: self.condition_kind.clone(),
            condition_filter: self.condition_filter.clone(),
            condition_formula: self.condition_formula.clone(),
            locked_fields: self.locked_fields.clone(),
            due_hours: self.due_hours.map(i64::from),
            is_active: self.is_active,
            steps: self
                .steps
                .iter()
                .map(|step| StepInput {
                    name: step.name.clone(),
                    approver_kind: step.approver.clone(),
                    approver_user_id: None,
                    approver_role_id: None,
                    rule: Some(step.rule.clone()),
                })
                .collect(),
        }
    }

    fn snapshot(&self) -> Vec<(String, Value)> {
        vec![
            ("name".to_owned(), json!(self.name)),
            ("description".to_owned(), json!(self.description)),
            ("record_type".to_owned(), json!(self.record_type)),
            ("moment".to_owned(), json!(self.moment)),
            ("target_stage".to_owned(), json!(self.target_stage)),
            ("condition_kind".to_owned(), json!(self.condition_kind)),
            ("condition_filter".to_owned(), json!(self.condition_filter)),
            (
                "condition_formula".to_owned(),
                json!(self.condition_formula),
            ),
            ("locked_fields".to_owned(), json!(self.locked_fields)),
            ("due_hours".to_owned(), json!(self.due_hours)),
            ("is_active".to_owned(), json!(self.is_active)),
            ("steps".to_owned(), json!(self.steps)),
        ]
    }
}

struct KeyedPipeline {
    key: String,
    stages: Vec<(Uuid, String)>,
}

struct Index {
    workflow: Vec<KeyedPipeline>,
    roles: Vec<(Uuid, String)>,
    members: Vec<(UserId, String)>,
}

impl Index {
    fn stage_reference(&self, stage_id: Uuid) -> Option<String> {
        self.workflow.iter().find_map(|pipeline| {
            pipeline
                .stages
                .iter()
                .find(|(id, _)| *id == stage_id)
                .map(|(_, key)| format!("{}.{}", pipeline.key, key))
        })
    }

    fn pipeline(&self, key: &str) -> Option<&KeyedPipeline> {
        self.workflow.iter().find(|pipeline| pipeline.key == key)
    }

    fn role_key(&self, id: Uuid) -> Option<&str> {
        self.roles
            .iter()
            .find(|(role, _)| *role == id)
            .map(|(_, key)| key.as_str())
    }

    fn role_id(&self, key: &str) -> Option<Uuid> {
        self.roles
            .iter()
            .find(|(_, role)| role == key)
            .map(|(id, _)| *id)
    }

    fn email(&self, user: &UserId) -> Option<&str> {
        self.members
            .iter()
            .find(|(id, _)| id == user)
            .map(|(_, email)| email.as_str())
    }

    fn user(&self, email: &str) -> Option<UserId> {
        self.members
            .iter()
            .find(|(_, candidate)| candidate.eq_ignore_ascii_case(email.trim()))
            .map(|(id, _)| id.clone())
    }
}

fn parse(content: &Value) -> Result<Vec<PackageApprovalPolicy>, AppError> {
    parse_section(content).map_err(|issue| AppError::Validation(issue.message))
}

fn split_stage(reference: &str) -> Option<(&str, &str)> {
    reference
        .split_once('.')
        .filter(|(pipeline, stage)| !pipeline.is_empty() && !stage.is_empty())
}

pub(crate) struct ApprovalPoliciesSection {
    registry: Arc<RecordTypeRegistry>,
    repository: ApprovalStore,
    directory: ApprovalDirectory,
    pool: MySqlPool,
}

impl ApprovalPoliciesSection {
    pub(crate) fn new(
        registry: Arc<RecordTypeRegistry>,
        repository: ApprovalStore,
        directory: ApprovalDirectory,
        pool: MySqlPool,
    ) -> Self {
        Self {
            registry,
            repository,
            directory,
            pool,
        }
    }

    async fn index(&self) -> Result<Index, AppError> {
        let pipeline_keys =
            reference_keys(&self.pool, ConfigReferenceKind::new("workflow.pipeline")).await?;
        let stage_keys =
            reference_keys(&self.pool, ConfigReferenceKind::new("workflow.stage")).await?;
        let workflow = pipeline_keys
            .into_values()
            .map(|key| {
                let prefix = format!("{key}/");
                let stages = stage_keys
                    .iter()
                    .filter_map(|(id, value)| {
                        value
                            .strip_prefix(&prefix)
                            .map(|stage| (*id, stage.to_owned()))
                    })
                    .collect();
                KeyedPipeline { key, stages }
            })
            .collect();
        Ok(Index {
            workflow,
            roles: self.directory.role_keys().await?,
            members: self.directory.member_emails().await?,
        })
    }

    fn portable(&self, index: &Index, policy: &ApprovalPolicy) -> PackageApprovalPolicy {
        PackageApprovalPolicy {
            key: policy.key.clone(),
            name: policy.name.clone(),
            description: policy.description.clone(),
            record_type: policy.record_type.clone(),
            moment: policy.moment.as_str().to_owned(),
            target_stage: policy
                .target_id
                .and_then(|stage| index.stage_reference(stage)),
            condition_kind: policy.condition.kind().as_str().to_owned(),
            condition_filter: policy
                .condition
                .filter()
                .and_then(|filter| serde_json::to_value(filter).ok()),
            condition_formula: policy.condition.formula().map(str::to_owned),
            locked_fields: policy.locked_fields.clone(),
            due_hours: policy.due_hours,
            is_active: policy.is_active,
            steps: policy
                .steps
                .iter()
                .map(|step| PackageApprovalStep {
                    name: step.name.clone(),
                    approver: step.approver_kind.as_str().to_owned(),
                    user_email: step
                        .approver_user_id
                        .as_ref()
                        .and_then(|user| index.email(user))
                        .map(str::to_owned),
                    role_key: step
                        .approver_role_id
                        .and_then(|role| index.role_key(role))
                        .map(str::to_owned),
                    rule: step.rule.as_str().to_owned(),
                })
                .collect(),
        }
    }

    async fn plans(
        &self,
        entries: &[PackageApprovalPolicy],
        index: &Index,
    ) -> Result<Vec<(PlanItem, Option<ApprovalPolicy>)>, AppError> {
        let existing = self
            .repository
            .policies(&PolicyListFilter::default())
            .await?;
        let mut plans = Vec::new();
        for entry in entries {
            let item = PlanItem::new(
                APPROVAL_POLICIES_SECTION,
                APPROVAL_POLICY_KIND,
                entry.key.clone(),
                entry.name.clone(),
            );
            match existing.iter().find(|policy| policy.key == entry.key) {
                None => {
                    let changes = entry
                        .snapshot()
                        .into_iter()
                        .map(|(property, value)| PlanChange::new(property, Value::Null, value))
                        .collect();
                    plans.push((item.created(changes), None));
                }
                Some(current) => {
                    let current_entry = self.portable(index, current);
                    plans.push((
                        item.compared(diff(&current_entry.snapshot(), &entry.snapshot())),
                        Some(current.clone()),
                    ));
                }
            }
        }
        Ok(plans)
    }
}

#[async_trait]
impl ConfigSection<MySqlTx> for ApprovalPoliciesSection {
    fn key(&self) -> &'static str {
        APPROVAL_POLICIES_SECTION
    }

    async fn export(&self, _scope: &AccessScope) -> Result<Option<Value>, AppError> {
        let policies = self
            .repository
            .policies(&PolicyListFilter::default())
            .await?;
        if policies.is_empty() {
            return Ok(None);
        }
        let index = self.index().await?;
        let entries: Vec<PackageApprovalPolicy> = policies
            .iter()
            .map(|policy| self.portable(&index, policy))
            .collect();
        Ok(Some(json!(entries)))
    }

    async fn validate(
        &self,
        scope: &AccessScope,
        package: &ConfigPackage,
        content: &Value,
    ) -> Result<Vec<PackageIssue>, AppError> {
        let entries: Vec<PackageApprovalPolicy> = match parse_section(content) {
            Ok(entries) => entries,
            Err(issue) => return Ok(vec![issue]),
        };
        let mut issues = Vec::new();
        if entries.len() > MAX_PACKAGE_APPROVAL_POLICIES {
            issues.push(PackageIssue::new(
                "",
                "too_many_approval_policies",
                format!(
                    "a package can have at most {MAX_PACKAGE_APPROVAL_POLICIES} approval policies"
                ),
            ));
        }
        let mut packaged_stages: HashMap<String, Vec<String>> =
            package_reference_keys(package, ConfigReferenceKind::new("workflow.pipeline"))
                .into_iter()
                .map(|key| (key, Vec::new()))
                .collect();
        for key in package_reference_keys(package, ConfigReferenceKind::new("workflow.stage")) {
            if let Some((pipeline, stage)) = key.split_once('/') {
                packaged_stages
                    .entry(pipeline.to_owned())
                    .or_default()
                    .push(stage.to_owned());
            }
        }
        let packaged_types: BTreeSet<String> = package
            .section(CUSTOM_OBJECTS_SECTION)
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|object| object.get("key").and_then(Value::as_str))
            .map(|key| format!("custom.{key}"))
            .collect();
        let index = self.index().await?;
        let mut keys = BTreeSet::new();
        for (position, entry) in entries.iter().enumerate() {
            let path = |field: &str| format!("{position}.{field}");
            check_key(&entry.key, &path("key"), &mut keys, &mut issues);
            let definition = RecordType::parse(entry.record_type.trim())
                .ok()
                .and_then(|record_type| self.registry.get(&record_type));
            let mut violations = Vec::new();
            let shape = check_shape(&self.registry, &entry.draft(), &mut violations);
            for violation in violations {
                if violation.field == "record_type"
                    && packaged_types.contains(entry.record_type.trim())
                {
                    continue;
                }
                issues.push(PackageIssue::new(
                    path(&violation.field),
                    "invalid_approval_policy",
                    violation.message,
                ));
            }
            if shape.moment == Some(ApprovalMoment::StageChange) {
                let known = entry
                    .target_stage
                    .as_deref()
                    .and_then(split_stage)
                    .is_some_and(|(pipeline, stage)| match packaged_stages.get(pipeline) {
                        Some(stages) => stages.iter().any(|key| key == stage),
                        None => index.pipeline(pipeline).is_some_and(|pipeline| {
                            pipeline.stages.iter().any(|(_, key)| key == stage)
                        }),
                    });
                if !known {
                    issues.push(PackageIssue::new(
                        path("target_stage"),
                        "unknown_stage",
                        "must be `<pipeline key>.<stage key>` of a pipeline of the package or of the organization",
                    ));
                }
            }
            if let (Some(definition), true) = (definition, shape.definition_found)
                && let Some(condition) = &shape.condition
            {
                for violation in self
                    .directory
                    .condition_violations(scope, definition, condition)
                    .await?
                {
                    let field = match condition {
                        PolicyCondition::Formula(_) => "condition_formula".to_owned(),
                        _ => "condition_filter".to_owned(),
                    };
                    issues.push(PackageIssue::new(
                        path(&field),
                        "invalid_condition",
                        format!("{}: {}", violation.field, violation.message),
                    ));
                }
                let lockable = self.directory.lockable_fields(scope, definition).await?;
                for (index, field) in entry.locked_fields.iter().enumerate() {
                    if !lockable.iter().any(|candidate| candidate == field.trim()) {
                        issues.push(PackageIssue::new(
                            path(&format!("locked_fields.{index}")),
                            "unknown_field",
                            "must be a field of the record type",
                        ));
                    }
                }
            }
            for (step_index, step) in entry.steps.iter().enumerate() {
                let step_path = |field: &str| path(&format!("steps.{step_index}.{field}"));
                match ApproverKind::parse(step.approver.trim()) {
                    Some(ApproverKind::User) => {
                        if step
                            .user_email
                            .as_deref()
                            .and_then(|email| index.user(email))
                            .is_none()
                        {
                            issues.push(PackageIssue::new(
                                step_path("user_email"),
                                "unknown_user",
                                "must be the e-mail of a member of the organization",
                            ));
                        }
                    }
                    Some(ApproverKind::Role) => {
                        if step
                            .role_key
                            .as_deref()
                            .and_then(|key| index.role_id(key.trim()))
                            .is_none()
                        {
                            issues.push(PackageIssue::new(
                                step_path("role_key"),
                                "unknown_role",
                                "must be the key of a role of the organization",
                            ));
                        }
                    }
                    Some(ApproverKind::OwnerTeamManager) | None => {}
                }
            }
        }
        Ok(issues)
    }

    async fn plan(
        &self,
        _scope: &AccessScope,
        _package: &ConfigPackage,
        content: &Value,
    ) -> Result<Vec<PlanItem>, AppError> {
        let entries = parse(content)?;
        let index = self.index().await?;
        Ok(self
            .plans(&entries, &index)
            .await?
            .into_iter()
            .map(|(item, _)| item)
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
        let entries = parse(content)?;
        let index = self.index().await?;
        let plans = self.plans(&entries, &index).await?;
        let mut next_position = self.repository.next_position().await?;
        for (entry, (item, current)) in entries.iter().zip(plans) {
            if item.action == PlanAction::Unchanged || !decisions.executes(&item) {
                continue;
            }
            let mut violations = Vec::new();
            let shape = check_shape(&self.registry, &entry.draft(), &mut violations);
            let (Some(moment), Some(condition)) = (shape.moment, shape.condition) else {
                continue;
            };
            let mut target_id = None;
            if moment == ApprovalMoment::StageChange {
                let Some((pipeline_key, stage_key)) =
                    entry.target_stage.as_deref().and_then(split_stage)
                else {
                    continue;
                };
                target_id = context
                    .resolved(
                        ConfigReferenceKind::new("workflow.stage"),
                        &format!("{pipeline_key}/{stage_key}"),
                    )
                    .or_else(|| {
                        index.pipeline(pipeline_key).and_then(|pipeline| {
                            pipeline
                                .stages
                                .iter()
                                .find(|(_, key)| key == stage_key)
                                .map(|(id, _)| *id)
                        })
                    });
                if target_id.is_none() {
                    continue;
                }
            }
            let mut steps = Vec::new();
            for (position, step) in entry.steps.iter().enumerate() {
                let Some(kind) = ApproverKind::parse(step.approver.trim()) else {
                    continue;
                };
                let approver_user_id = match kind {
                    ApproverKind::User => step
                        .user_email
                        .as_deref()
                        .and_then(|email| index.user(email)),
                    _ => None,
                };
                let approver_role_id = match kind {
                    ApproverKind::Role => step
                        .role_key
                        .as_deref()
                        .and_then(|key| index.role_id(key.trim())),
                    _ => None,
                };
                steps.push(PolicyStep {
                    id: Uuid::now_v7(),
                    position: u32::try_from(position).unwrap_or_default(),
                    name: step
                        .name
                        .as_deref()
                        .map(str::trim)
                        .filter(|name| !name.is_empty())
                        .map(str::to_owned),
                    approver_kind: kind,
                    approver_user_id,
                    approver_role_id,
                    rule: ApprovalRule::parse(step.rule.trim()).unwrap_or(ApprovalRule::Any),
                });
            }
            if steps.is_empty() {
                continue;
            }
            let policy = ApprovalPolicy {
                id: current
                    .as_ref()
                    .map_or_else(Uuid::now_v7, |current| current.id),
                key: entry.key.clone(),
                name: entry.name.trim().to_owned(),
                description: entry
                    .description
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(str::to_owned),
                record_type: entry.record_type.trim().to_owned(),
                moment,
                target_id,
                condition,
                locked_fields: entry
                    .locked_fields
                    .iter()
                    .map(|field| field.trim().to_owned())
                    .collect(),
                due_hours: entry.due_hours,
                is_active: entry.is_active,
                position: current
                    .as_ref()
                    .map_or(next_position, |current| current.position),
                steps,
                created_by: current
                    .as_ref()
                    .map_or_else(|| acting_user(scope), |current| current.created_by.clone()),
                created_at: current
                    .as_ref()
                    .map_or(context.at, |current| current.created_at),
                updated_at: context.at,
            };
            match current {
                Some(_) => self.repository.update_policy(tx, &policy).await?,
                None => {
                    self.repository.insert_policy(tx, &policy).await?;
                    next_position += 1;
                }
            }
            context.resolve(
                ConfigReferenceKind::new("approvals.policy"),
                &policy.key,
                policy.id,
            );
        }
        Ok(())
    }
}
