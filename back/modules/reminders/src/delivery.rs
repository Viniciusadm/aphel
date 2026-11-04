use aphel_sdk::calendar::local_day;
const MORNING_HOUR: u32 = 8;
use aphel_contract_crm::CrmRecordTypes;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{Duration, SecondsFormat, Timelike};
use serde_json::{Value, json};
use sqlx::MySqlPool;

use crate::domain::{DueReminder, DueTaskReminder};
use crate::store::ReminderStore;
use aphel_kernel::application::fields::RecordTypeRegistry;
use aphel_kernel::application::jobs::ScheduledJobDefinition;
use aphel_kernel::application::shared::{JobContext, JobHandler, RecordRef, RecordType};
use aphel_kernel::errors::AppError;
use aphel_kernel::infrastructure::jobs::MySqlJobQueue;
use aphel_sdk::notifications::LocalizedContent;
use aphel_sdk::notifications::NotificationClient;
use aphel_sdk::notifications::{NotificationIntent, dispatch};

pub(crate) const REMINDERS_DELIVER_JOB: &str = "reminders.deliver";
pub(crate) const REMINDERS_DELIVER_CRON: &str = "0 * * * * *";
pub(crate) const REMINDER_GRACE_HOURS: i64 = 24;
pub(crate) const DEFAULT_REMINDER_TITLE: &str = "Lembrete";
pub(crate) const TASKS_DUE_TODAY_JOB: &str = "reminders.tasks_due_today";
pub(crate) const TASKS_DUE_TODAY_CRON: &str = "0 */5 * * * *";

pub(crate) fn reminder_delivery_handler(pool: MySqlPool) -> Arc<dyn JobHandler> {
    let notifications = NotificationClient::new(
        pool.clone(),
        Arc::new(RecordTypeRegistry::builtin()),
        Arc::new(MySqlJobQueue::new(pool.clone())),
    );
    Arc::new(ReminderDeliveryHandler::new(
        notifications,
        ReminderStore::new(pool, Arc::new(RecordTypeRegistry::builtin())),
    ))
}

pub(crate) fn tasks_due_today_handler(pool: MySqlPool) -> Arc<dyn JobHandler> {
    Arc::new(TasksDueTodayHandler::new(NotificationClient::new(
        pool.clone(),
        Arc::new(RecordTypeRegistry::builtin()),
        Arc::new(MySqlJobQueue::new(pool)),
    )))
}

pub(crate) fn tasks_due_today_schedule() -> ScheduledJobDefinition {
    ScheduledJobDefinition::new(TASKS_DUE_TODAY_JOB, TASKS_DUE_TODAY_CRON)
        .expect("the tasks due today cron expression is valid")
        .with_max_attempts(1)
}

pub(crate) fn reminders_deliver_schedule() -> ScheduledJobDefinition {
    ScheduledJobDefinition::new(REMINDERS_DELIVER_JOB, REMINDERS_DELIVER_CRON)
        .expect("the reminders cron expression is valid")
        .with_max_attempts(1)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct ReminderDeliveryRun {
    pub(crate) tasks: usize,
    pub(crate) reminders: usize,
}

pub(crate) struct ReminderDeliveryHandler {
    notifications: NotificationClient,
    reminders: ReminderStore,
}

impl ReminderDeliveryHandler {
    pub(crate) fn new(notifications: NotificationClient, reminders: ReminderStore) -> Self {
        Self {
            notifications,
            reminders,
        }
    }

    pub(crate) async fn run(&self) -> Result<ReminderDeliveryRun, AppError> {
        let now = chrono::Utc::now();
        let since = now - Duration::hours(REMINDER_GRACE_HOURS);
        let mut run = ReminderDeliveryRun::default();
        for task in self.reminders.due_task_reminders(since, now).await? {
            dispatch(&self.notifications, task_intent(&task)).await?;
            self.reminders.mark_task_reminder_sent(&task, now).await?;
            run.tasks += 1;
        }
        for reminder in self.reminders.due_reminders(since, now).await? {
            let intent = self.reminder_intent(&reminder).await?;
            dispatch(&self.notifications, intent).await?;
            self.reminders.mark_reminder_sent(&reminder, now).await?;
            run.reminders += 1;
        }
        Ok(run)
    }

    async fn reminder_intent(
        &self,
        reminder: &DueReminder,
    ) -> Result<NotificationIntent, AppError> {
        let directory = self.notifications.directory()?;
        let mut record = None;
        let mut record_title = None;
        if let Some(target) = &reminder.record
            && directory.can_read(&reminder.user_id, target).await?
        {
            record_title = directory
                .record_summary(target)
                .await?
                .map(|summary| summary.title)
                .filter(|title| !title.trim().is_empty());
            record = Some(target.clone());
        }
        let custom_title = record_title.clone().or_else(|| reminder.note.clone());
        let localized = custom_title
            .is_none()
            .then_some(LocalizedContent::DefaultReminder);
        let title = custom_title.unwrap_or_else(|| DEFAULT_REMINDER_TITLE.to_owned());
        let body = if record_title.is_some() {
            reminder.note.clone()
        } else {
            None
        };
        let intent = NotificationIntent::new(
            reminder.user_id.clone(),
            crate::notifications::REMINDER_NOTIFICATION,
            title,
            format!(
                "reminder:{}:{}",
                reminder.id,
                reminder.remind_at.timestamp()
            ),
        )
        .with_body(body)
        .with_data(json!({
            "reminder_id": reminder.id.to_string(),
            "remind_at": reminder.remind_at.to_rfc3339_opts(SecondsFormat::Secs, true),
        }));
        let intent = match localized {
            Some(content) => intent.with_localized(content),
            None => intent,
        };
        Ok(match record {
            Some(record) => intent.with_record(Some(record)),
            None => intent.with_link(Some("/inbox".to_owned())),
        })
    }
}

fn task_intent(task: &DueTaskReminder) -> NotificationIntent {
    NotificationIntent::new(
        task.owner_id.clone(),
        tasks::notifications::TASK_DUE_NOTIFICATION,
        task.title.clone(),
        format!("task_reminder:{}:{}", task.id, task.reminder_at.timestamp()),
    )
    .with_record(Some(RecordRef::new(RecordType::ACTIVITY, task.id)))
    .with_data(json!({
        "window": "reminder",
        "minutes_before": task.minutes_before,
        "due_at": task.due_at.to_rfc3339_opts(SecondsFormat::Secs, true),
    }))
}

#[async_trait]
impl JobHandler for ReminderDeliveryHandler {
    fn kind(&self) -> &str {
        REMINDERS_DELIVER_JOB
    }

    async fn handle(&self, _context: &JobContext, _payload: Value) -> Result<(), AppError> {
        self.run().await.map(|_| ())
    }
}

pub(crate) struct TasksDueTodayHandler {
    notifications: NotificationClient,
}

impl TasksDueTodayHandler {
    pub(crate) fn new(notifications: NotificationClient) -> Self {
        Self { notifications }
    }

    pub(crate) async fn run(&self) -> Result<usize, AppError> {
        let now = chrono::Utc::now();
        let directory = self.notifications.directory()?;
        let timezone = directory.organization_timezone().await?;
        if now.with_timezone(&timezone).hour() != MORNING_HOUR {
            return Ok(0);
        }
        let (date, _, end) = local_day(timezone, now);
        let mut sent = 0;
        for task in directory.open_tasks_due(None, now, end).await? {
            let intent = NotificationIntent::new(
                task.owner_id.clone(),
                tasks::notifications::TASK_DUE_NOTIFICATION,
                task.title.clone(),
                format!("task_due_today:{}:{date}", task.id),
            )
            .with_record(Some(RecordRef::new(RecordType::ACTIVITY, task.id)))
            .with_data(json!({
                "window": "today",
                "due_at": task.due_at.to_rfc3339_opts(SecondsFormat::Secs, true),
            }));
            dispatch(&self.notifications, intent).await?;
            sent += 1;
        }
        Ok(sent)
    }
}

#[async_trait]
impl JobHandler for TasksDueTodayHandler {
    fn kind(&self) -> &str {
        TASKS_DUE_TODAY_JOB
    }

    async fn handle(&self, _context: &JobContext, _payload: Value) -> Result<(), AppError> {
        self.run().await.map(|_| ())
    }
}
