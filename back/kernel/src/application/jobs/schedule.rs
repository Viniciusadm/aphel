use std::str::FromStr;

use chrono::{DateTime, Utc};
use serde_json::Value;

use crate::application::shared::{DEFAULT_MAX_ATTEMPTS, DEFAULT_QUEUE};
use crate::errors::AppError;

#[derive(Debug, Clone)]
pub struct CronSchedule {
    expression: String,
    schedule: cron::Schedule,
}

impl CronSchedule {
    pub fn parse(expression: &str) -> Result<Self, AppError> {
        let expression = expression.trim();
        let schedule = cron::Schedule::from_str(expression).map_err(|error| {
            AppError::Validation(format!("invalid cron expression {expression:?}: {error}"))
        })?;
        Ok(Self {
            expression: expression.to_owned(),
            schedule,
        })
    }

    pub fn expression(&self) -> &str {
        &self.expression
    }

    pub fn next_after(&self, after: DateTime<Utc>) -> Option<DateTime<Utc>> {
        self.schedule.after(&after).next()
    }
}

impl PartialEq for CronSchedule {
    fn eq(&self, other: &Self) -> bool {
        self.expression == other.expression
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ScheduledJobDefinition {
    pub kind: String,
    pub cron: CronSchedule,
    pub payload: Value,
    pub queue: String,
    pub max_attempts: u32,
}

impl ScheduledJobDefinition {
    pub fn new(kind: impl Into<String>, cron: &str) -> Result<Self, AppError> {
        Ok(Self {
            kind: kind.into(),
            cron: CronSchedule::parse(cron)?,
            payload: Value::Object(Default::default()),
            queue: DEFAULT_QUEUE.to_owned(),
            max_attempts: DEFAULT_MAX_ATTEMPTS,
        })
    }

    pub fn with_payload(mut self, payload: Value) -> Self {
        self.payload = payload;
        self
    }

    pub fn on_queue(mut self, queue: impl Into<String>) -> Self {
        self.queue = queue.into();
        self
    }

    pub fn with_max_attempts(mut self, max_attempts: u32) -> Self {
        self.max_attempts = max_attempts.max(1);
        self
    }

    pub fn unique_key_for(kind: &str, run_at: DateTime<Utc>) -> String {
        format!(
            "scheduled:{kind}:{}",
            run_at.to_rfc3339_opts(chrono::SecondsFormat::Micros, true)
        )
    }
}
