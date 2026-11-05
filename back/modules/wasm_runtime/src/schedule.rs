use std::sync::Arc;

use aphel_sdk::platform::application::shared::ExecutionContext;
use async_trait::async_trait;
use chrono::{DateTime, Duration, SecondsFormat, Utc};
use serde_json::{Value, json};
use sqlx::{MySqlPool, Row, types::Json};
use uuid::Uuid;

use crate::registry::{self, LoadedPlugin};
use crate::runtime;
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::application::jobs::{CronSchedule, ScheduledJobDefinition};
use aphel_sdk::platform::application::shared::{Job, JobContext, JobHandler, JobQueue};
use aphel_sdk::platform::infrastructure::jobs::MySqlJobQueue;

pub(crate) const TICK_JOB: &str = "wasm_runtime.tick";
pub(crate) const RUN_JOB: &str = "plugin.schedule";
const TICK_CRON: &str = "0 * * * * *";
const LOOKBACK_MINUTES: i64 = 5;
const MAX_SLOTS_PER_TICK: usize = 60;
const AHEAD_SECONDS: i64 = 60;
const MAX_AHEAD_SLOTS: usize = 12;
const RUN_MAX_ATTEMPTS: u32 = 3;

pub(crate) fn tick_schedule() -> Result<ScheduledJobDefinition, AppError> {
    Ok(ScheduledJobDefinition::new(TICK_JOB, TICK_CRON)?.with_max_attempts(1))
}

fn slot_text(slot: DateTime<Utc>) -> String {
    slot.to_rfc3339_opts(SecondsFormat::Secs, true)
}

fn due_slots(cron: &CronSchedule, now: DateTime<Utc>) -> Vec<DateTime<Utc>> {
    let mut slots = Vec::new();
    let mut cursor = now - Duration::minutes(LOOKBACK_MINUTES);
    while let Some(next) = cron.next_after(cursor) {
        if next > now || slots.len() >= MAX_SLOTS_PER_TICK {
            break;
        }
        slots.push(next);
        cursor = next;
    }
    slots
}

fn upcoming_slots(cron: &CronSchedule, now: DateTime<Utc>) -> Vec<DateTime<Utc>> {
    let mut slots = Vec::new();
    let horizon = now + Duration::seconds(AHEAD_SECONDS);
    let mut cursor = now;
    while let Some(next) = cron.next_after(cursor) {
        if next > horizon || slots.len() >= MAX_AHEAD_SLOTS {
            break;
        }
        slots.push(next);
        cursor = next;
    }
    slots
}

fn run_job(plugin: &LoadedPlugin, name: &str, slot: DateTime<Utc>) -> Job {
    let slot = slot_text(slot);
    Job::new(
        RUN_JOB,
        json!({ "plugin": plugin.key, "name": name, "slot": slot }),
    )
    .with_max_attempts(RUN_MAX_ATTEMPTS)
    .with_unique_key(format!("{RUN_JOB}:{}:{name}:{slot}", plugin.key))
}

pub(crate) struct Tick {
    pool: MySqlPool,
    queue: MySqlJobQueue,
}

impl Tick {
    pub(crate) fn shared(pool: MySqlPool) -> Arc<dyn JobHandler> {
        Arc::new(Self {
            queue: MySqlJobQueue::new(pool.clone()),
            pool,
        })
    }
}

#[async_trait]
impl JobHandler for Tick {
    fn kind(&self) -> &str {
        TICK_JOB
    }

    async fn handle(&self, _context: &JobContext, _payload: Value) -> Result<(), AppError> {
        if let Err(error) = crate::changes::prune(&self.pool).await {
            tracing::warn!(error = %error, "could not prune the plugin change feed");
        }
        let snapshot = registry::current(&self.pool).await?;
        let now = Utc::now();
        for plugin in &snapshot.plugins {
            if plugin.backend.is_none() {
                continue;
            }
            for schedule in &plugin.manifest.schedules {
                let cron = match CronSchedule::parse(&schedule.cron) {
                    Ok(cron) => cron,
                    Err(error) => {
                        tracing::warn!(plugin = %plugin.key, schedule = %schedule.name, error = %error, "invalid plugin schedule");
                        continue;
                    }
                };
                for slot in due_slots(&cron, now) {
                    self.queue
                        .enqueue_now(run_job(plugin, &schedule.name, slot))
                        .await?;
                }
                for slot in upcoming_slots(&cron, now) {
                    self.queue
                        .enqueue_now(run_job(plugin, &schedule.name, slot).run_at(slot))
                        .await?;
                }
            }
        }
        Ok(())
    }
}

pub(crate) struct Run {
    pool: MySqlPool,
}

impl Run {
    pub(crate) fn shared(pool: MySqlPool) -> Arc<dyn JobHandler> {
        Arc::new(Self { pool })
    }
}

#[async_trait]
impl JobHandler for Run {
    fn kind(&self) -> &str {
        RUN_JOB
    }

    async fn handle(&self, _context: &JobContext, payload: Value) -> Result<(), AppError> {
        let text = |key: &str| {
            payload
                .get(key)
                .and_then(Value::as_str)
                .map(str::to_owned)
                .ok_or_else(|| AppError::Validation(format!("{RUN_JOB} needs {key}")))
        };
        let (key, name, slot) = (text("plugin")?, text("name")?, text("slot")?);
        let snapshot = registry::current(&self.pool).await?;
        let Some(plugin) = snapshot.plugin(&key) else {
            return Ok(());
        };
        if !plugin
            .manifest
            .schedules
            .iter()
            .any(|schedule| schedule.name == name)
        {
            return Ok(());
        }
        let source = format!("schedule:{name}:{slot}");
        let execution =
            ExecutionContext::plugin(&key, "schedule", None, None, None, Uuid::now_v7())?;
        let mut tx =
            aphel_sdk::platform::database::transaction::MySqlTx::from(self.pool.begin().await?);
        sqlx::query("INSERT IGNORE INTO plugins_executions (id, plugin_key, source_key, input, execution) VALUES (?, ?, ?, ?, ?)")
            .bind(execution.id.to_string()).bind(&key).bind(&source).bind(Json(json!({"name": name, "slot": slot})))
            .bind(Json(&execution)).execute(&mut *tx).await?;
        tx.commit().await?;
        let mut tx =
            aphel_sdk::platform::database::transaction::MySqlTx::from(self.pool.begin().await?);
        let row = sqlx::query("SELECT id, execution, status FROM plugins_executions WHERE plugin_key = ? AND source_key = ? FOR UPDATE")
            .bind(&key).bind(&source).fetch_one(&mut *tx).await?;
        if row.try_get::<String, _>("status")? == "completed" {
            return Ok(());
        }
        let id: String = row.try_get("id")?;
        let Json(execution): Json<ExecutionContext> = row.try_get("execution")?;
        match runtime::on_schedule(plugin, &name, &slot, execution).await {
            Ok(()) => {
                sqlx::query("UPDATE plugins_executions SET status = 'completed', completed_at = UTC_TIMESTAMP(6), last_error = NULL WHERE id = ?")
                .bind(id).execute(&mut *tx).await?;
            }
            Err(error) => {
                sqlx::query("UPDATE plugins_executions SET last_error = ? WHERE id = ?")
                    .bind(error.client_message())
                    .bind(id)
                    .execute(&mut *tx)
                    .await?;
                tx.commit().await?;
                return Err(error);
            }
        }
        tx.commit().await?;
        Ok(())
    }
}
