use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::Value;
use utoipa::ToSchema;

use crate::calendar::BusyInterval;
use crate::sso::IdentityProvider;
use aphel_kernel::domain::access::Permission;
use aphel_kernel::errors::AppError;
use aphel_kernel::modules::{SettingField, SettingKind, SettingOption};

pub const CURSOR_EXPIRED: &str = "calendar_cursor_expired";
pub const REMOTE_NOT_FOUND: &str = "calendar_event_not_found";
pub const CALENDAR_API_FAILED: &str = "calendar_api_failed";
pub const NOT_CONNECTED: &str = "calendar_not_connected";
pub const NO_REFRESH_TOKEN: &str = "calendar_no_refresh_token";
pub const AUTHORIZATION_REVOKED: &str = "calendar_authorization_revoked";
pub const MEMBER_UNAVAILABLE: &str = "calendar_member_unavailable";
pub const PERMISSION_REVOKED: &str = "calendar_permission_revoked";
pub const MEETING_TYPE_NOT_FOUND: &str = "calendar_meeting_type_not_found";
pub const EVENT_FAILED: &str = "calendar_event_failed";

pub const STATUS_ACTIVE: &str = "active";
pub const STATUS_ERROR: &str = "error";
pub const STATUS_REVOKED: &str = "revoked";

pub const MEETING_TYPE: &str = "meeting_type";
pub const IMPORT_SCOPE: &str = "import_scope";
pub const IMPORT_ALL: &str = "all";
pub const IMPORT_WITH_CONTACTS: &str = "with_contacts";
pub const DEFAULT_MEETING_TYPE: &str = "meeting";

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
    pub fn contains(&self, instant: DateTime<Utc>) -> bool {
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
    pub fn permission(&self) -> Permission {
        Permission::from_static(self.permission)
    }

    pub fn page_path(&self) -> String {
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
    pub account_email: Option<String>,
    #[schema(example = "active")]
    pub status: String,
    pub last_synced_at: Option<String>,
    pub last_error_code: Option<String>,
    pub last_error_message: Option<String>,
    pub imported_count: u32,
    pub exported_count: u32,
    pub connected_at: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct CalendarStatus {
    pub can_connect: bool,
    pub configured: bool,
    pub connection: Option<CalendarConnectionStatus>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct CalendarConnectUrl {
    pub url: String,
}

pub fn parse_instant(value: Option<&Value>) -> Option<DateTime<Utc>> {
    value
        .and_then(Value::as_str)
        .and_then(|text| DateTime::parse_from_rfc3339(text).ok())
        .map(|instant| instant.with_timezone(&Utc))
}
pub fn encode_segment(value: &str) -> String {
    value
        .bytes()
        .map(|byte| {
            if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
                (byte as char).to_string()
            } else {
                format!("%{byte:02X}")
            }
        })
        .collect()
}

#[async_trait]
pub trait RemoteCalendars: Send + Sync {
    async fn status(
        &self,
        ctx: &crate::Ctx<'_>,
        provider: &str,
    ) -> Result<CalendarStatus, AppError>;
    async fn connect_url(
        &self,
        ctx: &crate::Ctx<'_>,
        provider: &str,
    ) -> Result<CalendarConnectUrl, AppError>;
    async fn complete(
        &self,
        ctx: &crate::Ctx<'_>,
        provider: &str,
        state: &crate::sso::StoredState,
        code: &str,
    ) -> axum::response::Response;
    async fn disconnect(&self, ctx: &crate::Ctx<'_>, provider: &str) -> Result<(), AppError>;
    async fn sync_now(&self, ctx: &crate::Ctx<'_>, provider: &str) -> Result<(), AppError>;
    async fn busy(
        &self,
        ctx: &crate::Ctx<'_>,
        provider: &str,
        user: &aphel_kernel::domain::UserId,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<Option<Vec<BusyInterval>>, AppError>;
    async fn send(
        &self,
        request: reqwest::RequestBuilder,
        cursor_request: bool,
    ) -> Result<Value, AppError>;
}
fn service() -> Result<std::sync::Arc<dyn RemoteCalendars>, AppError> {
    aphel_kernel::modules::app()
        .services()
        .get::<dyn RemoteCalendars>()
        .ok_or_else(|| {
            AppError::unavailable_code(
                "remote_calendars_unavailable",
                "remote task calendars are not enabled",
            )
        })
}
pub async fn status(
    ctx: &crate::Ctx<'_>,
    api: &dyn CalendarApi,
) -> Result<CalendarStatus, AppError> {
    let Ok(service) = service() else {
        return Ok(CalendarStatus {
            can_connect: false,
            configured: false,
            connection: None,
        });
    };
    service.status(ctx, api.spec().key).await
}
pub async fn connect_url(
    ctx: &crate::Ctx<'_>,
    api: &dyn CalendarApi,
) -> Result<CalendarConnectUrl, AppError> {
    service()?.connect_url(ctx, api.spec().key).await
}
pub async fn complete_connection(
    ctx: &crate::Ctx<'_>,
    api: &dyn CalendarApi,
    state: &crate::sso::StoredState,
    code: &str,
) -> axum::response::Response {
    match service() {
        Ok(service) => service.complete(ctx, api.spec().key, state, code).await,
        Err(_) => crate::sso::login::redirect_to(&format!(
            "/{}?calendar_error=remote_calendars_unavailable",
            api.spec().key
        )),
    }
}
pub async fn disconnect(ctx: &crate::Ctx<'_>, api: &dyn CalendarApi) -> Result<(), AppError> {
    service()?.disconnect(ctx, api.spec().key).await
}
pub async fn sync_now(ctx: &crate::Ctx<'_>, api: &dyn CalendarApi) -> Result<(), AppError> {
    service()?.sync_now(ctx, api.spec().key).await
}
pub async fn send(
    request: reqwest::RequestBuilder,
    cursor_request: bool,
) -> Result<Value, AppError> {
    service()?.send(request, cursor_request).await
}
pub struct CalendarBusy<A: CalendarApi> {
    api: std::sync::Arc<A>,
    key: String,
}
impl<A: CalendarApi> CalendarBusy<A> {
    pub fn new(api: std::sync::Arc<A>) -> Self {
        Self {
            key: format!("{}.calendar", api.spec().key),
            api,
        }
    }
}
#[async_trait]
impl<A: CalendarApi + 'static> crate::calendar::CalendarAvailability for CalendarBusy<A> {
    fn key(&self) -> &str {
        &self.key
    }
    async fn busy(
        &self,
        ctx: &crate::Ctx<'_>,
        user: &aphel_kernel::domain::UserId,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<Option<Vec<BusyInterval>>, AppError> {
        let Ok(service) = service() else {
            return Ok(None);
        };
        service.busy(ctx, self.api.spec().key, user, from, to).await
    }
}
