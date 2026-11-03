use serde_json::{Value, json};
use uuid::Uuid;

use crate::domain::{
    BusinessType, CompleteOnboardingCommand, OnboardingAnswers, OnboardingState, OnboardingStatus,
    Recommendation, SaveAnswersCommand, TeamSize, recommend,
};
use crate::store::OnboardingStore;
use ::config_packages::application::TemplateCatalog;
use ::config_packages::infrastructure::EmbeddedTemplates;
use aphel_kernel::Ctx;
use aphel_kernel::application::access::Authorizer;
use aphel_kernel::application::shared::{
    Actor, AuditEntry, AuditLog, DomainEvent, EventPublisher, FieldChange, RecordRef,
};
use aphel_kernel::domain::access::Permission;
use aphel_kernel::domain::organization_profile::DEFAULT_LOCALE;
use aphel_kernel::errors::{AppError, FieldViolation};

pub const ONBOARDING_NOT_PENDING: &str = "onboarding_not_pending";
pub(crate) const ONBOARDING_COMPLETED: &str = "onboarding.completed";
pub(crate) const ONBOARDING_SKIPPED: &str = "onboarding.skipped";
pub(crate) const ONBOARDING_REOPENED: &str = "onboarding.reopened";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OnboardingView {
    pub(crate) state: OnboardingState,
    pub(crate) answers: OnboardingAnswers,
    pub(crate) recommendation: Option<Recommendation>,
}

impl From<OnboardingState> for OnboardingView {
    fn from(state: OnboardingState) -> Self {
        let answers = if state.answers.is_empty() {
            state.installer_answers.clone()
        } else {
            state.answers.clone()
        };
        Self {
            recommendation: recommend(&answers),
            answers,
            state,
        }
    }
}

pub(crate) async fn get(ctx: &Ctx<'_>) -> Result<OnboardingView, AppError> {
    ctx.scope.require(&Permission::SETTINGS_MANAGE)?;
    Ok(load_state(ctx).await?.into())
}

pub(crate) async fn save_answers(
    ctx: &Ctx<'_>,
    command: SaveAnswersCommand,
) -> Result<OnboardingView, AppError> {
    ctx.scope.require(&Permission::SETTINGS_MANAGE)?;
    let answers = answers_from(command);
    let current = load_state(ctx).await?;
    ensure_pending(&current)?;
    let state = OnboardingState {
        answers,
        updated_at: Some(ctx.now()),
        ..current
    };

    let mut tx = ctx.begin().await?;
    OnboardingStore::of(ctx).save(&mut tx, &state).await?;
    tx.commit().await?;
    Ok(state.into())
}

pub async fn complete(
    ctx: &Ctx<'_>,
    command: CompleteOnboardingCommand,
) -> Result<OnboardingView, AppError> {
    ctx.scope.require(&Permission::SETTINGS_MANAGE)?;
    let current = load_state(ctx).await?;
    ensure_pending(&current)?;
    let (template_key, install_id) = resolve_choice(ctx, &command).await?;
    let now = ctx.now();
    let state = OnboardingState {
        status: OnboardingStatus::Completed,
        template_key,
        install_id,
        completed_by: Some(ctx.scope.user_id().clone()),
        completed_at: Some(now),
        skipped_by: None,
        skipped_at: None,
        updated_at: Some(now),
        ..current.clone()
    };
    let payload = json!({
        "template_key": state.template_key,
        "install_id": state.install_id.map(|id| id.to_string()),
        "answers": answers_json(&state.answers),
    });
    transition(
        ctx,
        ONBOARDING_COMPLETED,
        &current,
        &state,
        payload,
        command.ip,
    )
    .await?;
    Ok(state.into())
}

pub(crate) async fn skip(ctx: &Ctx<'_>, ip: Option<String>) -> Result<OnboardingView, AppError> {
    ctx.scope.require(&Permission::SETTINGS_MANAGE)?;
    let current = load_state(ctx).await?;
    ensure_pending(&current)?;
    let now = ctx.now();
    let state = OnboardingState {
        status: OnboardingStatus::Skipped,
        skipped_by: Some(ctx.scope.user_id().clone()),
        skipped_at: Some(now),
        updated_at: Some(now),
        ..current.clone()
    };
    transition(ctx, ONBOARDING_SKIPPED, &current, &state, json!({}), ip).await?;
    Ok(state.into())
}

pub(crate) async fn reopen(ctx: &Ctx<'_>, ip: Option<String>) -> Result<OnboardingView, AppError> {
    ctx.scope.require(&Permission::SETTINGS_MANAGE)?;
    let current = load_state(ctx).await?;
    if current.status == OnboardingStatus::Pending {
        return Ok(current.into());
    }
    let state = OnboardingState {
        status: OnboardingStatus::Pending,
        answers: current.answers.clone(),
        installer_answers: current.installer_answers.clone(),
        updated_at: Some(ctx.now()),
        ..OnboardingState::default()
    };
    let payload = json!({ "previous_status": current.status.as_str() });
    transition(ctx, ONBOARDING_REOPENED, &current, &state, payload, ip).await?;
    Ok(state.into())
}

async fn load_state(ctx: &Ctx<'_>) -> Result<OnboardingState, AppError> {
    let store = OnboardingStore::of(ctx);
    let mut state = store.find().await?.unwrap_or_default();
    state.installer_answers = store.installer_answers().await?;
    Ok(state)
}

async fn resolve_choice(
    ctx: &Ctx<'_>,
    command: &CompleteOnboardingCommand,
) -> Result<(Option<String>, Option<Uuid>), AppError> {
    let mut violations = Vec::new();
    let requested_key = command
        .template_key
        .as_deref()
        .map(str::trim)
        .filter(|key| !key.is_empty());
    if let Some(key) = requested_key
        && EmbeddedTemplates.template(key, DEFAULT_LOCALE).is_none()
    {
        violations.push(FieldViolation::new(
            "template_key",
            "template was not found",
        ));
    }
    let install = match command.install_id.as_deref() {
        None => None,
        Some(raw) => match Uuid::parse_str(raw.trim()) {
            Err(_) => {
                violations.push(FieldViolation::new("install_id", "must be a UUID"));
                None
            }
            Ok(id) => {
                let found = OnboardingStore::of(ctx).find_install(id).await?;
                if found.is_none() {
                    violations.push(FieldViolation::new(
                        "install_id",
                        "installation was not found",
                    ));
                }
                found
            }
        },
    };
    if let (Some(key), Some(install)) = (requested_key, install.as_ref())
        && install
            .template_key
            .as_deref()
            .is_some_and(|installed| installed != key)
    {
        violations.push(FieldViolation::new(
            "template_key",
            "does not match the template of the installation",
        ));
    }
    if !violations.is_empty() {
        return Err(AppError::invalid_fields(
            "invalid onboarding choice",
            violations,
        ));
    }
    let template_key = requested_key.map(str::to_owned).or_else(|| {
        install
            .as_ref()
            .and_then(|install| install.template_key.clone())
    });
    Ok((template_key, install.map(|install| install.id)))
}

async fn transition(
    ctx: &Ctx<'_>,
    action: &str,
    before: &OnboardingState,
    after: &OnboardingState,
    payload: Value,
    ip: Option<String>,
) -> Result<(), AppError> {
    let record = RecordRef::organization();
    let actor = Actor::from_scope(ctx.scope);
    let now = after.updated_at.unwrap_or_else(|| ctx.now());
    let event = DomainEvent::new(actor.clone(), action, now)
        .with_record(record.clone())
        .with_payload(payload);
    let entry = AuditEntry::new(actor, action, now)
        .with_record(record)
        .with_changes(state_changes(before, after))
        .with_ip(ip);

    let mut tx = ctx.begin().await?;
    OnboardingStore::of(ctx).save(&mut tx, after).await?;
    ctx.events().publish(&mut tx, event).await?;
    ctx.audit().record(&mut tx, entry).await?;
    Ok(tx.commit().await?)
}

fn ensure_pending(state: &OnboardingState) -> Result<(), AppError> {
    if state.status == OnboardingStatus::Pending {
        return Ok(());
    }
    Err(AppError::conflict_code(
        ONBOARDING_NOT_PENDING,
        "the onboarding assistant is not open; reopen it first",
    ))
}

fn answers_from(command: SaveAnswersCommand) -> OnboardingAnswers {
    let mut offering = command.offering;
    offering.sort();
    offering.dedup();
    OnboardingAnswers {
        business_type: command.business_type,
        offering,
        team_size: command.team_size,
    }
}

pub(crate) fn answers_json(answers: &OnboardingAnswers) -> Value {
    json!({
        "business_type": answers.business_type.map(BusinessType::as_str),
        "offering": answers.offering.iter().map(|offering| offering.as_str()).collect::<Vec<_>>(),
        "team_size": answers.team_size.map(TeamSize::as_str),
    })
}

fn state_changes(before: &OnboardingState, after: &OnboardingState) -> Vec<FieldChange> {
    let mut changes = Vec::new();
    let mut push = |field: &str, old: Value, new: Value| {
        if old != new {
            changes.push(FieldChange::new(field, old, new));
        }
    };
    push(
        "status",
        json!(before.status.as_str()),
        json!(after.status.as_str()),
    );
    push(
        "template_key",
        json!(before.template_key),
        json!(after.template_key),
    );
    push(
        "install_id",
        json!(before.install_id.map(|id| id.to_string())),
        json!(after.install_id.map(|id| id.to_string())),
    );
    changes
}
