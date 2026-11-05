pub mod remote_calendar;
use crate::AppError;
pub use aphel_contract_tasks::*;
use aphel_kernel::domain::UserId;
use chrono::{DateTime, Utc};
use sqlx::MySqlPool;
use uuid::Uuid;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DueTaskReminder {
    pub id: Uuid,
    pub owner_id: UserId,
    pub title: String,
    pub due_at: DateTime<Utc>,
    pub reminder_at: DateTime<Utc>,
    pub minutes_before: u32,
}

#[async_trait::async_trait]
pub trait TaskReminderSource: Send + Sync {
    async fn due(
        &self,
        pool: &MySqlPool,
        since: DateTime<Utc>,
        now: DateTime<Utc>,
        limit: u32,
    ) -> Result<Vec<DueTaskReminder>, AppError>;
    async fn mark_sent(
        &self,
        pool: &MySqlPool,
        reminder: &DueTaskReminder,
        now: DateTime<Utc>,
    ) -> Result<(), AppError>;
}

use serde_json::{Map, Value};
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordLinkInput {
    pub record_type: String,
    pub record_id: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ActivityLocationInput {
    pub latitude: f64,
    pub longitude: f64,
    pub accuracy_m: Option<f64>,
    pub captured_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct CreateActivityCommand {
    pub title: String,
    pub activity_type_id: String,
    pub status: Option<String>,
    pub priority: Option<String>,
    pub due_at: Option<String>,
    pub completed_at: Option<String>,
    pub owner_id: Option<String>,
    pub record_type: Option<String>,
    pub record_id: Option<String>,
    pub links: Vec<RecordLinkInput>,
    pub duration_minutes: Option<i64>,
    pub description: Option<String>,
    pub location: Option<ActivityLocationInput>,
    pub custom_fields: Map<String, Value>,
    pub ip: Option<String>,
}

#[async_trait::async_trait]
pub trait TaskWriter: Send + Sync {
    async fn create(
        &self,
        ctx: &crate::Ctx<'_>,
        command: CreateActivityCommand,
    ) -> Result<Uuid, AppError>;
}

#[derive(Debug, Clone, PartialEq)]
pub struct MeetingActivity {
    pub activity_type_id: Uuid,
    pub title: String,
    pub owner_id: UserId,
    pub record: crate::shared::RecordRef,
    pub starts_at: DateTime<Utc>,
    pub duration_minutes: u32,
    pub description: Option<String>,
    pub ip: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeetingActivityMove {
    pub starts_at: DateTime<Utc>,
    pub duration_minutes: u32,
    pub ip: Option<String>,
}

#[async_trait::async_trait]
pub trait MeetingScheduler: Send + Sync {
    async fn schedule(
        &self,
        ctx: &crate::Ctx<'_>,
        tx: &mut aphel_kernel::database::transaction::MySqlTx,
        meeting: MeetingActivity,
    ) -> Result<Uuid, AppError>;

    async fn move_to(
        &self,
        ctx: &crate::Ctx<'_>,
        tx: &mut aphel_kernel::database::transaction::MySqlTx,
        scope: &crate::access::AccessScope,
        activity_id: Uuid,
        change: MeetingActivityMove,
    ) -> Result<Option<crate::records::PendingCommit>, AppError>;

    async fn cancel(
        &self,
        ctx: &crate::Ctx<'_>,
        tx: &mut aphel_kernel::database::transaction::MySqlTx,
        scope: &crate::access::AccessScope,
        activity_id: Uuid,
        ip: Option<String>,
    ) -> Result<Option<crate::records::PendingCommit>, AppError>;

    async fn busy(
        &self,
        ctx: &crate::Ctx<'_>,
        owner: &UserId,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
        exclude: Option<Uuid>,
    ) -> Result<Vec<crate::calendar::BusyInterval>, AppError>;
}

pub fn meetings() -> Result<std::sync::Arc<dyn MeetingScheduler>, AppError> {
    aphel_kernel::modules::app()
        .services()
        .get::<dyn MeetingScheduler>()
        .ok_or_else(|| {
            AppError::unavailable_code(
                "meeting_scheduler_unavailable",
                "meeting scheduling is not enabled",
            )
        })
}

pub fn writer() -> Result<std::sync::Arc<dyn TaskWriter>, AppError> {
    let app = aphel_kernel::modules::app();
    if app.is_enabled("tasks")
        && let Some(writer) = app.services().get::<dyn TaskWriter>()
    {
        return Ok(writer);
    }
    if app.operation("tasks.create").is_some() {
        return Ok(std::sync::Arc::new(OperationTaskWriter));
    }
    Err(AppError::unavailable_code(
        "task_writer_unavailable",
        "task creation is not enabled",
    ))
}

async fn call_operation(ctx: &crate::Ctx<'_>, key: &str, input: Value) -> Result<Value, AppError> {
    let entry = aphel_kernel::modules::app().operation(key).ok_or_else(|| {
        AppError::unavailable_code(
            "contract_unavailable",
            format!("no enabled provider for operation `{key}`"),
        )
    })?;
    entry.call(ctx, input).await
}

struct OperationTaskWriter;

#[async_trait::async_trait]
impl TaskWriter for OperationTaskWriter {
    async fn create(
        &self,
        ctx: &crate::Ctx<'_>,
        command: CreateActivityCommand,
    ) -> Result<Uuid, AppError> {
        let output = call_operation(
            ctx,
            "tasks.create",
            serde_json::json!({
                "title": command.title,
                "activity_type_id": command.activity_type_id,
                "due_at": command.due_at,
                "owner_id": command.owner_id,
                "record_type": command.record_type,
                "record_id": command.record_id,
                "description": command.description,
                "duration_minutes": command.duration_minutes,
            }),
        )
        .await?;
        output
            .get("id")
            .and_then(Value::as_str)
            .and_then(|id| Uuid::parse_str(id).ok())
            .ok_or_else(|| AppError::Internal("tasks.create returned no valid task id".to_owned()))
    }
}

struct OperationTaskTypes;

#[async_trait::async_trait]
impl TaskTypes for OperationTaskTypes {
    fn require_reader(&self, _scope: &crate::access::AccessScope) -> Result<(), AppError> {
        Ok(())
    }

    async fn list(
        &self,
        ctx: &crate::Ctx<'_>,
        _include_inactive: bool,
        _window: aphel_kernel::application::organization_admin::ListWindow,
    ) -> Result<aphel_kernel::application::organization_admin::Listing<ActivityTypeView>, AppError>
    {
        let output = call_operation(ctx, "tasks.types", serde_json::json!({})).await?;
        let now = Utc::now();
        let items: Vec<ActivityTypeView> = output
            .get("items")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .enumerate()
                    .filter_map(|(position, item)| {
                        Some(ActivityTypeView {
                            id: Uuid::parse_str(item.get("id")?.as_str()?).ok()?,
                            key: item.get("key")?.as_str()?.to_owned(),
                            name: item.get("name")?.as_str()?.to_owned(),
                            icon: None,
                            color: None,
                            is_system: false,
                            is_active: true,
                            position: i32::try_from(position).unwrap_or(i32::MAX),
                            created_at: now,
                            updated_at: now,
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();
        let total = items.len() as u64;
        Ok(aphel_kernel::application::organization_admin::Listing { items, total })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivityTypeView {
    pub id: Uuid,
    pub key: String,
    pub name: String,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub is_system: bool,
    pub is_active: bool,
    pub position: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[async_trait::async_trait]
pub trait TaskTypes: Send + Sync {
    fn require_reader(&self, scope: &crate::access::AccessScope) -> Result<(), AppError>;

    async fn list(
        &self,
        ctx: &crate::Ctx<'_>,
        include_inactive: bool,
        window: aphel_kernel::application::organization_admin::ListWindow,
    ) -> Result<aphel_kernel::application::organization_admin::Listing<ActivityTypeView>, AppError>;
}

pub fn types() -> Result<std::sync::Arc<dyn TaskTypes>, AppError> {
    let app = aphel_kernel::modules::app();
    if app.is_enabled("tasks")
        && let Some(types) = app.services().get::<dyn TaskTypes>()
    {
        return Ok(types);
    }
    if app.operation("tasks.types").is_some() {
        return Ok(std::sync::Arc::new(OperationTaskTypes));
    }
    Err(AppError::unavailable_code(
        "task_types_unavailable",
        "task types are not enabled",
    ))
}

pub mod dto;
pub const RESOURCE: aphel_kernel::application::public_events::PublicResource =
    aphel_kernel::application::public_events::PublicResource::new("tasks.task", "ActivityResponse");
pub fn instant(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}
pub struct TaskRecord {
    pub id: Uuid,
    pub record: Option<aphel_kernel::application::shared::RecordRef>,
    pub response: dto::ActivityResponse,
}
#[async_trait::async_trait]
pub trait PreparedTask: Send + Sync {
    fn id(&self) -> Uuid;
    fn record(&self) -> Option<aphel_kernel::application::shared::RecordRef>;
    fn ip(&self) -> Option<String>;
    async fn insert(
        &self,
        ctx: &crate::Ctx<'_>,
        tx: &mut aphel_kernel::database::transaction::MySqlTx,
    ) -> Result<(), AppError>;
    async fn response(&self, ctx: &crate::Ctx<'_>) -> Result<TaskRecord, AppError>;
}
#[async_trait::async_trait]
pub trait TaskTransactions: Send + Sync {
    async fn type_for_creation(
        &self,
        ctx: &crate::Ctx<'_>,
        key: &str,
    ) -> Result<Option<ActivityTypeView>, AppError>;

    async fn prepare_completed(
        &self,
        ctx: &crate::Ctx<'_>,
        command: CreateActivityCommand,
    ) -> Result<Box<dyn PreparedTask>, AppError>;
    async fn get(&self, ctx: &crate::Ctx<'_>, id: &str) -> Result<TaskRecord, AppError>;
}
pub fn transactions() -> Result<std::sync::Arc<dyn TaskTransactions>, AppError> {
    aphel_kernel::modules::app()
        .services()
        .get::<dyn TaskTransactions>()
        .ok_or_else(|| {
            AppError::unavailable_code(
                "task_transactions_unavailable",
                "task transactions are not enabled",
            )
        })
}

pub struct TaskReportQuery {
    pub from: DateTime<Utc>,
    pub to: DateTime<Utc>,
    pub owner_id: Option<UserId>,
    pub after: Option<Uuid>,
}
pub struct TaskReportSubject {
    pub id: Uuid,
    pub user: Option<aphel_kernel::application::files::UserReference>,
}
pub struct TaskReportPage {
    pub rows: Vec<TaskReportSubject>,
    pub after: Option<Uuid>,
    pub more: bool,
}
#[async_trait::async_trait]
pub trait TaskReportSubjects: Send + Sync {
    async fn page(
        &self,
        ctx: &crate::Ctx<'_>,
        query: TaskReportQuery,
    ) -> Result<TaskReportPage, AppError>;
}
