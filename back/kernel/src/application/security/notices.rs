use async_trait::async_trait;
use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::application::auth::UserRepository;
use crate::application::mail::{
    EmailMessage, MailLocale, Mailer, is_mail_not_configured, templates,
};
use crate::application::security::{
    NewSecurityEvent, SecurityEventKind, SecurityEventRecorder, describe_device, device_key,
};
use crate::application::shared::{JobContext, JobHandler};
use crate::domain::UserId;
use crate::errors::AppError;

pub const SECURITY_EMAIL_JOB: &str = "security.email";
pub const DEVICE_KEY_METADATA: &str = "device_key";
const DEFAULT_NOTICE_TIMEZONE: Tz = chrono_tz::America::Sao_Paulo;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SecurityNoticeKind {
    NewDevice,
    PasswordChanged,
    TwoFactorEnabled,
    TwoFactorDisabled,
}

impl SecurityNoticeKind {
    pub fn for_event(kind: SecurityEventKind) -> Option<Self> {
        match kind {
            SecurityEventKind::PasswordChanged => Some(Self::PasswordChanged),
            SecurityEventKind::TwoFactorEnabled => Some(Self::TwoFactorEnabled),
            SecurityEventKind::TwoFactorDisabled => Some(Self::TwoFactorDisabled),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecurityNotice {
    pub kind: SecurityNoticeKind,
    pub user_id: String,
    pub ip: Option<String>,
    pub user_agent: Option<String>,
    pub occurred_at: DateTime<Utc>,
}

impl SecurityNotice {
    fn from_event(kind: SecurityNoticeKind, user_id: &UserId, event: &NewSecurityEvent) -> Self {
        Self {
            kind,
            user_id: user_id.as_str().to_owned(),
            ip: event.ip.clone(),
            user_agent: event.user_agent.clone(),
            occurred_at: event.occurred_at,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceHistory {
    FirstTrackedLogin,
    Known,
    New,
}

#[async_trait]
pub trait LoginHistory: Send + Sync {
    async fn device_history(
        &self,
        user_id: &UserId,
        device_key: &str,
    ) -> Result<DeviceHistory, AppError>;
}

#[async_trait]
pub trait SecurityNoticeQueue: Send + Sync {
    async fn enqueue(&self, notice: SecurityNotice) -> Result<(), AppError>;
}

#[derive(Clone)]
pub struct NotifyingSecurityEventRecorder<R, H, Q> {
    inner: R,
    history: H,
    queue: Q,
}

impl<R, H, Q> NotifyingSecurityEventRecorder<R, H, Q> {
    pub fn new(inner: R, history: H, queue: Q) -> Self {
        Self {
            inner,
            history,
            queue,
        }
    }
}

impl<R, H, Q> NotifyingSecurityEventRecorder<R, H, Q>
where
    H: LoginHistory,
{
    async fn new_device_notice(&self, event: &mut NewSecurityEvent) -> Option<SecurityNotice> {
        let user_id = event.user_id.clone()?;
        let key = device_key(event.user_agent.as_deref(), event.ip.as_deref());
        if let Value::Object(metadata) = &mut event.metadata {
            metadata.insert(DEVICE_KEY_METADATA.to_owned(), json!(key));
        } else {
            event.metadata = json!({ DEVICE_KEY_METADATA: key });
        }
        match self.history.device_history(&user_id, &key).await {
            Ok(DeviceHistory::New) => Some(SecurityNotice::from_event(
                SecurityNoticeKind::NewDevice,
                &user_id,
                event,
            )),
            Ok(_) => None,
            Err(error) => {
                tracing::warn!(%error, "failed to check the login history of a device");
                None
            }
        }
    }
}

#[async_trait]
impl<R, H, Q> SecurityEventRecorder for NotifyingSecurityEventRecorder<R, H, Q>
where
    R: SecurityEventRecorder,
    H: LoginHistory,
    Q: SecurityNoticeQueue,
{
    async fn record(&self, mut event: NewSecurityEvent) -> Result<(), AppError> {
        let notice = match event.kind {
            SecurityEventKind::LoginSucceeded => self.new_device_notice(&mut event).await,
            kind => SecurityNoticeKind::for_event(kind).and_then(|notice_kind| {
                event
                    .user_id
                    .as_ref()
                    .map(|user_id| SecurityNotice::from_event(notice_kind, user_id, &event))
            }),
        };
        self.inner.record(event).await?;
        if let Some(notice) = notice
            && let Err(error) = self.queue.enqueue(notice).await
        {
            tracing::warn!(%error, "failed to enqueue a security e-mail");
        }
        Ok(())
    }
}

pub struct SecurityEmailHandler<U, M> {
    users: U,
    mailer: M,
    app_base_url: crate::application::organization_profile::PublicBaseUrl,
}

impl<U, M> SecurityEmailHandler<U, M> {
    pub fn new(
        users: U,
        mailer: M,
        app_base_url: impl Into<crate::application::organization_profile::PublicBaseUrl>,
    ) -> Self {
        Self {
            users,
            mailer,
            app_base_url: app_base_url.into(),
        }
    }

    fn security_link(&self) -> String {
        self.app_base_url.app_link("/settings/security")
    }
}

fn notice_moment(occurred_at: DateTime<Utc>, timezone: Option<&str>, locale: MailLocale) -> String {
    let zone = timezone
        .and_then(|zone| zone.parse::<Tz>().ok())
        .unwrap_or(DEFAULT_NOTICE_TIMEZONE);
    occurred_at
        .with_timezone(&zone)
        .format(locale.pick("%d/%m/%Y às %H:%M (%Z)", "%b %-d, %Y at %H:%M (%Z)"))
        .to_string()
}

pub fn render_security_notice(
    notice: &SecurityNotice,
    to: &str,
    display_name: &str,
    timezone: Option<&str>,
    link: &str,
    locale: MailLocale,
) -> EmailMessage {
    let info = describe_device(notice.user_agent.as_deref());
    let device = if info.browser.is_none() && info.os.is_none() {
        locale
            .pick("Dispositivo desconhecido", "Unknown device")
            .to_owned()
    } else {
        info.label()
    };
    let details = templates::SecurityNoticeDetails {
        moment: notice_moment(notice.occurred_at, timezone, locale),
        device,
        ip: notice
            .ip
            .clone()
            .unwrap_or_else(|| locale.pick("desconhecido", "unknown").to_owned()),
    };
    match notice.kind {
        SecurityNoticeKind::NewDevice => {
            templates::security_new_device(to, display_name, &details, link, locale)
        }
        SecurityNoticeKind::PasswordChanged => {
            templates::security_password_changed(to, display_name, &details, link, locale)
        }
        SecurityNoticeKind::TwoFactorEnabled => {
            templates::security_two_factor_enabled(to, display_name, &details, link, locale)
        }
        SecurityNoticeKind::TwoFactorDisabled => {
            templates::security_two_factor_disabled(to, display_name, &details, link, locale)
        }
    }
}

#[async_trait]
impl<U, M> JobHandler for SecurityEmailHandler<U, M>
where
    U: UserRepository,
    M: Mailer,
{
    fn kind(&self) -> &str {
        SECURITY_EMAIL_JOB
    }

    async fn handle(&self, _context: &JobContext, payload: Value) -> Result<(), AppError> {
        let notice: SecurityNotice = serde_json::from_value(payload)
            .map_err(|error| AppError::Validation(format!("invalid security notice: {error}")))?;
        let Ok(user_id) = UserId::new(notice.user_id.clone()) else {
            return Ok(());
        };
        let Some(user) = self.users.find_by_id(&user_id).await? else {
            return Ok(());
        };
        if !user.is_active() {
            return Ok(());
        }
        let message = render_security_notice(
            &notice,
            user.email.as_str(),
            user.display_name.as_str(),
            user.timezone.as_deref(),
            &self.security_link(),
            MailLocale::resolve(user.locale.as_deref(), None),
        );
        match self.mailer.send(message).await {
            Err(error) if is_mail_not_configured(&error) => Ok(()),
            result => result,
        }
    }
}
