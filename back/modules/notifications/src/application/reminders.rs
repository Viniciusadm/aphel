use async_trait::async_trait;
use chrono::{DateTime, Duration, NaiveDate, TimeZone, Timelike, Utc};
use chrono_tz::Tz;
use serde_json::{Value, json};

use crate::application::texts::LocalizedContent;
use crate::application::{
    DispatchOutcome, NotificationDirectory, NotificationIntent, NotificationPorts,
    NotificationPreferenceRepository, dispatch,
};
use crate::domain::{NotificationChannel, NotificationKind};
use aphel_kernel::application::jobs::ScheduledJobDefinition;
use aphel_kernel::application::mail::MailLocale;
use aphel_kernel::application::security::Clock;
use aphel_kernel::application::shared::{JobContext, JobHandler};
use aphel_kernel::errors::AppError;

pub const DAILY_DIGEST_JOB: &str = "notification.daily_digest";
pub const DAILY_DIGEST_CRON: &str = "0 0 * * * *";
pub const MORNING_HOUR: u32 = 8;
pub const MAX_DIGEST_LINES: usize = 20;

pub fn daily_digest_schedule() -> ScheduledJobDefinition {
    ScheduledJobDefinition::new(DAILY_DIGEST_JOB, DAILY_DIGEST_CRON)
        .expect("the daily digest cron expression is valid")
        .with_max_attempts(1)
}

pub fn local_midnight(timezone: Tz, date: NaiveDate) -> DateTime<Utc> {
    (0..4)
        .filter_map(|hour| date.and_hms_opt(hour, 0, 0))
        .find_map(|naive| timezone.from_local_datetime(&naive).earliest())
        .map(|local| local.with_timezone(&Utc))
        .unwrap_or_else(|| Utc.from_utc_datetime(&date.and_hms_opt(0, 0, 0).unwrap_or_default()))
}

pub fn local_day(timezone: Tz, now: DateTime<Utc>) -> (NaiveDate, DateTime<Utc>, DateTime<Utc>) {
    let date = now.with_timezone(&timezone).date_naive();
    let start = local_midnight(timezone, date);
    let end = date
        .succ_opt()
        .map(|next| local_midnight(timezone, next))
        .unwrap_or(start + Duration::days(1));
    (date, start, end)
}

fn is_morning_or_later(timezone: Tz, now: DateTime<Utc>) -> bool {
    now.with_timezone(&timezone).hour() >= MORNING_HOUR
}

pub struct DailyDigestHandler<P> {
    ports: P,
}

impl<P> DailyDigestHandler<P>
where
    P: NotificationPorts,
{
    pub fn new(ports: P) -> Self {
        Self { ports }
    }

    pub async fn run(&self) -> Result<usize, AppError> {
        let now = self.ports.clock().now();
        let mut sent = 0;
        let subscribers = self
            .ports
            .preferences()
            .subscribers(NotificationKind::DAILY_DIGEST, NotificationChannel::Email)
            .await?;
        for user_id in subscribers {
            let timezone = self.ports.directory().user_timezone(&user_id).await?;
            if !is_morning_or_later(timezone, now) {
                continue;
            }
            let (date, start, end) = local_day(timezone, now);
            let tasks = self
                .ports
                .directory()
                .open_tasks_due(Some(&user_id), start, end)
                .await?;
            if tasks.is_empty() {
                continue;
            }
            let lines: Vec<String> = tasks
                .iter()
                .take(MAX_DIGEST_LINES)
                .map(|task| {
                    format!(
                        "{} — {}",
                        task.due_at.with_timezone(&timezone).format("%H:%M"),
                        task.title
                    )
                })
                .collect();
            let content = LocalizedContent::DailyDigest {
                count: tasks.len(),
                lines,
                more: tasks.len().saturating_sub(MAX_DIGEST_LINES),
            };
            let (title, body) = content.render(MailLocale::default());
            let intent = NotificationIntent::new(
                user_id,
                NotificationKind::DAILY_DIGEST,
                title,
                format!("daily_digest:{date}"),
            )
            .with_body(body)
            .with_localized(content)
            .with_link(Some("/tasks".to_owned()))
            .with_data(json!({ "date": date.to_string(), "count": tasks.len() }));
            if dispatch(&self.ports, intent).await? != DispatchOutcome::Duplicate {
                sent += 1;
            }
        }
        Ok(sent)
    }
}

#[async_trait]
impl<P> JobHandler for DailyDigestHandler<P>
where
    P: NotificationPorts,
{
    fn kind(&self) -> &str {
        DAILY_DIGEST_JOB
    }

    async fn handle(&self, _context: &JobContext, _payload: Value) -> Result<(), AppError> {
        self.run().await.map(|_| ())
    }
}
