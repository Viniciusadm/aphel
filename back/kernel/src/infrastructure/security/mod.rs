pub(crate) mod mysql_account_sessions;
pub(crate) mod mysql_security_events;

use std::env;
use std::sync::{Arc, OnceLock};

use chrono::Duration;
use sqlx::MySqlPool;

use crate::application::auth::AuthSettings;
use crate::application::mail::SharedMailer;
use crate::application::security::{
    AccountSecurityPorts, AccountSecurityUseCases, DEFAULT_LOCKOUT_ATTEMPTS,
    DEFAULT_LOCKOUT_WINDOW_SECONDS, LockoutLoginMonitor, LoginLockoutPolicy, SecurityEmailHandler,
};
use crate::application::shared::JobHandler;
use crate::infrastructure::auth::{Argon2PasswordHasher, SystemClock};
use crate::infrastructure::repositories::mysql_users::MySqlUserRepository;
pub(crate) use mysql_account_sessions::MySqlAccountSessionRepository;
pub(crate) use mysql_security_events::{MySqlSecurityEventQueries, MySqlSecurityEventRecorder};

pub(crate) const LOGIN_LOCKOUT_ATTEMPTS_ENV: &str = "LOGIN_LOCKOUT_ATTEMPTS";
pub(crate) const LOGIN_LOCKOUT_WINDOW_SECS_ENV: &str = "LOGIN_LOCKOUT_WINDOW_SECS";

pub(crate) type MySqlLoginMonitor =
    LockoutLoginMonitor<MySqlSecurityEventRecorder, MySqlSecurityEventQueries>;

pub(crate) fn login_lockout_policy() -> LoginLockoutPolicy {
    static POLICY: OnceLock<LoginLockoutPolicy> = OnceLock::new();
    *POLICY.get_or_init(|| {
        let max_failures = env::var(LOGIN_LOCKOUT_ATTEMPTS_ENV)
            .ok()
            .and_then(|value| value.trim().parse::<u32>().ok())
            .unwrap_or(DEFAULT_LOCKOUT_ATTEMPTS);
        let window = env::var(LOGIN_LOCKOUT_WINDOW_SECS_ENV)
            .ok()
            .and_then(|value| value.trim().parse::<i64>().ok())
            .filter(|seconds| *seconds >= 0)
            .unwrap_or(DEFAULT_LOCKOUT_WINDOW_SECONDS);
        LoginLockoutPolicy {
            max_failures,
            window: Duration::seconds(window),
        }
    })
}

pub(crate) fn mysql_login_monitor(
    pool: MySqlPool,
    _mailer: SharedMailer,
    _settings: &AuthSettings,
) -> MySqlLoginMonitor {
    LockoutLoginMonitor::new(
        MySqlSecurityEventRecorder::new(pool.clone()),
        MySqlSecurityEventQueries::new(pool),
        login_lockout_policy(),
    )
}

pub(crate) struct MySqlAccountSecurityPorts {
    sessions: MySqlAccountSessionRepository,
    users: MySqlUserRepository,
    events: MySqlSecurityEventQueries,
    recorder: MySqlSecurityEventRecorder,
}

impl MySqlAccountSecurityPorts {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self {
            sessions: MySqlAccountSessionRepository::new(pool.clone()),
            users: MySqlUserRepository::new(pool.clone()),
            events: MySqlSecurityEventQueries::new(pool.clone()),
            recorder: MySqlSecurityEventRecorder::new(pool),
        }
    }
}

impl AccountSecurityPorts for MySqlAccountSecurityPorts {
    type Sessions = MySqlAccountSessionRepository;
    type Users = MySqlUserRepository;
    type Hasher = Argon2PasswordHasher;
    type Events = MySqlSecurityEventQueries;
    type Recorder = MySqlSecurityEventRecorder;
    type Clock = SystemClock;

    fn sessions(&self) -> &Self::Sessions {
        &self.sessions
    }

    fn users(&self) -> &Self::Users {
        &self.users
    }

    fn hasher(&self) -> &Self::Hasher {
        &Argon2PasswordHasher
    }

    fn events(&self) -> &Self::Events {
        &self.events
    }

    fn recorder(&self) -> &Self::Recorder {
        &self.recorder
    }

    fn clock(&self) -> &Self::Clock {
        &SystemClock
    }
}

pub(crate) fn account_security_use_cases(
    pool: MySqlPool,
) -> AccountSecurityUseCases<MySqlAccountSecurityPorts> {
    AccountSecurityUseCases::new(MySqlAccountSecurityPorts::new(pool))
}

pub(crate) fn security_email_handler(
    pool: MySqlPool,
    mailer: SharedMailer,
    app_base_url: impl Into<crate::application::organizations::PublicBaseUrl>,
) -> Arc<dyn JobHandler> {
    Arc::new(SecurityEmailHandler::new(
        MySqlUserRepository::new(pool),
        mailer,
        app_base_url,
    ))
}
