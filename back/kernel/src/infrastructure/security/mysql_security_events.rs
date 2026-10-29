use async_trait::async_trait;
use chrono::{DateTime, NaiveDateTime, Utc};
use serde_json::Value;
use sqlx::types::Json;
use sqlx::{MySql, MySqlPool, QueryBuilder, Row};

use crate::application::organization_admin::{ListWindow, Listing};
use crate::application::security::{
    DeviceHistory, LoginFailureHistory, LoginHistory, NewSecurityEvent,
    NotifyingSecurityEventRecorder, RecentLoginFailures, SECURITY_EMAIL_JOB, SecurityEventFilter,
    SecurityEventKind, SecurityEventOwner, SecurityEventRecorder, SecurityEventRepository,
    SecurityEventUser, SecurityEventView, SecurityNotice, SecurityNoticeQueue,
};
use crate::application::shared::Job;
use crate::domain::UserId;
use crate::errors::AppError;
use crate::infrastructure::jobs::queue::insert_job;
use crate::infrastructure::repositories::mapper;

const MAX_USER_AGENT_LENGTH: usize = 255;
const MAX_IP_LENGTH: usize = 45;
const MAX_SUBJECT_LENGTH: usize = 255;
const SECURITY_EMAIL_ATTEMPTS: u32 = 5;

fn truncated(value: Option<&str>, length: usize) -> Option<String> {
    value.map(|value| value.chars().take(length).collect())
}

#[derive(Clone)]
pub(crate) struct MySqlSecurityEventStore {
    pool: MySqlPool,
}

impl MySqlSecurityEventStore {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl SecurityEventRecorder for MySqlSecurityEventStore {
    async fn record(&self, event: NewSecurityEvent) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO security_events (id, user_id, kind, subject, ip, user_agent, metadata, occurred_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(event.id.to_string())
        .bind(event.user_id.as_ref().map(UserId::as_str))
        .bind(event.kind.as_str())
        .bind(truncated(event.subject.as_deref(), MAX_SUBJECT_LENGTH))
        .bind(truncated(event.ip.as_deref(), MAX_IP_LENGTH))
        .bind(truncated(event.user_agent.as_deref(), MAX_USER_AGENT_LENGTH))
        .bind(Json(&event.metadata))
        .bind(mapper::naive(event.occurred_at))
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}

#[derive(Clone)]
pub(crate) struct MySqlSecurityNoticeQueue {
    pool: MySqlPool,
}

impl MySqlSecurityNoticeQueue {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl SecurityNoticeQueue for MySqlSecurityNoticeQueue {
    async fn enqueue(&self, notice: SecurityNotice) -> Result<(), AppError> {
        let payload = serde_json::to_value(&notice)
            .map_err(|error| AppError::Internal(format!("invalid security notice: {error}")))?;
        insert_job(
            &self.pool,
            Job::new(SECURITY_EMAIL_JOB, payload).with_max_attempts(SECURITY_EMAIL_ATTEMPTS),
        )
        .await?;
        Ok(())
    }
}

#[derive(Clone)]
pub struct MySqlSecurityEventQueries {
    pool: MySqlPool,
}

impl MySqlSecurityEventQueries {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    async fn latest(
        &self,
        kind: SecurityEventKind,
        subject: &str,
        ip: Option<&str>,
        since: DateTime<Utc>,
    ) -> Result<Option<DateTime<Utc>>, AppError> {
        let latest: Option<NaiveDateTime> = sqlx::query_scalar(
            "SELECT MAX(occurred_at) FROM security_events \
             WHERE kind = ? AND subject = ? AND ip <=> ? AND occurred_at >= ?",
        )
        .bind(kind.as_str())
        .bind(subject)
        .bind(ip)
        .bind(mapper::naive(since))
        .fetch_one(&self.pool)
        .await?;
        Ok(latest.map(mapper::utc))
    }
}

#[async_trait]
impl LoginHistory for MySqlSecurityEventQueries {
    async fn device_history(
        &self,
        user_id: &UserId,
        device_key: &str,
    ) -> Result<DeviceHistory, AppError> {
        let row = sqlx::query(
            "SELECT \
               CAST(EXISTS(SELECT 1 FROM security_events WHERE user_id = ? AND kind = ? \
                 AND JSON_EXTRACT(metadata, '$.device_key') IS NOT NULL) AS SIGNED) AS tracked, \
               CAST(EXISTS(SELECT 1 FROM security_events WHERE user_id = ? AND kind = ? \
                 AND JSON_UNQUOTE(JSON_EXTRACT(metadata, '$.device_key')) = ?) AS SIGNED) AS known",
        )
        .bind(user_id.as_str())
        .bind(SecurityEventKind::LoginSucceeded.as_str())
        .bind(user_id.as_str())
        .bind(SecurityEventKind::LoginSucceeded.as_str())
        .bind(device_key)
        .fetch_one(&self.pool)
        .await?;
        let tracked: i64 = row.try_get("tracked")?;
        let known: i64 = row.try_get("known")?;
        Ok(match (tracked > 0, known > 0) {
            (false, _) => DeviceHistory::FirstTrackedLogin,
            (true, true) => DeviceHistory::Known,
            (true, false) => DeviceHistory::New,
        })
    }
}

#[async_trait]
impl LoginFailureHistory for MySqlSecurityEventQueries {
    async fn recent_failures(
        &self,
        subject: &str,
        ip: Option<&str>,
        since: DateTime<Utc>,
        limit: u32,
    ) -> Result<RecentLoginFailures, AppError> {
        let last_success = self
            .latest(SecurityEventKind::LoginSucceeded, subject, ip, since)
            .await?;
        let start = last_success.map_or(since, |success| success.max(since));
        let rows = sqlx::query(
            "SELECT occurred_at, user_id FROM security_events \
             WHERE kind = ? AND subject = ? AND ip <=> ? AND occurred_at > ? \
             ORDER BY occurred_at DESC LIMIT ?",
        )
        .bind(SecurityEventKind::LoginFailed.as_str())
        .bind(subject)
        .bind(ip)
        .bind(mapper::naive(start))
        .bind(limit.max(1))
        .fetch_all(&self.pool)
        .await?;
        let mut failures = Vec::with_capacity(rows.len());
        let mut user_id = None;
        for row in rows {
            failures.push(mapper::utc(row.try_get("occurred_at")?));
            let row_user: Option<String> = row.try_get("user_id")?;
            if user_id.is_none() {
                user_id = row_user.and_then(|id| UserId::new(id).ok());
            }
        }
        let lock_recorded_at = self
            .latest(SecurityEventKind::LoginLocked, subject, ip, since)
            .await?;
        Ok(RecentLoginFailures {
            failures,
            user_id,
            lock_recorded_at,
        })
    }
}

fn push_filter(builder: &mut QueryBuilder<MySql>, filter: &SecurityEventFilter) {
    match &filter.owner {
        SecurityEventOwner::User(user_id) => {
            builder.push(" WHERE e.user_id = ");
            builder.push_bind(user_id.as_str().to_owned());
        }
        SecurityEventOwner::All => {
            builder.push(" WHERE 1 = 1");
        }
    }
    if !filter.kinds.is_empty() {
        builder.push(" AND e.kind IN (");
        let mut kinds = builder.separated(", ");
        for kind in &filter.kinds {
            kinds.push_bind(kind.as_str());
        }
        kinds.push_unseparated(")");
    }
    if let Some(from) = filter.from {
        builder.push(" AND e.occurred_at >= ");
        builder.push_bind(mapper::naive(from));
    }
    if let Some(to) = filter.to {
        builder.push(" AND e.occurred_at <= ");
        builder.push_bind(mapper::naive(to));
    }
}

fn public_metadata(metadata: Value) -> Value {
    match metadata {
        Value::Object(mut map) => {
            map.remove("device_key");
            Value::Object(map)
        }
        _ => Value::Object(Default::default()),
    }
}

#[async_trait]
impl SecurityEventRepository for MySqlSecurityEventQueries {
    async fn list(
        &self,
        filter: &SecurityEventFilter,
        window: ListWindow,
    ) -> Result<Listing<SecurityEventView>, AppError> {
        let mut count = QueryBuilder::<MySql>::new("SELECT COUNT(*) FROM security_events e");
        push_filter(&mut count, filter);
        let total: i64 = count.build_query_scalar().fetch_one(&self.pool).await?;

        let mut select = QueryBuilder::<MySql>::new(
            "SELECT e.id, e.kind, e.user_id, e.ip, e.user_agent, e.metadata, e.occurred_at, \
             u.display_name AS user_name, u.email AS user_email \
             FROM security_events e LEFT JOIN users u ON u.id = e.user_id",
        );
        push_filter(&mut select, filter);
        select.push(" ORDER BY e.occurred_at DESC, e.id DESC LIMIT ");
        select.push_bind(window.limit);
        select.push(" OFFSET ");
        select.push_bind(window.offset);
        let rows = select.build().fetch_all(&self.pool).await?;

        let mut items = Vec::with_capacity(rows.len());
        for row in rows {
            let kind: String = row.try_get("kind")?;
            let Some(kind) = SecurityEventKind::parse(&kind) else {
                continue;
            };
            let user_id: Option<String> = row.try_get("user_id")?;
            let user_name: Option<String> = row.try_get("user_name")?;
            let user_email: Option<String> = row.try_get("user_email")?;
            let user = match (user_id, user_email) {
                (Some(id), Some(email)) => Some(SecurityEventUser {
                    id: mapper::typed_id(id, UserId::new)?,
                    name: user_name.unwrap_or_default(),
                    email,
                }),
                _ => None,
            };
            let Json(metadata): Json<Value> = row.try_get("metadata")?;
            items.push(SecurityEventView {
                id: row.try_get("id")?,
                kind,
                user,
                ip: row.try_get("ip")?,
                user_agent: row.try_get("user_agent")?,
                metadata: public_metadata(metadata),
                occurred_at: mapper::utc(row.try_get("occurred_at")?),
            });
        }
        Ok(Listing {
            items,
            total: u64::try_from(total).unwrap_or(0),
        })
    }
}

pub(crate) type NotifyingMySqlRecorder = NotifyingSecurityEventRecorder<
    MySqlSecurityEventStore,
    MySqlSecurityEventQueries,
    MySqlSecurityNoticeQueue,
>;

#[derive(Clone)]
pub struct MySqlSecurityEventRecorder {
    inner: NotifyingMySqlRecorder,
}

impl MySqlSecurityEventRecorder {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self {
            inner: NotifyingSecurityEventRecorder::new(
                MySqlSecurityEventStore::new(pool.clone()),
                MySqlSecurityEventQueries::new(pool.clone()),
                MySqlSecurityNoticeQueue::new(pool),
            ),
        }
    }
}

#[async_trait]
impl SecurityEventRecorder for MySqlSecurityEventRecorder {
    async fn record(&self, event: NewSecurityEvent) -> Result<(), AppError> {
        self.inner.record(event).await
    }
}
