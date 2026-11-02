use std::time::Instant;

use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::conditions::{Partial, TriggerEvaluation};
use crate::hooks::{RunningAction, after_action, before_action};
use crate::model::{RunStep, truncate_error};
use aphel_contract_automation::application::actions::{ActionContext, ActionRegistry};
use aphel_contract_automation::application::definition::{
    AutomationDefinition, StepBody, StepSpec,
};
use aphel_contract_automation::application::gateway::AutomationGateway;
use aphel_contract_automation::application::templates::TemplateScope;
use aphel_contract_automation::application::time::duration::{DURATION_MESSAGE, parse_duration};
use aphel_contract_automation::application::time::waits::{
    WAIT_UNTIL_POLL_MINUTES, WaitTarget, parse_timestamp, timestamp,
};
use aphel_contract_automation::domain::StepStatus;
use aphel_kernel::Ctx;
use aphel_kernel::application::access::Authorizer;
use aphel_kernel::application::fields::FieldSet;
use aphel_kernel::application::query::Filter;
use aphel_kernel::application::security::Clock;
use aphel_kernel::application::shared::RecordRef;
use aphel_kernel::errors::AppError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EngineMode {
    Live,
    DryRun,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum EngineOutcome {
    Completed,
    Failed {
        error: String,
        retry: bool,
    },
    Waiting {
        step_id: String,
        resume_at: DateTime<Utc>,
    },
}

#[async_trait]
pub(crate) trait StepRecorder: Send + Sync {
    async fn record(&self, step: &RunStep) -> Result<(), AppError>;
}

pub(crate) struct DiscardSteps;

#[async_trait]
impl StepRecorder for DiscardSteps {
    async fn record(&self, _step: &RunStep) -> Result<(), AppError> {
        Ok(())
    }
}

pub(crate) struct EngineInput<'a> {
    pub(crate) ctx: &'a Ctx<'a>,
    pub(crate) gateway: &'a dyn AutomationGateway,
    pub(crate) actions: &'a ActionRegistry,
    pub(crate) clock: &'a dyn Clock,
    pub(crate) definition: &'a AutomationDefinition,
    pub(crate) record: &'a RecordRef,
    pub(crate) record_data: &'a Value,
    pub(crate) record_available: bool,
    pub(crate) fields: &'a FieldSet,
    pub(crate) templates: &'a TemplateScope,
    pub(crate) automation_id: Uuid,
    pub(crate) run_id: Uuid,
    pub(crate) mode: EngineMode,
    pub(crate) previous: &'a [RunStep],
}

pub(crate) fn is_transient(error: &AppError) -> bool {
    matches!(
        error,
        AppError::Persistence(_)
            | AppError::Internal(_)
            | AppError::Configuration(_)
            | AppError::RateLimited(_)
    )
}

pub(crate) fn step_error(error: &AppError) -> String {
    let message = match error {
        AppError::Internal(_) => "internal error".to_owned(),
        AppError::InvalidFields { message, fields }
        | AppError::CodedFields {
            message, fields, ..
        } => {
            let details: Vec<String> = fields
                .iter()
                .map(|field| format!("{}: {}", field.field, field.message))
                .collect();
            if details.is_empty() {
                message.clone()
            } else {
                format!("{message} ({})", details.join("; "))
            }
        }
        AppError::Coded { code, message, .. } => format!("{code}: {message}"),
        other => other.client_message().to_owned(),
    };
    truncate_error(&message)
}

struct StepDraft {
    next: Option<String>,
    condition_result: Option<bool>,
    input: Option<Value>,
    output: Option<Value>,
    status: StepStatus,
    error: Option<String>,
    failure: Option<EngineOutcome>,
    resume_at: Option<DateTime<Utc>>,
}

impl StepDraft {
    fn succeeded(next: Option<String>) -> Self {
        Self {
            next,
            condition_result: None,
            input: None,
            output: None,
            status: StepStatus::Succeeded,
            error: None,
            failure: None,
            resume_at: None,
        }
    }

    fn waiting(
        next: Option<String>,
        input: Value,
        output: Value,
        resume_at: DateTime<Utc>,
    ) -> Self {
        Self {
            input: Some(input),
            output: Some(output),
            status: StepStatus::Waiting,
            resume_at: Some(resume_at),
            ..Self::succeeded(next)
        }
    }

    fn with_io(mut self, input: Value, output: Value) -> Self {
        self.input = Some(input);
        if self.status != StepStatus::Failed {
            self.output = Some(output);
        }
        self
    }

    fn planned(mut self) -> Self {
        if self.status == StepStatus::Succeeded {
            self.status = StepStatus::Planned;
        }
        self
    }

    fn failed(input: Option<Value>, message: String, retry: bool) -> Self {
        Self {
            next: None,
            condition_result: None,
            input,
            output: None,
            status: StepStatus::Failed,
            error: Some(message.clone()),
            failure: Some(EngineOutcome::Failed {
                error: message,
                retry,
            }),
            resume_at: None,
        }
    }
}

impl EngineInput<'_> {
    pub(crate) async fn run(
        &self,
        recorder: &dyn StepRecorder,
    ) -> Result<(EngineOutcome, Vec<RunStep>), AppError> {
        let mut steps = Vec::new();
        let mut current = self.definition.start.clone();
        let mut position = 0u32;
        while let Some(step_id) = current.take() {
            let Some(step) = self.definition.step(&step_id) else {
                break;
            };
            let prior = self.previous.iter().find(|prior| prior.step_id == step_id);
            if let Some(prior) = prior.filter(|prior| prior.status == StepStatus::Succeeded) {
                current = prior.next_step_id.clone();
                steps.push(prior.clone());
                position += 1;
                continue;
            }
            let waiting = prior.filter(|prior| prior.status == StepStatus::Waiting);
            let started_at = waiting.map_or_else(|| self.clock.now(), |prior| prior.started_at);
            let timer = Instant::now();
            let draft = self.step(step, waiting).await;
            let record = RunStep {
                id: Uuid::now_v7(),
                run_id: self.run_id,
                position,
                step_id: step.id.clone(),
                kind: step.body.kind(),
                action: match &step.body {
                    StepBody::Action { action, .. } => Some(action.clone()),
                    _ => None,
                },
                input: draft.input,
                condition_result: draft.condition_result,
                next_step_id: draft.next.clone(),
                output: draft.output,
                status: draft.status,
                error: draft.error,
                started_at,
                duration_ms: u32::try_from(timer.elapsed().as_millis()).unwrap_or(u32::MAX),
            };
            if self.mode == EngineMode::Live {
                recorder.record(&record).await?;
            }
            steps.push(record);
            if let Some(failure) = draft.failure {
                return Ok((failure, steps));
            }
            if let Some(resume_at) = draft.resume_at {
                return Ok((
                    EngineOutcome::Waiting {
                        step_id: step.id.clone(),
                        resume_at,
                    },
                    steps,
                ));
            }
            current = draft.next;
            position += 1;
        }
        Ok((EngineOutcome::Completed, steps))
    }

    async fn step(&self, step: &StepSpec, waiting: Option<&RunStep>) -> StepDraft {
        match &step.body {
            StepBody::Condition { filter, next } => match self.evaluate(filter).await {
                Ok(result) => StepDraft {
                    condition_result: Some(result),
                    ..StepDraft::succeeded(if result { next.clone() } else { None })
                },
                Err(error) => StepDraft::failed(None, step_error(&error), false),
            },
            StepBody::Branch { cases, default } => {
                for case in cases {
                    match self.evaluate(&case.filter).await {
                        Ok(true) => {
                            return StepDraft {
                                condition_result: Some(true),
                                output: Some(json!({ "case": case.key })),
                                ..StepDraft::succeeded(case.next.clone())
                            };
                        }
                        Ok(false) => {}
                        Err(error) => return StepDraft::failed(None, step_error(&error), false),
                    }
                }
                StepDraft {
                    condition_result: Some(false),
                    output: Some(json!({ "case": "default" })),
                    ..StepDraft::succeeded(default.clone())
                }
            }
            StepBody::Action {
                action,
                config,
                next,
            } => self.action(step, action, config, next.clone()).await,
            StepBody::Wait {
                duration,
                until_field,
                offset,
                recheck,
                next,
            } => {
                let target = WaitTarget {
                    duration: duration.as_deref(),
                    until_field: until_field.as_deref(),
                    offset: offset.as_deref(),
                };
                self.wait(&target, recheck.as_ref(), next, waiting).await
            }
            StepBody::WaitUntil {
                filter,
                timeout,
                next,
                timeout_next,
            } => {
                self.wait_until(filter, timeout, next, timeout_next, waiting)
                    .await
            }
        }
    }

    async fn after_wait(&self, recheck: Option<&Filter>, next: &Option<String>) -> StepDraft {
        let Some(filter) = recheck else {
            return StepDraft::succeeded(next.clone());
        };
        match self.evaluate(filter).await {
            Ok(result) => StepDraft {
                condition_result: Some(result),
                ..StepDraft::succeeded(if result { next.clone() } else { None })
            },
            Err(error) => StepDraft::failed(None, step_error(&error), false),
        }
    }

    async fn wait(
        &self,
        target: &WaitTarget<'_>,
        recheck: Option<&Filter>,
        next: &Option<String>,
        waiting: Option<&RunStep>,
    ) -> StepDraft {
        let now = self.clock.now();
        let input = json!({
            "duration": target.duration,
            "until_field": target.until_field,
            "offset": target.offset,
        });
        if let Some(prior) = waiting {
            let resume_at = prior
                .output
                .as_ref()
                .and_then(|output| parse_timestamp(output.get("resume_at")));
            return self.after_wait(recheck, next).await.with_io(
                input,
                json!({
                    "resume_at": resume_at.map(timestamp),
                    "resumed_at": timestamp(now),
                }),
            );
        }
        let resume_at =
            match target.resolve(self.record_data, self.fields, self.templates.timezone, now) {
                Ok(Some(resume_at)) => resume_at,
                Ok(None) => {
                    return StepDraft {
                        condition_result: Some(false),
                        ..StepDraft::succeeded(None)
                    }
                    .with_io(input, json!({ "reason": "field_empty" }));
                }
                Err(message) => {
                    return StepDraft::failed(Some(input), truncate_error(&message), false);
                }
            };
        let output = json!({ "resume_at": timestamp(resume_at) });
        match self.mode {
            EngineMode::DryRun => self
                .after_wait(recheck, next)
                .await
                .with_io(input, output)
                .planned(),
            EngineMode::Live if resume_at > now => {
                StepDraft::waiting(next.clone(), input, output, resume_at)
            }
            EngineMode::Live => self.after_wait(recheck, next).await.with_io(
                input,
                json!({ "resume_at": timestamp(resume_at), "waited": false }),
            ),
        }
    }

    async fn wait_until(
        &self,
        filter: &Filter,
        timeout: &str,
        next: &Option<String>,
        timeout_next: &Option<String>,
        waiting: Option<&RunStep>,
    ) -> StepDraft {
        let now = self.clock.now();
        let input = json!({ "timeout": timeout });
        let Some(limit) = parse_duration(timeout) else {
            return StepDraft::failed(Some(input), format!("timeout {DURATION_MESSAGE}"), false);
        };
        let deadline = waiting
            .and_then(|prior| prior.output.as_ref())
            .and_then(|output| parse_timestamp(output.get("deadline")))
            .unwrap_or(now + limit);
        let matched = match self.evaluate(filter).await {
            Ok(matched) => matched,
            Err(error) => return StepDraft::failed(Some(input), step_error(&error), false),
        };
        let deadline_text = timestamp(deadline);
        if matched {
            let draft = StepDraft {
                condition_result: Some(true),
                ..StepDraft::succeeded(next.clone())
            }
            .with_io(input, json!({ "deadline": deadline_text, "matched": true }));
            return match self.mode {
                EngineMode::DryRun => draft.planned(),
                EngineMode::Live => draft,
            };
        }
        let timed_out = StepDraft {
            condition_result: Some(false),
            ..StepDraft::succeeded(timeout_next.clone())
        };
        match self.mode {
            EngineMode::DryRun => timed_out
                .with_io(
                    input,
                    json!({ "deadline": deadline_text, "assumed": "timeout" }),
                )
                .planned(),
            EngineMode::Live if now >= deadline => timed_out.with_io(
                input,
                json!({ "deadline": deadline_text, "timed_out": true }),
            ),
            EngineMode::Live => {
                let resume_at = (now + Duration::minutes(WAIT_UNTIL_POLL_MINUTES)).min(deadline);
                StepDraft {
                    condition_result: Some(false),
                    ..StepDraft::waiting(
                        None,
                        input,
                        json!({ "deadline": deadline_text, "resume_at": timestamp(resume_at) }),
                        resume_at,
                    )
                }
            }
        }
    }

    async fn action(
        &self,
        step: &StepSpec,
        key: &str,
        config: &Value,
        next: Option<String>,
    ) -> StepDraft {
        let Some(handler) = self.actions.get(key) else {
            return StepDraft::failed(None, format!("unknown action {key}"), false);
        };
        let record_type = self.record.record_type.as_str();
        for permission in handler.required_permissions(record_type, config) {
            if self.ctx.scope.require(&permission).is_err() {
                return StepDraft::failed(
                    None,
                    format!(
                        "permission_denied: the publisher of this version lacks {}",
                        permission.as_str()
                    ),
                    false,
                );
            }
        }
        let rendered = match self.templates.render(config) {
            Ok(rendered) => rendered,
            Err(message) => return StepDraft::failed(None, truncate_error(&message), false),
        };
        let input = Some(rendered.masked.clone());
        let context = ActionContext {
            scope: self.ctx.scope,
            gateway: self.gateway,
            record: self.record,
            record_data: self.record_data,
            fields: self.fields,
            automation_id: self.automation_id,
            run_id: self.run_id,
            step_id: &step.id,
            now: self.clock.now(),
        };
        let running = RunningAction {
            automation_id: self.automation_id,
            run_id: self.run_id,
            step_id: step.id.clone(),
            action: key.to_owned(),
            record: self.record.clone(),
            config: rendered.value.clone(),
            dry_run: self.mode == EngineMode::DryRun,
            output: None,
        };
        if let Err(error) = before_action(self.ctx, &running).await {
            return StepDraft::failed(input, step_error(&error), false);
        }
        match self.mode {
            EngineMode::DryRun => match handler.describe(&context, &rendered.value).await {
                Ok(details) => StepDraft {
                    input,
                    output: (!details.is_null()).then(|| {
                        if rendered.sensitive {
                            json!({ "details": "[redacted]" })
                        } else {
                            json!({ "details": details })
                        }
                    }),
                    status: StepStatus::Planned,
                    ..StepDraft::succeeded(next)
                },
                Err(error) => StepDraft::failed(input, step_error(&error), false),
            },
            EngineMode::Live => match self
                .filtered(handler.execute(&context, &rendered.value).await, running)
                .await
            {
                Ok(output) => StepDraft {
                    input,
                    output: Some(output),
                    ..StepDraft::succeeded(next)
                },
                Err(error) => {
                    let retry = handler.retry_safe(&rendered.value) && is_transient(&error);
                    if matches!(error, AppError::Internal(_)) {
                        tracing::warn!(step = %step.id, action = key, "automation action failed: {error}");
                    }
                    StepDraft::failed(input, step_error(&error), retry)
                }
            },
        }
    }

    async fn filtered(
        &self,
        result: Result<Value, AppError>,
        running: RunningAction,
    ) -> Result<Value, AppError> {
        let output = result?;
        after_action(
            self.ctx,
            RunningAction {
                output: Some(output),
                ..running
            },
        )
        .await
    }

    async fn evaluate(&self, filter: &Filter) -> Result<bool, AppError> {
        let evaluation = TriggerEvaluation {
            trigger: &self.templates.trigger,
            fields: self.fields,
            now: self.templates.now,
            timezone: self.templates.timezone,
            user_id: self.ctx.scope.user_id().as_str(),
        };
        match evaluation.partial(filter) {
            Partial::Constant(value) => Ok(value),
            Partial::Record(_) if !self.record_available => Ok(false),
            Partial::Record(filter) => {
                self.gateway
                    .record_matches(self.ctx.scope, self.record, &filter)
                    .await
            }
        }
    }
}
