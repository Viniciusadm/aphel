use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::application::fields::formulas::{
    FormulaEngine, FormulaPorts, FormulaSource, MAX_PROPAGATION_DEPTH, TypeFormulas, local_today,
};
use crate::application::jobs::ScheduledJobDefinition;
use crate::application::shared::{
    DomainEvent, EventConsumer, Job, JobContext, JobHandler, JobQueue, RecordType,
};
use crate::domain::access::AccessScope;
use crate::domain::fields::FORMULA_FIELD_TYPE;
use crate::errors::AppError;

pub const FORMULAS_CONSUMER: &str = "formulas.recalculate";
pub const FORMULAS_RECALCULATE_JOB: &str = "formulas.recalculate";
pub const FORMULAS_DAILY_JOB: &str = "formulas.daily";
pub const FORMULAS_DAILY_CRON: &str = "0 5 * * * *";

const FIELD_EVENTS: [&str; 3] = ["field.created", "field.updated", "field.activated"];
const LINK_EVENTS: [&str; 2] = ["record.linked", "record.unlinked"];

pub fn formulas_daily_schedule() -> ScheduledJobDefinition {
    ScheduledJobDefinition::new(FORMULAS_DAILY_JOB, FORMULAS_DAILY_CRON)
        .expect("the formulas daily cron expression is valid")
}

fn is_record_event(event_type: &str) -> bool {
    let Some((record_type, _)) = event_type.rsplit_once('.') else {
        return false;
    };
    if crate::domain::custom_objects::is_custom_record_type(record_type) {
        return true;
    }
    let Ok(record_type) = RecordType::parse(record_type) else {
        return false;
    };
    crate::modules::try_app().is_some_and(|app| app.record_types().get(&record_type).is_some())
}

pub fn is_formula_trigger(event_type: &str) -> bool {
    FIELD_EVENTS.contains(&event_type)
        || LINK_EVENTS.contains(&event_type)
        || is_record_event(event_type)
}

pub fn recalculate_job(record_type: &RecordType, strict: bool, depth: usize) -> Job {
    Job::new(
        FORMULAS_RECALCULATE_JOB,
        json!({ "record_type": record_type.as_str(), "strict": strict, "depth": depth }),
    )
}

fn previous_parents(payload: &Value) -> Vec<(String, Uuid)> {
    payload
        .get("changes")
        .and_then(Value::as_array)
        .map(|changes| {
            changes
                .iter()
                .filter_map(|change| {
                    let field = change.get("field")?.as_str()?;
                    let old = Uuid::parse_str(change.get("old")?.as_str()?).ok()?;
                    Some((field.to_owned(), old))
                })
                .collect()
        })
        .unwrap_or_default()
}

fn link_target(payload: &Value) -> Option<(RecordType, Uuid)> {
    let target = payload.get("target")?;
    let record_type = RecordType::parse(target.get("record_type")?.as_str()?).ok()?;
    let record_id = Uuid::parse_str(target.get("record_id")?.as_str()?).ok()?;
    Some((record_type, record_id))
}

pub struct FormulaEventConsumer<P> {
    ports: P,
}

impl<P> FormulaEventConsumer<P> {
    pub fn new(ports: P) -> Self {
        Self { ports }
    }
}

#[async_trait]
impl<P> EventConsumer for FormulaEventConsumer<P>
where
    P: FormulaPorts,
{
    fn name(&self) -> &str {
        FORMULAS_CONSUMER
    }

    fn accepts(&self, event_type: &str) -> bool {
        is_formula_trigger(event_type)
    }

    async fn consume(&self, event: &DomainEvent) -> Result<(), AppError> {
        let scope = AccessScope::system();
        if FIELD_EVENTS.contains(&event.event_type.as_str()) {
            let is_formula =
                event.payload.get("field_type").and_then(Value::as_str) == Some(FORMULA_FIELD_TYPE);
            let record_type = event
                .payload
                .get("record_type")
                .and_then(Value::as_str)
                .and_then(|record_type| RecordType::parse(record_type).ok());
            if let (true, Some(record_type)) = (is_formula, record_type) {
                self.ports
                    .jobs()
                    .enqueue_now(recalculate_job(&record_type, true, 0))
                    .await?;
            }
            return Ok(());
        }
        let Some(record) = &event.record else {
            return Ok(());
        };
        if self.ports.registry().get(&record.record_type).is_none() {
            return Ok(());
        }
        let engine = FormulaEngine::new(&self.ports, &scope);
        engine
            .recalculate_with_dependents(
                &record.record_type,
                &[record.record_id],
                &previous_parents(&event.payload),
            )
            .await?;
        if LINK_EVENTS.contains(&event.event_type.as_str())
            && let Some((target_type, target_id)) = link_target(&event.payload)
            && self.ports.registry().get(&target_type).is_some()
        {
            engine
                .recalculate_with_dependents(&target_type, &[target_id], &[])
                .await?;
        }
        if event.event_type.ends_with(".deleted") {
            for dependent in engine.linked_dependents(&record.record_type).await? {
                self.ports
                    .jobs()
                    .enqueue_now(recalculate_job(&dependent, false, 0))
                    .await?;
            }
        }
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
struct RecalculatePayload {
    record_type: String,
    #[serde(default)]
    strict: bool,
    #[serde(default)]
    depth: usize,
}

pub struct FormulaRecalculateHandler<P> {
    ports: P,
}

impl<P> FormulaRecalculateHandler<P> {
    pub fn new(ports: P) -> Self {
        Self { ports }
    }
}

#[async_trait]
impl<P> JobHandler for FormulaRecalculateHandler<P>
where
    P: FormulaPorts,
{
    fn kind(&self) -> &str {
        FORMULAS_RECALCULATE_JOB
    }

    async fn handle(&self, _context: &JobContext, payload: Value) -> Result<(), AppError> {
        let payload: RecalculatePayload = serde_json::from_value(payload)
            .map_err(|error| AppError::Internal(format!("invalid formula job payload: {error}")))?;
        let record_type = RecordType::parse(payload.record_type)?;
        if self.ports.registry().get(&record_type).is_none() {
            return Ok(());
        }
        let scope = AccessScope::system();
        let engine = FormulaEngine::new(&self.ports, &scope);
        let mut workspace = engine.workspace();
        let formulas = TypeFormulas::new(workspace.context(&record_type).await?);
        engine.recalculate_all(&formulas, payload.strict).await?;
        if payload.depth >= MAX_PROPAGATION_DEPTH {
            return Ok(());
        }
        for parent_type in self.ports.source().formula_record_types().await? {
            if parent_type == record_type || self.ports.registry().get(&parent_type).is_none() {
                continue;
            }
            let parent = TypeFormulas::new(workspace.context(&parent_type).await?);
            if parent.aggregating_over(&record_type).is_empty() {
                continue;
            }
            self.ports
                .jobs()
                .enqueue_now(recalculate_job(&parent_type, false, payload.depth + 1))
                .await?;
        }
        Ok(())
    }
}

pub struct FormulaDailyHandler<P> {
    ports: P,
}

impl<P> FormulaDailyHandler<P> {
    pub fn new(ports: P) -> Self {
        Self { ports }
    }
}

#[async_trait]
impl<P> JobHandler for FormulaDailyHandler<P>
where
    P: FormulaPorts,
{
    fn kind(&self) -> &str {
        FORMULAS_DAILY_JOB
    }

    async fn handle(&self, _context: &JobContext, _payload: Value) -> Result<(), AppError> {
        let source = self.ports.source();
        let record_types = source.formula_record_types().await?;
        if record_types.is_empty() {
            return Ok(());
        }
        let timezone = source.timezone().await?;
        if !source.mark_daily_run(local_today(timezone)).await? {
            return Ok(());
        }
        for record_type in record_types {
            self.ports
                .jobs()
                .enqueue_now(recalculate_job(&record_type, false, MAX_PROPAGATION_DEPTH))
                .await?;
        }
        Ok(())
    }
}
