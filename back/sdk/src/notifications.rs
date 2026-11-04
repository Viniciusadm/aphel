use aphel_kernel::application::files::UserReference;
use aphel_kernel::application::mail::MailLocale;
use aphel_kernel::application::shared::{DomainEvent, RecordRef};
use aphel_kernel::domain::UserId;
use aphel_kernel::errors::AppError;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use serde_json::{Value, json};
use uuid::Uuid;

pub use aphel_contract_notifications::*;

pub trait NotificationKindExt {
    fn all() -> Vec<NotificationKind>;
    fn parse(value: &str) -> Option<NotificationKind>;
    fn lookup(value: &str) -> Option<NotificationKindDefinition>;
    fn definition(&self) -> Option<NotificationKindDefinition>;
    fn channels(&self) -> &'static [NotificationChannel];
    fn supports(&self, channel: NotificationChannel) -> bool;
    fn default_enabled(&self, channel: NotificationChannel) -> bool;
}

impl NotificationKindExt for NotificationKind {
    fn all() -> Vec<Self> {
        aphel_kernel::modules::app()
            .contributions::<NotificationKindDefinition>()
            .into_iter()
            .map(|definition| definition.kind)
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    fn parse(value: &str) -> Option<Self> {
        Self::lookup(value).map(|definition| definition.kind)
    }

    fn lookup(value: &str) -> Option<NotificationKindDefinition> {
        aphel_kernel::modules::app()
            .contributions::<NotificationKindDefinition>()
            .into_iter()
            .rev()
            .find(|definition| definition.kind.as_str() == value)
            .map(|definition| *definition)
    }

    fn definition(&self) -> Option<NotificationKindDefinition> {
        Self::lookup(self.as_str())
    }

    fn channels(&self) -> &'static [NotificationChannel] {
        self.definition()
            .map(|definition| definition.channels)
            .unwrap_or(&[])
    }

    fn supports(&self, channel: NotificationChannel) -> bool {
        self.channels().contains(&channel)
    }

    fn default_enabled(&self, channel: NotificationChannel) -> bool {
        let Some(definition) = self.definition() else {
            return false;
        };
        if !definition.channels.contains(&channel) {
            return false;
        }
        match channel {
            NotificationChannel::InApp => true,
            NotificationChannel::Email => definition.email_default,
            NotificationChannel::Push => definition.push_default,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct NotificationView {
    pub id: Uuid,
    pub kind: NotificationKind,
    pub title: String,
    pub body: Option<String>,
    pub record: Option<RecordRef>,
    pub actor: Option<UserReference>,
    pub link: Option<String>,
    pub data: Value,
    pub read_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Recipient {
    pub user_id: UserId,
    pub display_name: String,
    pub email: String,
    pub locale: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordSummary {
    pub title: String,
    pub owner_id: Option<UserId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DueTask {
    pub id: Uuid,
    pub owner_id: UserId,
    pub title: String,
    pub due_at: DateTime<Utc>,
}

#[async_trait]
pub trait RecordFollowerSource: Send + Sync {
    async fn followers(
        &self,
        pool: &sqlx::MySqlPool,
        record: &RecordRef,
    ) -> Result<Vec<UserId>, AppError>;
}

#[async_trait]
pub trait NotificationDirectory: Send + Sync {
    async fn visible_title(
        &self,
        user_id: &UserId,
        record: &RecordRef,
    ) -> Result<Option<String>, AppError>;
    async fn recipient(&self, user_id: &UserId) -> Result<Option<Recipient>, AppError>;
    async fn record_summary(&self, record: &RecordRef) -> Result<Option<RecordSummary>, AppError>;
    async fn can_read(&self, user_id: &UserId, record: &RecordRef) -> Result<bool, AppError>;
    async fn followers(&self, record: &RecordRef) -> Result<Vec<UserId>, AppError>;
    async fn organization_timezone(&self) -> Result<Tz, AppError>;
    async fn api_token_owner(&self, token_id: &str) -> Result<Option<UserId>, AppError>;
    async fn user_timezone(&self, user_id: &UserId) -> Result<Tz, AppError>;
    async fn open_tasks_due(
        &self,
        owner_id: Option<&UserId>,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<Vec<DueTask>, AppError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LocalizedContent {
    RecordType(String),
    DefaultReminder,
    DailyDigest {
        count: usize,
        lines: Vec<String>,
        more: usize,
    },
}

pub trait NotificationTexts: Send + Sync {
    fn headline(&self, view: &NotificationView, actor: &str, locale: MailLocale) -> Option<String>;

    fn notice(&self, _view: &NotificationView, _locale: MailLocale) -> Option<String> {
        None
    }

    fn action_label(&self, _view: &NotificationView, _locale: MailLocale) -> Option<&'static str> {
        None
    }

    fn default_path(&self, _view: &NotificationView) -> Option<&'static str> {
        None
    }
}

pub fn data_text<'a>(view: &'a NotificationView, key: &str) -> Option<&'a str> {
    view.data.get(key).and_then(Value::as_str)
}

#[derive(Debug, Clone, PartialEq)]
pub struct NotificationIntent {
    pub recipient: UserId,
    pub kind: NotificationKind,
    pub title: String,
    pub body: Option<String>,
    pub record: Option<RecordRef>,
    pub actor_id: Option<UserId>,
    pub link: Option<String>,
    pub data: Value,
    pub dedupe_key: String,
    pub localized: Option<LocalizedContent>,
}

impl NotificationIntent {
    pub fn new(
        recipient: UserId,
        kind: NotificationKind,
        title: impl Into<String>,
        dedupe_key: impl Into<String>,
    ) -> Self {
        Self {
            recipient,
            kind,
            title: title.into(),
            body: None,
            record: None,
            actor_id: None,
            link: None,
            data: json!({}),
            dedupe_key: dedupe_key.into(),
            localized: None,
        }
    }

    pub fn with_localized(mut self, content: LocalizedContent) -> Self {
        self.localized = Some(content);
        self
    }

    pub fn with_body(mut self, body: Option<String>) -> Self {
        self.body = body;
        self
    }

    pub fn with_record(mut self, record: Option<RecordRef>) -> Self {
        self.link = record.as_ref().and_then(record_path);
        self.record = record;
        self
    }

    pub fn with_link(mut self, link: Option<String>) -> Self {
        self.link = link;
        self
    }

    pub fn with_actor(mut self, actor_id: Option<UserId>) -> Self {
        self.actor_id = actor_id;
        self
    }

    pub fn with_data(mut self, data: Value) -> Self {
        self.data = data;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DispatchOutcome {
    Delivered {
        notification_id: Uuid,
        in_app: bool,
        email: bool,
        push: bool,
    },
    Duplicate,
    Muted,
    SkippedRecipient,
}

pub fn record_path(record: &RecordRef) -> Option<String> {
    let record_type = record.record_type.as_str();
    if let Some(key) = record_type.strip_prefix("custom.") {
        return Some(format!("/objects/{key}/{}", record.record_id));
    }
    let base = aphel_kernel::modules::app()
        .record_types()
        .capabilities(record_type)?
        .web_path_value()?
        .to_owned();
    Some(format!("/{base}/{}", record.record_id))
}

pub const DEFAULT_SOURCE_ORDER: i32 = 0;

pub struct NotificationSourceContext<'a> {
    pub directory: &'a dyn NotificationDirectory,
    pub actor: Option<&'a UserId>,
    pub notified: &'a [UserId],
}

#[async_trait]
pub trait NotificationSource: Send + Sync {
    fn accepts(&self, event_type: &str) -> bool;

    fn order(&self) -> i32 {
        DEFAULT_SOURCE_ORDER
    }

    async fn intents(
        &self,
        context: &NotificationSourceContext<'_>,
        event: &DomainEvent,
    ) -> Result<Vec<NotificationIntent>, AppError>;
}

pub fn event_dedupe(event: &DomainEvent, kind: NotificationKind) -> String {
    format!("event:{}:{}", event.id, kind.as_str())
}

pub fn payload_text(payload: &Value, key: &str) -> Option<String> {
    payload.get(key).and_then(Value::as_str).map(str::to_owned)
}

pub fn payload_user(value: Option<&Value>) -> Option<UserId> {
    value
        .and_then(Value::as_str)
        .and_then(|id| UserId::new(id).ok())
}

pub fn requested_job_intents(
    event: &DomainEvent,
    kind: NotificationKind,
    link: Option<String>,
    data: Value,
) -> Vec<NotificationIntent> {
    let Some(requested_by) = payload_user(event.payload.get("requested_by")) else {
        return Vec::new();
    };
    let record_type = payload_text(&event.payload, "record_type").unwrap_or_default();
    vec![
        NotificationIntent::new(
            requested_by,
            kind,
            record_type_label(&record_type, MailLocale::default()),
            event_dedupe(event, kind),
        )
        .with_localized(LocalizedContent::RecordType(record_type))
        .with_link(link)
        .with_data(data),
    ]
}

pub fn record_type_label(record_type: &str, locale: MailLocale) -> String {
    aphel_kernel::modules::app()
        .record_types()
        .capabilities(record_type)
        .and_then(|capabilities| capabilities.plural_labels_value())
        .map_or_else(
            || record_type.to_owned(),
            |labels| labels.get(locale.html_lang()).to_owned(),
        )
}
#[async_trait]
pub trait DigestSource: Send + Sync {
    async fn open_tasks_due(
        &self,
        pool: &sqlx::MySqlPool,
        owner: Option<&UserId>,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
        limit: u64,
    ) -> Result<Vec<DueTask>, AppError>;
}

#[async_trait]
pub trait NotificationDelivery: Send + Sync {
    fn directory(
        &self,
        pool: sqlx::MySqlPool,
        registry: std::sync::Arc<aphel_kernel::application::fields::RecordTypeRegistry>,
    ) -> std::sync::Arc<dyn NotificationDirectory>;

    async fn dispatch(
        &self,
        pool: sqlx::MySqlPool,
        registry: std::sync::Arc<aphel_kernel::application::fields::RecordTypeRegistry>,
        jobs: aphel_kernel::infrastructure::platform::SharedJobQueue,
        intent: NotificationIntent,
    ) -> Result<DispatchOutcome, AppError>;
}

#[derive(Clone)]
pub struct NotificationClient {
    pool: sqlx::MySqlPool,
    registry: std::sync::Arc<aphel_kernel::application::fields::RecordTypeRegistry>,
    jobs: aphel_kernel::infrastructure::platform::SharedJobQueue,
}

fn delivery() -> Result<std::sync::Arc<dyn NotificationDelivery>, AppError> {
    aphel_kernel::modules::app()
        .services()
        .get::<dyn NotificationDelivery>()
        .ok_or_else(|| {
            AppError::unavailable_code(
                "notification_delivery_unavailable",
                "notification delivery is not enabled",
            )
        })
}

impl NotificationClient {
    pub fn new(
        pool: sqlx::MySqlPool,
        registry: std::sync::Arc<aphel_kernel::application::fields::RecordTypeRegistry>,
        jobs: aphel_kernel::infrastructure::platform::SharedJobQueue,
    ) -> Self {
        Self {
            pool,
            registry,
            jobs,
        }
    }

    pub fn directory(&self) -> Result<std::sync::Arc<dyn NotificationDirectory>, AppError> {
        Ok(delivery()?.directory(self.pool.clone(), self.registry.clone()))
    }
}

pub async fn dispatch(
    client: &NotificationClient,
    intent: NotificationIntent,
) -> Result<DispatchOutcome, AppError> {
    delivery()?
        .dispatch(
            client.pool.clone(),
            client.registry.clone(),
            client.jobs.clone(),
            intent,
        )
        .await
}

pub use aphel_kernel::modules::intern::intern;
pub fn runtime_notification_kind(
    kind: &str,
    channels: &'static [NotificationChannel],
    email_default: bool,
    push_default: bool,
) -> NotificationKindDefinition {
    NotificationKindDefinition {
        kind: NotificationKind::from_static(intern(kind)),
        channels,
        email_default,
        push_default,
    }
}
