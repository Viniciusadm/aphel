use aphel_contract_crm::CrmPermissions;
use std::collections::{BTreeSet, HashMap, HashSet};

use chrono::{DateTime, Utc};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::domain::*;
use crate::engine::{
    self, RequestEvent, acting_user, clean_comment, record_not_found, status_change,
};
use crate::store::{ApprovalDirectory, ApprovalStore};
use aphel_kernel::Ctx;
use aphel_kernel::domain::UserId;
use aphel_kernel::errors::{AppError, FieldViolation};
use aphel_kernel::records::{RecordMutation, changes_payload, publish_mutation};
use aphel_sdk::configuration::is_config_key;
use aphel_sdk::platform::application::access::Authorizer;
use aphel_sdk::platform::application::audit::{DiffOptions, diff_values};
use aphel_sdk::platform::application::fields::{EntityDefinition, RecordTypeRegistry};
use aphel_sdk::platform::application::organization_admin::{ListWindow, Listing};
use aphel_sdk::platform::application::query::Filter;
use aphel_sdk::platform::application::shared::{RecordRef, RecordType, ensure_version};
use aphel_sdk::platform::domain::access::{AccessScope, Permission};

pub(crate) const QUOTE_RECORD_TYPE: &str = aphel_contract_crm::keys::QUOTE;
pub(crate) const STAGE_FIELD: &str = "stage_id";
const INVALID_POLICY: &str = "invalid approval policy";
const POLICY_DELETED_REASON: &str = "policy_deleted";
const POLICY_CHANGED_REASON: &str = "policy_changed";

pub(crate) fn policy_not_found() -> AppError {
    AppError::not_found_code(APPROVAL_POLICY_NOT_FOUND, "approval policy was not found")
}

pub(crate) fn policy_record(id: Uuid) -> RecordRef {
    RecordRef::new(RecordType::from_static(APPROVAL_POLICY_RECORD), id)
}

#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct PolicyDraft {
    pub(crate) key: String,
    pub(crate) name: String,
    pub(crate) description: Option<String>,
    pub(crate) record_type: String,
    pub(crate) moment: String,
    pub(crate) target_id: Option<String>,
    pub(crate) condition_kind: String,
    pub(crate) condition_filter: Option<Value>,
    pub(crate) condition_formula: Option<String>,
    pub(crate) locked_fields: Vec<String>,
    pub(crate) due_hours: Option<i64>,
    pub(crate) is_active: bool,
    pub(crate) steps: Vec<StepInput>,
}

impl PolicyDraft {
    pub(crate) fn from_policy(policy: &ApprovalPolicy) -> Self {
        Self {
            key: policy.key.clone(),
            name: policy.name.clone(),
            description: policy.description.clone(),
            record_type: policy.record_type.clone(),
            moment: policy.moment.as_str().to_owned(),
            target_id: policy.target_id.map(|id| id.to_string()),
            condition_kind: policy.condition.kind().as_str().to_owned(),
            condition_filter: policy
                .condition
                .filter()
                .and_then(|filter| serde_json::to_value(filter).ok()),
            condition_formula: policy.condition.formula().map(str::to_owned),
            locked_fields: policy.locked_fields.clone(),
            due_hours: policy.due_hours.map(i64::from),
            is_active: policy.is_active,
            steps: policy
                .steps
                .iter()
                .map(|step| StepInput {
                    name: step.name.clone(),
                    approver_kind: step.approver_kind.as_str().to_owned(),
                    approver_user_id: step
                        .approver_user_id
                        .as_ref()
                        .map(|id| id.as_str().to_owned()),
                    approver_role_id: step.approver_role_id.map(|id| id.to_string()),
                    rule: Some(step.rule.as_str().to_owned()),
                })
                .collect(),
        }
    }

    pub(crate) fn apply(&mut self, command: &PolicyCommand) {
        if let Some(name) = &command.name {
            self.name = name.clone();
        }
        if let Some(description) = &command.description {
            self.description = description.clone();
        }
        if let Some(record_type) = &command.record_type {
            self.record_type = record_type.clone();
        }
        if let Some(moment) = &command.moment {
            self.moment = moment.clone();
        }
        if let Some(target_id) = &command.target_id {
            self.target_id = target_id.clone();
        }
        if let Some(kind) = &command.condition_kind {
            self.condition_kind = kind.clone();
        }
        if let Some(filter) = &command.condition_filter {
            self.condition_filter = Some(filter.clone()).filter(|value| !value.is_null());
        }
        if let Some(formula) = &command.condition_formula {
            self.condition_formula = formula.clone();
        }
        if let Some(fields) = &command.locked_fields {
            self.locked_fields = fields.clone();
        }
        if let Some(hours) = command.due_hours {
            self.due_hours = hours;
        }
        if let Some(active) = command.is_active {
            self.is_active = active;
        }
        if let Some(steps) = &command.steps {
            self.steps = steps.clone();
        }
    }
}

fn trimmed(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

pub(crate) struct PolicyShape {
    pub(crate) definition_found: bool,
    pub(crate) moment: Option<ApprovalMoment>,
    pub(crate) condition: Option<PolicyCondition>,
}

pub(crate) fn check_shape(
    registry: &RecordTypeRegistry,
    draft: &PolicyDraft,
    violations: &mut Vec<FieldViolation>,
) -> PolicyShape {
    let definition = RecordType::parse(draft.record_type.trim())
        .ok()
        .and_then(|record_type| registry.get(&record_type));
    if definition.is_none() {
        violations.push(FieldViolation::new(
            "record_type",
            "must be a record type of the organization",
        ));
    }
    let known = ApprovalMoment::parse(draft.moment.trim())
        .and_then(|moment| crate::moments::definition(&moment));
    let moment = known.as_ref().map(|known| known.moment.clone());
    match &known {
        None => violations.push(FieldViolation::new(
            "moment",
            "must be a known approval moment",
        )),
        Some(known) if known.moment == ApprovalMoment::STAGE_CHANGE => {
            if definition
                .is_some_and(|definition| definition.system_field_by_key(STAGE_FIELD).is_none())
            {
                violations.push(FieldViolation::new(
                    "moment",
                    "stage_change is only available for record types with stages",
                ));
            }
        }
        Some(known) => {
            if !known.applies_to(draft.record_type.trim()) {
                violations.push(FieldViolation::new(
                    "moment",
                    format!("{} is not available for this record type", known.moment),
                ));
            }
        }
    }
    if let Some(known) = &known
        && known.target != MomentTarget::Policy
        && trimmed(draft.target_id.as_deref()).is_some()
    {
        violations.push(FieldViolation::new(
            "target_id",
            "must be empty for this moment",
        ));
    }
    let condition = match ConditionKind::parse(draft.condition_kind.trim()) {
        Some(ConditionKind::Always) => Some(PolicyCondition::Always),
        Some(ConditionKind::Filter) => match &draft.condition_filter {
            Some(value) => match serde_json::from_value::<Filter>(value.clone()) {
                Ok(filter) => Some(PolicyCondition::Filter(filter)),
                Err(_) => {
                    violations.push(FieldViolation::new(
                        "condition_filter",
                        "must be a valid filter",
                    ));
                    None
                }
            },
            None => {
                violations.push(FieldViolation::new(
                    "condition_filter",
                    "is required when the condition is a filter",
                ));
                None
            }
        },
        Some(ConditionKind::Formula) => match trimmed(draft.condition_formula.as_deref()) {
            Some(formula) if formula.chars().count() <= MAX_CONDITION_FORMULA_LENGTH => {
                Some(PolicyCondition::Formula(formula))
            }
            Some(_) => {
                violations.push(FieldViolation::new(
                    "condition_formula",
                    format!("must have at most {MAX_CONDITION_FORMULA_LENGTH} characters"),
                ));
                None
            }
            None => {
                violations.push(FieldViolation::new(
                    "condition_formula",
                    "is required when the condition is a formula",
                ));
                None
            }
        },
        None => {
            violations.push(FieldViolation::new(
                "condition_kind",
                "must be always, filter or formula",
            ));
            None
        }
    };
    if let Some(hours) = draft.due_hours
        && !(1..=i64::from(MAX_DUE_HOURS)).contains(&hours)
    {
        violations.push(FieldViolation::new(
            "due_hours",
            format!("must be between 1 and {MAX_DUE_HOURS}"),
        ));
    }
    if draft.steps.is_empty() || draft.steps.len() > MAX_POLICY_STEPS {
        violations.push(FieldViolation::new(
            "steps",
            format!("must have between 1 and {MAX_POLICY_STEPS} steps"),
        ));
    }
    if draft.locked_fields.len() > MAX_LOCKED_FIELDS {
        violations.push(FieldViolation::new(
            "locked_fields",
            format!("must have at most {MAX_LOCKED_FIELDS} fields"),
        ));
    }
    PolicyShape {
        definition_found: definition.is_some(),
        moment,
        condition,
    }
}

pub(crate) async fn build_policy(
    registry: &RecordTypeRegistry,
    directory: &ApprovalDirectory,
    scope: &AccessScope,
    draft: &PolicyDraft,
    base: Option<&ApprovalPolicy>,
    at: DateTime<Utc>,
) -> Result<ApprovalPolicy, AppError> {
    let mut violations = Vec::new();
    let key = draft.key.trim().to_owned();
    if base.is_none() && !is_config_key(&key) {
        violations.push(FieldViolation::new(
            "key",
            "must be snake_case: lowercase letters, digits and single underscores, starting with a letter (max 64)",
        ));
    }
    let name = draft.name.trim().to_owned();
    if name.is_empty() || name.chars().count() > MAX_POLICY_NAME_LENGTH {
        violations.push(FieldViolation::new(
            "name",
            format!("must have between 1 and {MAX_POLICY_NAME_LENGTH} characters"),
        ));
    }
    let description = trimmed(draft.description.as_deref());
    if description
        .as_ref()
        .is_some_and(|value| value.chars().count() > MAX_POLICY_DESCRIPTION_LENGTH)
    {
        violations.push(FieldViolation::new(
            "description",
            format!("must have at most {MAX_POLICY_DESCRIPTION_LENGTH} characters"),
        ));
    }
    let shape = check_shape(registry, draft, &mut violations);
    let definition: Option<&EntityDefinition> = RecordType::parse(draft.record_type.trim())
        .ok()
        .and_then(|record_type| registry.get(&record_type));
    let mut target_id = None;
    if shape.moment == Some(ApprovalMoment::STAGE_CHANGE) {
        match trimmed(draft.target_id.as_deref()).and_then(|id| Uuid::parse_str(&id).ok()) {
            Some(id) => {
                if directory.stage_names(&[id]).await?.contains_key(&id) {
                    target_id = Some(id);
                } else {
                    violations.push(FieldViolation::new(
                        "target_id",
                        "must be a stage of a pipeline of the organization",
                    ));
                }
            }
            None => violations.push(FieldViolation::new(
                "target_id",
                "must be the id of the stage that requires the approval",
            )),
        }
    }
    if let (Some(definition), Some(condition)) = (definition, &shape.condition) {
        for violation in directory
            .condition_violations(scope, definition, condition)
            .await?
        {
            let field = match condition {
                PolicyCondition::Formula(_) => "condition_formula".to_owned(),
                _ => match violation.field.strip_prefix("filter") {
                    Some(rest) => format!("condition_filter{rest}"),
                    None => format!("condition_filter.{}", violation.field),
                },
            };
            violations.push(FieldViolation::new(field, violation.message));
        }
    }
    let mut locked_fields = Vec::new();
    if let Some(definition) = definition {
        let lockable = directory.lockable_fields(scope, definition).await?;
        for (index, field) in draft.locked_fields.iter().enumerate() {
            let field = field.trim();
            if !lockable.iter().any(|candidate| candidate == field) {
                violations.push(FieldViolation::new(
                    format!("locked_fields.{index}"),
                    "must be a field of the record type",
                ));
            } else if !locked_fields.iter().any(|existing| existing == field) {
                locked_fields.push(field.to_owned());
            }
        }
    }
    let steps = build_steps(directory, definition, &draft.steps, &mut violations).await?;
    if !violations.is_empty() {
        return Err(AppError::invalid_fields(INVALID_POLICY, violations));
    }
    let (Some(moment), Some(condition)) = (shape.moment, shape.condition) else {
        return Err(AppError::invalid_fields(INVALID_POLICY, Vec::new()));
    };
    Ok(ApprovalPolicy {
        id: base.map_or_else(Uuid::now_v7, |base| base.id),
        key: base.map_or(key, |base| base.key.clone()),
        name,
        description,
        record_type: draft.record_type.trim().to_owned(),
        moment,
        target_id,
        version: base.map_or(1, |base| base.version + 1),
        condition,
        locked_fields,
        due_hours: draft.due_hours.and_then(|hours| u32::try_from(hours).ok()),
        is_active: draft.is_active,
        position: base.map_or(0, |base| base.position),
        steps,
        created_by: base
            .map(|base| base.created_by.clone())
            .unwrap_or_else(|| acting_user(scope)),
        created_at: base.map_or(at, |base| base.created_at),
        updated_at: at,
    })
}

async fn build_steps(
    directory: &ApprovalDirectory,
    definition: Option<&EntityDefinition>,
    inputs: &[StepInput],
    violations: &mut Vec<FieldViolation>,
) -> Result<Vec<PolicyStep>, AppError> {
    let mut steps = Vec::new();
    for (index, input) in inputs.iter().take(MAX_POLICY_STEPS).enumerate() {
        let path = |field: &str| format!("steps.{index}.{field}");
        let name = trimmed(input.name.as_deref());
        if name
            .as_ref()
            .is_some_and(|name| name.chars().count() > MAX_STEP_NAME_LENGTH)
        {
            violations.push(FieldViolation::new(
                path("name"),
                format!("must have at most {MAX_STEP_NAME_LENGTH} characters"),
            ));
        }
        let rule = match trimmed(input.rule.as_deref()) {
            None => ApprovalRule::Any,
            Some(rule) => ApprovalRule::parse(&rule).unwrap_or_else(|| {
                violations.push(FieldViolation::new(path("rule"), "must be any or all"));
                ApprovalRule::Any
            }),
        };
        let mut approver_user_id = None;
        let mut approver_role_id = None;
        let kind = ApproverKind::parse(input.approver_kind.trim());
        match kind {
            None => violations.push(FieldViolation::new(
                path("approver_kind"),
                "must be user, role or owner_team_manager",
            )),
            Some(ApproverKind::User) => {
                match trimmed(input.approver_user_id.as_deref()).and_then(|id| UserId::new(id).ok())
                {
                    Some(user) if directory.is_member(&user).await? => {
                        approver_user_id = Some(user);
                    }
                    _ => violations.push(FieldViolation::new(
                        path("approver_user_id"),
                        "must be a member of the organization",
                    )),
                }
            }
            Some(ApproverKind::Role) => {
                match trimmed(input.approver_role_id.as_deref())
                    .and_then(|id| Uuid::parse_str(&id).ok())
                {
                    Some(role) if directory.role_exists(role).await? => {
                        approver_role_id = Some(role);
                    }
                    _ => violations.push(FieldViolation::new(
                        path("approver_role_id"),
                        "must be a role of the organization",
                    )),
                }
            }
            Some(ApproverKind::OwnerTeamManager) => {
                if definition.is_some_and(|definition| definition.owner_column.is_none()) {
                    violations.push(FieldViolation::new(
                        path("approver_kind"),
                        "records of this type have no owner",
                    ));
                }
            }
        }
        steps.push(PolicyStep {
            id: Uuid::now_v7(),
            position: u32::try_from(index).unwrap_or_default(),
            name,
            approver_kind: kind.unwrap_or(ApproverKind::OwnerTeamManager),
            approver_user_id,
            approver_role_id,
            rule,
        });
    }
    Ok(steps)
}

pub(crate) async fn list_policies(
    ctx: &Ctx<'_>,
    filter: PolicyListFilter,
) -> Result<Vec<ApprovalPolicy>, AppError> {
    ctx.scope.require(&Permission::APPROVALS_MANAGE)?;
    ApprovalStore::of(ctx).policies(&filter).await
}

pub(crate) async fn get_policy(ctx: &Ctx<'_>, id: &str) -> Result<ApprovalPolicy, AppError> {
    ctx.scope.require(&Permission::APPROVALS_MANAGE)?;
    find_policy(ctx, id).await
}

async fn find_policy(ctx: &Ctx<'_>, id: &str) -> Result<ApprovalPolicy, AppError> {
    let id = Uuid::parse_str(id.trim()).map_err(|_| policy_not_found())?;
    ApprovalStore::of(ctx)
        .policy(id)
        .await?
        .ok_or_else(policy_not_found)
}

pub async fn create_policy(
    ctx: &Ctx<'_>,
    command: PolicyCommand,
) -> Result<ApprovalPolicy, AppError> {
    ctx.scope.require(&Permission::APPROVALS_MANAGE)?;
    let mut draft = PolicyDraft {
        key: command.key.clone().unwrap_or_default(),
        condition_kind: ConditionKind::Always.as_str().to_owned(),
        is_active: true,
        ..PolicyDraft::default()
    };
    draft.apply(&command);
    let now = ctx.now();
    let mut policy = build_policy(
        ctx.registry(),
        &ApprovalDirectory::of(ctx),
        ctx.scope,
        &draft,
        None,
        now,
    )
    .await?;
    if ApprovalStore::of(ctx)
        .policy_by_key(&policy.key)
        .await?
        .is_some()
    {
        return Err(key_taken());
    }
    policy.position = ApprovalStore::of(ctx).next_position().await?;
    let changes = diff_values(&Value::Null, &policy.snapshot(), &DiffOptions::default())?;
    let mut tx = ctx.begin().await?;
    ApprovalStore::of(ctx)
        .insert_policy(&mut tx, &policy)
        .await?;
    let mut payload = changes_payload(&changes);
    payload["policy"] = policy.snapshot();
    publish_mutation(
        ctx.events(),
        ctx.audit(),
        &mut tx,
        ctx.scope,
        now,
        RecordMutation {
            record: policy_record(policy.id),
            event_type: APPROVAL_POLICY_CREATED,
            payload,
            changes,
            ip: command.ip,
        },
    )
    .await?;
    tx.commit().await?;
    Ok(policy)
}

pub(crate) async fn update_policy(
    ctx: &Ctx<'_>,
    id: &str,
    command: PolicyCommand,
) -> Result<ApprovalPolicy, AppError> {
    ctx.scope.require(&Permission::APPROVALS_MANAGE)?;
    let current = find_policy(ctx, id).await?;
    ensure_version(command.expected_updated_at, current.updated_at)?;
    if let Some(key) = command.key.as_deref()
        && key.trim() != current.key
    {
        return Err(AppError::invalid_fields(
            INVALID_POLICY,
            vec![FieldViolation::new("key", "cannot be changed")],
        ));
    }
    let mut draft = PolicyDraft::from_policy(&current);
    draft.apply(&command);
    let now = ctx.now();
    let policy = build_policy(
        ctx.registry(),
        &ApprovalDirectory::of(ctx),
        ctx.scope,
        &draft,
        Some(&current),
        now,
    )
    .await?;
    let changes = diff_values(
        &current.snapshot(),
        &policy.snapshot(),
        &DiffOptions::default(),
    )?;
    if changes.is_empty() {
        return Ok(current);
    }
    let mut tx = ctx.begin().await?;
    for request in ApprovalStore::of(ctx)
        .pending_for_policy(&mut tx, policy.id)
        .await?
    {
        engine::cancel(
            ctx,
            &mut tx,
            request,
            Some(POLICY_CHANGED_REASON.to_owned()),
            command.ip.clone(),
        )
        .await?;
    }
    ApprovalStore::of(ctx)
        .update_policy(&mut tx, &policy)
        .await?;
    let mut payload = changes_payload(&changes);
    payload["policy"] = policy.snapshot();
    publish_mutation(
        ctx.events(),
        ctx.audit(),
        &mut tx,
        ctx.scope,
        now,
        RecordMutation {
            record: policy_record(policy.id),
            event_type: APPROVAL_POLICY_UPDATED,
            payload,
            changes,
            ip: command.ip,
        },
    )
    .await?;
    tx.commit().await?;
    Ok(policy)
}

pub(crate) async fn delete_policy(
    ctx: &Ctx<'_>,
    id: &str,
    expected_updated_at: Option<DateTime<Utc>>,
    ip: Option<String>,
) -> Result<(), AppError> {
    ctx.scope.require(&Permission::APPROVALS_MANAGE)?;
    let current = find_policy(ctx, id).await?;
    ensure_version(expected_updated_at, current.updated_at)?;
    let now = ctx.now();
    let changes = diff_values(&current.snapshot(), &Value::Null, &DiffOptions::default())?;
    let mut tx = ctx.begin().await?;
    for request in ApprovalStore::of(ctx)
        .pending_for_policy(&mut tx, current.id)
        .await?
    {
        engine::cancel(
            ctx,
            &mut tx,
            request,
            Some(POLICY_DELETED_REASON.to_owned()),
            ip.clone(),
        )
        .await?;
    }
    ApprovalStore::of(ctx)
        .delete_policy(&mut tx, current.id)
        .await?;
    publish_mutation(
        ctx.events(),
        ctx.audit(),
        &mut tx,
        ctx.scope,
        now,
        RecordMutation {
            record: policy_record(current.id),
            event_type: APPROVAL_POLICY_DELETED,
            payload: json!({ "key": current.key, "name": current.name }),
            changes,
            ip,
        },
    )
    .await?;
    Ok(tx.commit().await?)
}

pub(crate) async fn reorder_policies(
    ctx: &Ctx<'_>,
    ids: &[String],
    ip: Option<String>,
) -> Result<Vec<ApprovalPolicy>, AppError> {
    ctx.scope.require(&Permission::APPROVALS_MANAGE)?;
    let policies = ApprovalStore::of(ctx)
        .policies(&PolicyListFilter::default())
        .await?;
    let mut seen = HashSet::new();
    let mut ordered = Vec::new();
    for (index, id) in ids.iter().enumerate() {
        let policy = Uuid::parse_str(id.trim())
            .ok()
            .and_then(|id| policies.iter().find(|policy| policy.id == id));
        match policy {
            Some(policy) if seen.insert(policy.id) => ordered.push(policy.id),
            _ => {
                return Err(AppError::invalid_fields(
                    INVALID_POLICY,
                    vec![FieldViolation::new(
                        format!("ids.{index}"),
                        "must be a distinct approval policy id",
                    )],
                ));
            }
        }
    }
    for policy in &policies {
        if seen.insert(policy.id) {
            ordered.push(policy.id);
        }
    }
    let positions: Vec<(Uuid, i32)> = ordered
        .iter()
        .enumerate()
        .map(|(index, id)| (*id, i32::try_from(index).unwrap_or(i32::MAX)))
        .collect();
    let old: Vec<String> = policies.iter().map(|policy| policy.key.clone()).collect();
    let new: Vec<String> = ordered
        .iter()
        .filter_map(|id| policies.iter().find(|policy| policy.id == *id))
        .map(|policy| policy.key.clone())
        .collect();
    if old != new {
        let now = ctx.now();
        let mut tx = ctx.begin().await?;
        ApprovalStore::of(ctx)
            .set_positions(&mut tx, &positions, now)
            .await?;
        let changes = vec![aphel_sdk::platform::application::shared::FieldChange::new(
            "order",
            json!(old),
            json!(new),
        )];
        publish_mutation(
            ctx.events(),
            ctx.audit(),
            &mut tx,
            ctx.scope,
            now,
            RecordMutation {
                record: RecordRef::new(
                    RecordType::from_static(APPROVAL_POLICY_RECORD),
                    Uuid::nil(),
                ),
                event_type: APPROVAL_POLICIES_REORDERED,
                payload: changes_payload(&changes),
                changes,
                ip,
            },
        )
        .await?;
        tx.commit().await?;
    }
    ApprovalStore::of(ctx)
        .policies(&PolicyListFilter::default())
        .await
}

fn key_taken() -> AppError {
    AppError::conflict_code(
        APPROVAL_POLICY_KEY_TAKEN,
        "another approval policy already uses this key",
    )
}

const INVALID_REQUEST: &str = "invalid approval request";

pub(crate) fn request_not_found() -> AppError {
    AppError::not_found_code(APPROVAL_REQUEST_NOT_FOUND, "approval request was not found")
}

fn not_pending() -> AppError {
    AppError::conflict_code(
        APPROVAL_NOT_PENDING,
        "the approval request is no longer pending",
    )
}

fn not_approver() -> AppError {
    AppError::forbidden_code(
        APPROVAL_NOT_APPROVER,
        "you are not an approver of the current step of this request",
    )
}

fn invalid(field: &str, message: &str) -> AppError {
    AppError::invalid_fields(INVALID_REQUEST, vec![FieldViolation::new(field, message)])
}

fn manages(scope: &AccessScope) -> bool {
    scope.require(&Permission::APPROVALS_MANAGE).is_ok()
}

fn parse_record(record_type: &str, record_id: &str) -> Result<RecordRef, AppError> {
    let record_type = RecordType::parse(record_type.trim())
        .map_err(|_| invalid("record_type", "must be a record type"))?;
    let record_id = Uuid::parse_str(record_id.trim())
        .map_err(|_| invalid("record_id", "must be a valid id"))?;
    Ok(RecordRef::new(record_type, record_id))
}

async fn readable(ctx: &Ctx<'_>, record: &RecordRef) -> Result<bool, AppError> {
    let Some(definition) = ctx.registry().get(&record.record_type) else {
        return Ok(false);
    };
    let Ok(visibility) = ctx.scope.record_filter(&definition.read_permission) else {
        return Ok(false);
    };
    Ok(ApprovalDirectory::of(ctx)
        .record_state(ctx.scope, definition, record.record_id, Some(&visibility))
        .await?
        .is_some())
}

pub(crate) async fn list_requests(
    ctx: &Ctx<'_>,
    query: RequestListQuery,
    window: ListWindow,
) -> Result<Listing<RequestView>, AppError> {
    let mut filter = RequestListFilter {
        status: query.status,
        ..RequestListFilter::default()
    };
    match (query.record_type.as_deref(), query.record_id.as_deref()) {
        (Some(record_type), Some(record_id)) => {
            let record = parse_record(record_type, record_id)?;
            let definition = ctx.registry().require(record.record_type.as_str())?;
            ctx.scope.record_filter(&definition.read_permission)?;
            if !readable(ctx, &record).await? {
                return Err(record_not_found());
            }
            filter.record = Some(record);
        }
        (None, None) => {
            let box_name = match query
                .box_name
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
            {
                None => RequestBox::default(),
                Some(value) => RequestBox::parse(value).ok_or_else(|| {
                    invalid("box", "must be pending_for_me, requested_by_me or all")
                })?,
            };
            match box_name {
                RequestBox::PendingForMe => {
                    filter.awaiting = Some(ctx.scope.user_id().clone());
                    filter.status = Some(ApprovalStatus::Pending);
                }
                RequestBox::RequestedByMe => {
                    filter.requested_by = Some(ctx.scope.user_id().clone());
                }
                RequestBox::All => {
                    ctx.scope.require(&Permission::APPROVALS_MANAGE)?;
                }
            }
        }
        _ => {
            return Err(invalid(
                "record_id",
                "record_type and record_id must be sent together",
            ));
        }
    }
    let listing = ApprovalStore::of(ctx).requests(&filter, window).await?;
    Ok(Listing {
        items: views(ctx, listing.items).await?,
        total: listing.total,
    })
}

pub(crate) async fn pending_count(ctx: &Ctx<'_>) -> Result<u64, AppError> {
    ApprovalStore::of(ctx)
        .pending_count(ctx.scope.user_id())
        .await
}

pub(crate) async fn get_request(ctx: &Ctx<'_>, id: &str) -> Result<RequestView, AppError> {
    let record = find_visible(ctx, id).await?;
    view(ctx, record).await
}

async fn find_visible(ctx: &Ctx<'_>, id: &str) -> Result<ApprovalRequestRecord, AppError> {
    let id = Uuid::parse_str(id.trim()).map_err(|_| request_not_found())?;
    let record = ApprovalStore::of(ctx)
        .request(id)
        .await?
        .ok_or_else(request_not_found)?;
    let user = ctx.scope.user_id();
    let involved = record.request.requested_by.as_ref() == Some(user)
        || record
            .approvers
            .iter()
            .any(|approver| &approver.user_id == user);
    if involved || manages(ctx.scope) || readable(ctx, &record.request.record).await? {
        Ok(record)
    } else {
        Err(request_not_found())
    }
}

pub(crate) async fn submit(
    ctx: &Ctx<'_>,
    command: SubmitApprovalCommand,
    only_policy: Option<&str>,
) -> Result<SubmissionView, AppError> {
    let record = parse_record(&command.record_type, &command.record_id)?;
    let moment = ApprovalMoment::parse(command.moment.trim())
        .filter(|moment| crate::moments::definition(moment).is_some())
        .ok_or_else(|| invalid("moment", "must be a known approval moment"))?;
    let target_id = match command
        .target_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        None => None,
        Some(value) => {
            Some(Uuid::parse_str(value).map_err(|_| invalid("target_id", "must be a valid id"))?)
        }
    };
    let submission = ApprovalSubmission {
        checkpoint: ApprovalCheckpoint {
            record,
            moment,
            target_id,
        },
        comment: command.comment,
        ip: command.ip,
    };
    let mut tx = ctx.begin().await?;
    let outcome = engine::submit(ctx, &mut tx, &submission, only_policy).await?;
    tx.commit().await?;
    let (name, summaries) = match outcome {
        SubmissionOutcome::NotRequired => ("not_required", Vec::new()),
        SubmissionOutcome::Pending(requests) => ("pending", requests),
        SubmissionOutcome::Approved(requests) => ("approved", requests),
    };
    let mut records = Vec::new();
    for summary in summaries {
        if let Some(record) = ApprovalStore::of(ctx).request(summary.id).await? {
            records.push(record);
        }
    }
    Ok(SubmissionView {
        outcome: name,
        requests: views(ctx, records).await?,
    })
}

pub(crate) async fn approve(
    ctx: &Ctx<'_>,
    id: &str,
    command: DecideCommand,
) -> Result<RequestView, AppError> {
    decide(ctx, id, ApprovalDecision::Approved, command).await
}

pub(crate) async fn reject(
    ctx: &Ctx<'_>,
    id: &str,
    command: DecideCommand,
) -> Result<RequestView, AppError> {
    decide(ctx, id, ApprovalDecision::Rejected, command).await
}

async fn decide(
    ctx: &Ctx<'_>,
    id: &str,
    decision: ApprovalDecision,
    command: DecideCommand,
) -> Result<RequestView, AppError> {
    let id = Uuid::parse_str(id.trim()).map_err(|_| request_not_found())?;
    let mut violations = Vec::new();
    let comment = clean_comment(command.comment.as_deref(), "comment", &mut violations);
    if decision == ApprovalDecision::Rejected && comment.is_none() {
        violations.push(FieldViolation::new(
            "comment",
            "is required to reject a request",
        ));
    }
    if !violations.is_empty() {
        return Err(AppError::invalid_fields(INVALID_REQUEST, violations));
    }
    let user = acting_user(ctx.scope).ok_or_else(not_approver)?;
    let mut tx = ctx.begin().await?;
    let record = ApprovalStore::of(ctx)
        .lock_request(&mut tx, id)
        .await?
        .ok_or_else(request_not_found)?;
    if record.request.status != ApprovalStatus::Pending {
        return Err(not_pending());
    }
    if !record.awaits(&user) {
        return Err(not_approver());
    }
    let now = ctx.now();
    let step = record.request.current_step;
    ApprovalStore::of(ctx)
        .record_decision(
            &mut tx,
            &DecisionEntry {
                request_id: id,
                step,
                user_id: &user,
                decision,
                comment: comment.as_deref(),
                at: now,
            },
        )
        .await?;
    let mut request = record.request.clone();
    let decision_change = vec![aphel_sdk::platform::application::shared::FieldChange::new(
        "decision",
        serde_json::Value::Null,
        serde_json::json!(decision.as_str()),
    )];
    match decision {
        ApprovalDecision::Rejected => {
            request.status = ApprovalStatus::Rejected;
            request.resolved_at = Some(now);
            request.resolved_by = Some(user.clone());
            request.resolution_comment = comment;
            request.updated_at = now;
            ApprovalStore::of(ctx)
                .save_request_state(&mut tx, &request)
                .await?;
            let mut changes = status_change(ApprovalStatus::Pending, ApprovalStatus::Rejected);
            changes.extend(decision_change);
            engine::publish(
                ctx,
                &mut tx,
                RequestEvent::new(APPROVAL_REJECTED, &request)
                    .step(step)
                    .changes(changes)
                    .ip(command.ip),
            )
            .await?;
        }
        ApprovalDecision::Approved => {
            let total = record.step_approvers(step).count();
            let approvals = record
                .step_approvers(step)
                .filter(|approver| approver.decision == Some(ApprovalDecision::Approved))
                .count()
                + 1;
            let rule = request
                .current()
                .map(|snapshot| snapshot.rule())
                .unwrap_or(ApprovalRule::Any);
            if !rule.is_satisfied(approvals, total) {
                engine::publish(
                    ctx,
                    &mut tx,
                    RequestEvent::new(APPROVAL_DECISION_RECORDED, &request)
                        .step(step)
                        .changes(decision_change)
                        .ip(command.ip),
                )
                .await?;
            } else if (step as usize) + 1 < request.steps.len() {
                request.current_step = step + 1;
                request.updated_at = now;
                ApprovalStore::of(ctx)
                    .save_request_state(&mut tx, &request)
                    .await?;
                let owner = engine::owner_of(ctx, &mut tx, &request.record).await?;
                let approvers =
                    engine::start_step(ctx, &mut tx, &request, step + 1, owner.as_ref()).await?;
                let mut changes = decision_change;
                changes.push(aphel_sdk::platform::application::shared::FieldChange::new(
                    "current_step",
                    serde_json::json!(step),
                    serde_json::json!(step + 1),
                ));
                engine::publish(
                    ctx,
                    &mut tx,
                    RequestEvent::new(APPROVAL_STEP_ADVANCED, &request)
                        .step(step + 1)
                        .approvers(&approvers)
                        .changes(changes)
                        .ip(command.ip),
                )
                .await?;
            } else {
                request.status = ApprovalStatus::Approved;
                request.resolved_at = Some(now);
                request.resolved_by = Some(user.clone());
                request.resolution_comment = comment;
                request.updated_at = now;
                ApprovalStore::of(ctx)
                    .save_request_state(&mut tx, &request)
                    .await?;
                let mut changes = status_change(ApprovalStatus::Pending, ApprovalStatus::Approved);
                changes.extend(decision_change);
                engine::publish(
                    ctx,
                    &mut tx,
                    RequestEvent::new(APPROVAL_APPROVED, &request)
                        .step(step)
                        .changes(changes)
                        .ip(command.ip),
                )
                .await?;
            }
        }
    }
    tx.commit().await?;
    let record = ApprovalStore::of(ctx)
        .request(id)
        .await?
        .ok_or_else(request_not_found)?;
    view(ctx, record).await
}

pub(crate) async fn cancel(
    ctx: &Ctx<'_>,
    id: &str,
    command: DecideCommand,
) -> Result<RequestView, AppError> {
    let id = Uuid::parse_str(id.trim()).map_err(|_| request_not_found())?;
    let mut violations = Vec::new();
    let comment = clean_comment(command.comment.as_deref(), "comment", &mut violations);
    if !violations.is_empty() {
        return Err(AppError::invalid_fields(INVALID_REQUEST, violations));
    }
    let mut tx = ctx.begin().await?;
    let record = ApprovalStore::of(ctx)
        .lock_request(&mut tx, id)
        .await?
        .ok_or_else(request_not_found)?;
    let requester = record.request.requested_by.as_ref() == Some(ctx.scope.user_id());
    if !requester && !manages(ctx.scope) {
        if record
            .approvers
            .iter()
            .any(|approver| &approver.user_id == ctx.scope.user_id())
        {
            ctx.scope.require(&Permission::APPROVALS_MANAGE)?;
        }
        return Err(request_not_found());
    }
    if record.request.status != ApprovalStatus::Pending {
        return Err(not_pending());
    }
    engine::cancel(ctx, &mut tx, record.request, comment, command.ip).await?;
    tx.commit().await?;
    let record = ApprovalStore::of(ctx)
        .request(id)
        .await?
        .ok_or_else(request_not_found)?;
    view(ctx, record).await
}

pub(crate) async fn revoke(
    ctx: &Ctx<'_>,
    id: &str,
    command: DecideCommand,
) -> Result<RequestView, AppError> {
    ctx.scope.require(&Permission::APPROVALS_MANAGE)?;
    let id = Uuid::parse_str(id.trim()).map_err(|_| request_not_found())?;
    let mut violations = Vec::new();
    let comment = clean_comment(command.comment.as_deref(), "comment", &mut violations);
    if !violations.is_empty() {
        return Err(AppError::invalid_fields(INVALID_REQUEST, violations));
    }
    let mut tx = ctx.begin().await?;
    let record = ApprovalStore::of(ctx)
        .lock_request(&mut tx, id)
        .await?
        .ok_or_else(request_not_found)?;
    if !record.request.is_unconsumed_approval() {
        return Err(AppError::conflict_code(
            "approval_not_revocable",
            "only an approval that was not used yet can be revoked",
        ));
    }
    engine::cancel(ctx, &mut tx, record.request, comment, command.ip).await?;
    tx.commit().await?;
    let record = ApprovalStore::of(ctx)
        .request(id)
        .await?
        .ok_or_else(request_not_found)?;
    view(ctx, record).await
}

pub(crate) async fn view(
    ctx: &Ctx<'_>,
    record: ApprovalRequestRecord,
) -> Result<RequestView, AppError> {
    views(ctx, vec![record])
        .await?
        .pop()
        .ok_or_else(request_not_found)
}

pub(crate) async fn views(
    ctx: &Ctx<'_>,
    records: Vec<ApprovalRequestRecord>,
) -> Result<Vec<RequestView>, AppError> {
    let directory = ApprovalDirectory::of(ctx);
    let mut users: BTreeSet<String> = BTreeSet::new();
    let mut by_type: HashMap<RecordType, Vec<Uuid>> = HashMap::new();
    let mut stages: Vec<Uuid> = Vec::new();
    for record in &records {
        let request = &record.request;
        users.extend(request.requested_by.iter().map(|id| id.as_str().to_owned()));
        users.extend(request.resolved_by.iter().map(|id| id.as_str().to_owned()));
        users.extend(
            record
                .approvers
                .iter()
                .map(|approver| approver.user_id.as_str().to_owned()),
        );
        by_type
            .entry(request.record.record_type.clone())
            .or_default()
            .push(request.record.record_id);
        if request.moment == ApprovalMoment::STAGE_CHANGE
            && let Some(target) = request.target_id
        {
            stages.push(target);
        }
    }
    let ids: Vec<UserId> = users
        .into_iter()
        .filter_map(|id| UserId::new(id).ok())
        .collect();
    let names = directory.names(&ids).await?;
    let mut titles: HashMap<(RecordType, Uuid), String> = HashMap::new();
    for (record_type, ids) in by_type {
        let Some(definition) = ctx.registry().get(&record_type) else {
            continue;
        };
        for (id, title) in directory.record_titles(definition, &ids).await? {
            titles.insert((record_type.clone(), id), title);
        }
    }
    let stage_names = if stages.is_empty() {
        HashMap::new()
    } else {
        directory.stage_names(&stages).await?
    };
    let now = ctx.now();
    let manages = manages(ctx.scope);
    Ok(records
        .into_iter()
        .map(|record| {
            let request = &record.request;
            let pending = request.status == ApprovalStatus::Pending;
            let can_decide = record.awaits(ctx.scope.user_id());
            let can_cancel =
                pending && (manages || request.requested_by.as_ref() == Some(ctx.scope.user_id()));
            let is_overdue = pending && request.due_at.is_some_and(|due| due < now);
            let record_title = titles
                .get(&(request.record.record_type.clone(), request.record.record_id))
                .cloned();
            let target_name = request
                .target_id
                .and_then(|id| stage_names.get(&id).cloned());
            RequestView {
                record_title,
                target_name,
                names: names.clone(),
                can_decide,
                can_cancel,
                is_overdue,
                record,
            }
        })
        .collect())
}
