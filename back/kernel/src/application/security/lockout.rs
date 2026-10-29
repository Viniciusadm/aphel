use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use serde_json::json;

use crate::application::auth::ClientMetadata;
use crate::application::security::{
    LoginAttempt, LoginFailure, LoginMonitor, NewSecurityEvent, RecordingLoginMonitor,
    SecurityEventKind, SecurityEventRecorder, record_quietly,
};
use crate::domain::users::User;
use crate::domain::{SessionId, UserId};
use crate::errors::AppError;

pub const LOGIN_LOCKED: &str = "login_locked";
pub const DEFAULT_LOCKOUT_ATTEMPTS: u32 = 5;
pub const DEFAULT_LOCKOUT_WINDOW_SECONDS: i64 = 15 * 60;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoginLockoutPolicy {
    pub max_failures: u32,
    pub window: Duration,
}

impl Default for LoginLockoutPolicy {
    fn default() -> Self {
        Self {
            max_failures: DEFAULT_LOCKOUT_ATTEMPTS,
            window: Duration::seconds(DEFAULT_LOCKOUT_WINDOW_SECONDS),
        }
    }
}

impl LoginLockoutPolicy {
    pub fn is_enabled(&self) -> bool {
        self.max_failures > 0 && self.window > Duration::zero()
    }

    pub fn history_start(&self, now: DateTime<Utc>) -> DateTime<Utc> {
        now - self.window - self.window
    }

    pub fn locked_until(
        &self,
        failures: &[DateTime<Utc>],
        now: DateTime<Utc>,
    ) -> Option<DateTime<Utc>> {
        if !self.is_enabled() {
            return None;
        }
        let needed = self.max_failures as usize;
        let newest = *failures.first()?;
        let oldest = *failures.get(needed - 1)?;
        if newest - oldest > self.window {
            return None;
        }
        let until = newest + self.window;
        (now < until).then_some(until)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RecentLoginFailures {
    pub failures: Vec<DateTime<Utc>>,
    pub user_id: Option<UserId>,
    pub lock_recorded_at: Option<DateTime<Utc>>,
}

#[async_trait]
pub trait LoginFailureHistory: Send + Sync {
    async fn recent_failures(
        &self,
        subject: &str,
        ip: Option<&str>,
        since: DateTime<Utc>,
        limit: u32,
    ) -> Result<RecentLoginFailures, AppError>;
}

pub fn login_locked(until: DateTime<Utc>, now: DateTime<Utc>) -> AppError {
    let seconds = (until - now).num_seconds().max(1);
    let minutes = (seconds + 59) / 60;
    AppError::CodedDetails {
        kind: crate::errors::CodedKind::TooManyRequests,
        code: LOGIN_LOCKED,
        message: format!("too many failed sign-in attempts; try again in {minutes} minute(s)"),
        details: serde_json::json!({ "retry_after": seconds }),
    }
}

pub struct LockoutLoginMonitor<R, H> {
    recording: RecordingLoginMonitor<R>,
    recorder: R,
    history: H,
    policy: LoginLockoutPolicy,
}

impl<R, H> LockoutLoginMonitor<R, H>
where
    R: Clone,
{
    pub fn new(recorder: R, history: H, policy: LoginLockoutPolicy) -> Self {
        Self {
            recording: RecordingLoginMonitor::new(recorder.clone()),
            recorder,
            history,
            policy,
        }
    }
}

#[async_trait]
impl<R, H> LoginMonitor for LockoutLoginMonitor<R, H>
where
    R: SecurityEventRecorder + Clone,
    H: LoginFailureHistory,
{
    async fn ensure_allowed(&self, attempt: &LoginAttempt) -> Result<(), AppError> {
        if !self.policy.is_enabled() {
            return Ok(());
        }
        let now = Utc::now();
        let ip = attempt.client.ip.as_deref();
        let recent = self
            .history
            .recent_failures(
                &attempt.email,
                ip,
                self.policy.history_start(now),
                self.policy.max_failures,
            )
            .await?;
        let Some(until) = self.policy.locked_until(&recent.failures, now) else {
            return Ok(());
        };
        let newest = recent.failures.first().copied().unwrap_or(now);
        if recent.lock_recorded_at.is_none_or(|at| at < newest) {
            let mut event = NewSecurityEvent::new(SecurityEventKind::LoginLocked, now)
                .subject(&attempt.email)
                .client(ip, attempt.client.user_agent.as_deref())
                .metadata(json!({
                    "failures": recent.failures.len(),
                    "locked_until": until.to_rfc3339(),
                }));
            if let Some(user_id) = &recent.user_id {
                event = event.user(user_id);
            }
            record_quietly(&self.recorder, event).await;
        }
        Err(login_locked(until, now))
    }

    async fn failed(
        &self,
        attempt: &LoginAttempt,
        user: Option<&User>,
        reason: LoginFailure,
    ) -> Result<(), AppError> {
        self.recording.failed(attempt, user, reason).await
    }

    async fn succeeded(
        &self,
        user: &User,
        session_id: &SessionId,
        client: &ClientMetadata,
    ) -> Result<(), AppError> {
        self.recording.succeeded(user, session_id, client).await
    }

    async fn password_changed(&self, user: &User, client: &ClientMetadata) -> Result<(), AppError> {
        self.recording.password_changed(user, client).await
    }
}
