use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::Value;
use utoipa::ToSchema;

use aphel_kernel::domain::access::Permission;
use aphel_kernel::errors::AppError;
use aphel_kernel::modules::{SettingField, SettingKind, SettingOption};
use aphel_sdk::calendar::BusyInterval;
use aphel_sdk::sso::IdentityProvider;

pub(crate) const CURSOR_EXPIRED: &str = "calendar_cursor_expired";
pub(crate) const REMOTE_NOT_FOUND: &str = "calendar_event_not_found";
pub(crate) const CALENDAR_API_FAILED: &str = "calendar_api_failed";
pub(crate) const NOT_CONNECTED: &str = "calendar_not_connected";
pub(crate) const NO_REFRESH_TOKEN: &str = "calendar_no_refresh_token";
pub(crate) const AUTHORIZATION_REVOKED: &str = "calendar_authorization_revoked";
pub(crate) const MEMBER_UNAVAILABLE: &str = "calendar_member_unavailable";
pub(crate) const PERMISSION_REVOKED: &str = "calendar_permission_revoked";
pub(crate) const MEETING_TYPE_NOT_FOUND: &str = "calendar_meeting_type_not_found";
pub(crate) const EVENT_FAILED: &str = "calendar_event_failed";

pub(crate) const STATUS_ACTIVE: &str = "active";
pub(crate) const STATUS_ERROR: &str = "error";
pub(crate) const STATUS_REVOKED: &str = "revoked";

pub(crate) const MEETING_TYPE: &str = "meeting_type";
pub(crate) const IMPORT_SCOPE: &str = "import_scope";
pub(crate) const IMPORT_ALL: &str = "all";
pub(crate) const IMPORT_WITH_CONTACTS: &str = "with_contacts";
pub(crate) const DEFAULT_MEETING_TYPE: &str = "meeting";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteEvent {
    pub id: String,
    pub etag: Option<String>,
    pub updated_at: Option<DateTime<Utc>>,
    pub cancelled: bool,
    pub all_day: bool,
    pub title: String,
    pub description: Option<String>,
    pub start: Option<DateTime<Utc>>,
    pub end: Option<DateTime<Utc>>,
    pub attendees: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalEvent {
    pub title: String,
    pub description: Option<String>,
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RemoteChanges {
    pub events: Vec<RemoteEvent>,
    pub cursor: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SyncWindow {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

impl SyncWindow {
    pub(crate) fn contains(&self, instant: DateTime<Utc>) -> bool {
        instant >= self.start && instant <= self.end
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CalendarSpec {
    pub key: &'static str,
    pub permission: &'static str,
    pub connected_event: &'static str,
    pub disconnected_event: &'static str,
}

impl CalendarSpec {
    pub(crate) fn permission(&self) -> Permission {
        Permission::from_static(self.permission)
    }

    pub(crate) fn page_path(&self) -> String {
        format!("/{}", self.key)
    }
}

#[async_trait]
pub trait CalendarApi: IdentityProvider {
    fn spec(&self) -> CalendarSpec;

    fn consent_scopes(&self) -> &'static [&'static str];

    fn consent_params(&self) -> Vec<(&'static str, String)>;

    fn refresh_scopes(&self) -> Option<&'static str> {
        None
    }

    async fn changes(
        &self,
        access_token: &str,
        cursor: Option<&str>,
        window: SyncWindow,
    ) -> Result<RemoteChanges, AppError>;

    async fn create_event(
        &self,
        access_token: &str,
        event: &LocalEvent,
    ) -> Result<RemoteEvent, AppError>;

    async fn update_event(
        &self,
        access_token: &str,
        remote_id: &str,
        event: &LocalEvent,
    ) -> Result<RemoteEvent, AppError>;

    async fn delete_event(&self, access_token: &str, remote_id: &str) -> Result<(), AppError>;

    async fn busy(
        &self,
        access_token: &str,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<Vec<BusyInterval>, AppError>;
}

pub fn calendar_setting_fields() -> Vec<SettingField> {
    vec![
        SettingField::new(
            MEETING_TYPE,
            "Tipo de atividade das reuniões",
            SettingKind::Text,
        )
        .default_value(Value::String(DEFAULT_MEETING_TYPE.to_owned()))
        .help("Chave do tipo de atividade usado para reuniões sincronizadas (padrão: meeting)"),
        SettingField::new(
            IMPORT_SCOPE,
            "Eventos importados da agenda",
            SettingKind::Select {
                options: vec![
                    SettingOption {
                        value: IMPORT_WITH_CONTACTS,
                        label: "Só eventos com algum contato do CRM",
                    },
                    SettingOption {
                        value: IMPORT_ALL,
                        label: "Todos os eventos com horário",
                    },
                ],
            },
        )
        .default_value(Value::String(IMPORT_WITH_CONTACTS.to_owned())),
    ]
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct CalendarConnectionStatus {
    #[schema(example = "ana@company.com")]
    pub(crate) account_email: Option<String>,
    #[schema(example = "active")]
    pub(crate) status: String,
    pub(crate) last_synced_at: Option<String>,
    pub(crate) last_error_code: Option<String>,
    pub(crate) last_error_message: Option<String>,
    pub(crate) imported_count: u32,
    pub(crate) exported_count: u32,
    pub(crate) connected_at: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct CalendarStatus {
    pub(crate) can_connect: bool,
    pub(crate) configured: bool,
    pub(crate) connection: Option<CalendarConnectionStatus>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct CalendarConnectUrl {
    pub(crate) url: String,
}
