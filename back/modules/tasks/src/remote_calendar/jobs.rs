use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{Value, json};
use uuid::Uuid;

use super::engine::{self, configured_meeting_key, failure_status, schedule_push};
use super::model::CalendarApi;
use super::store;
use aphel_kernel::Ctx;
use aphel_kernel::application::jobs::ScheduledJobDefinition;
use aphel_kernel::application::shared::{
    DomainEvent, EventConsumer, Job, JobContext, JobHandler, JobQueue,
};
use aphel_kernel::domain::access::AccessScope;
use aphel_kernel::errors::AppError;
use aphel_kernel::modules::{WorkerEnv, WorkerParts};
use aphel_kernel::state::AppState;
use aphel_sdk::sso::errors::{error_code, is_internal, is_unavailable};

pub(crate) const SYNC_ALL_KIND: &str = "tasks.calendar_sync_all";
pub(crate) const SYNC_KIND: &str = "tasks.calendar_sync";
pub(crate) const PUSH_KIND: &str = "tasks.calendar_push";
pub(crate) const SYNC_SCHEDULE: &str = "0 */5 * * * *";
pub(crate) const CONSUMER_NAME: &str = "tasks.calendar_export";

const ACTIVITY_CREATED: &str = "activity.created";
const ACTIVITY_UPDATED: &str = "activity.updated";
const ACTIVITY_DELETED: &str = "activity.deleted";

fn uuid_field(payload: &Value, key: &str) -> Option<Uuid> {
    payload
        .get(key)
        .and_then(Value::as_str)
        .and_then(|value| Uuid::parse_str(value).ok())
}

pub(crate) async fn enqueue_sync(ctx: &Ctx<'_>, key: &str, user_id: Uuid) -> Result<(), AppError> {
    ctx.jobs()
        .enqueue_now(
            Job::new(SYNC_KIND, json!({ "provider": key, "user_id": user_id }))
                .with_unique_key(format!("{key}.sync:{user_id}"))
                .with_max_attempts(1),
        )
        .await?;
    Ok(())
}

struct CalendarWorker {
    state: AppState,
}
impl CalendarWorker {
    fn new(state: &AppState) -> Self {
        Self {
            state: state.clone(),
        }
    }
}
fn payload_provider(
    ctx: &Ctx<'_>,
    payload: &Value,
) -> Result<Option<Arc<dyn CalendarApi>>, AppError> {
    let Some(key) = payload.get("provider").and_then(Value::as_str) else {
        return Ok(None);
    };
    super::service::provider(ctx, key)
}

struct SyncAllHandler(CalendarWorker);

#[async_trait]
impl JobHandler for SyncAllHandler {
    fn kind(&self) -> &str {
        SYNC_ALL_KIND
    }

    async fn handle(&self, _context: &JobContext, _payload: Value) -> Result<(), AppError> {
        let scope = AccessScope::system();
        let ctx = Ctx::from_state(&self.0.state, &scope).await?;
        for api in super::service::providers(&ctx)? {
            let key = api.spec().key;
            for connection in store::list_connections(ctx.pool(), key).await? {
                enqueue_sync(&ctx, key, connection.user_id).await?;
            }
        }
        Ok(())
    }
}

struct SyncHandler(CalendarWorker);

#[async_trait]
impl JobHandler for SyncHandler {
    fn kind(&self) -> &str {
        SYNC_KIND
    }

    async fn handle(&self, _context: &JobContext, payload: Value) -> Result<(), AppError> {
        let Some(user_id) = uuid_field(&payload, "user_id") else {
            return Ok(());
        };
        let scope = AccessScope::system();
        let ctx = Ctx::from_state(&self.0.state, &scope).await?;
        let Some(api) = payload_provider(&ctx, &payload)? else {
            return Ok(());
        };
        engine::run_sync(&ctx, api.as_ref(), user_id).await
    }
}

struct PushHandler(CalendarWorker);

#[async_trait]
impl JobHandler for PushHandler {
    fn kind(&self) -> &str {
        PUSH_KIND
    }

    async fn handle(&self, context: &JobContext, payload: Value) -> Result<(), AppError> {
        let Some(activity_id) = uuid_field(&payload, "activity_id") else {
            return Ok(());
        };
        let owner = uuid_field(&payload, "owner_id");
        let deleted = payload
            .get("deleted")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let scope = AccessScope::system();
        let ctx = Ctx::from_state(&self.0.state, &scope).await?;
        let Some(api) = payload_provider(&ctx, &payload)? else {
            return Ok(());
        };
        match engine::push(&ctx, api.as_ref(), activity_id, owner, deleted).await {
            Ok(()) => Ok(()),
            Err(error) if is_unavailable(&error) && context.attempt < context.max_attempts => {
                Err(error)
            }
            Err(error) => {
                let key = api.spec().key;
                let code = error_code(&error);
                tracing::warn!(module = key, code, "calendar export failed");
                let user = match owner {
                    Some(owner) => Some(owner),
                    None => store::link_by_activity(ctx.pool(), key, activity_id)
                        .await?
                        .map(|(user, _)| user),
                };
                if let Some(user_id) = user {
                    let message = if is_internal(&error) {
                        "unexpected failure while sending the meeting to the calendar".to_owned()
                    } else {
                        error.client_message().to_owned()
                    };
                    store::mark_failure(
                        ctx.pool(),
                        key,
                        user_id,
                        failure_status(&error),
                        code,
                        &message,
                    )
                    .await?;
                }
                Ok(())
            }
        }
    }
}

struct ActivityExportConsumer {
    state: AppState,
}
#[async_trait]
impl EventConsumer for ActivityExportConsumer {
    fn name(&self) -> &str {
        CONSUMER_NAME
    }
    fn accepts(&self, event_type: &str) -> bool {
        matches!(
            event_type,
            ACTIVITY_CREATED | ACTIVITY_UPDATED | ACTIVITY_DELETED
        )
    }
    async fn consume(&self, event: &DomainEvent) -> Result<(), AppError> {
        let Some(record) = &event.record else {
            return Ok(());
        };
        if !engine::is_activity(&record.record_type) {
            return Ok(());
        }
        let scope = AccessScope::system();
        let ctx = Ctx::from_state(&self.state, &scope).await?;
        let mut failure = None;
        for api in super::service::providers(&ctx)? {
            if let Err(error) = export_activity(&ctx, api.as_ref(), event, record.record_id).await {
                failure.get_or_insert(error);
            }
        }
        match failure {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }
}
async fn export_activity(
    ctx: &Ctx<'_>,
    api: &dyn CalendarApi,
    event: &DomainEvent,
    activity_id: Uuid,
) -> Result<(), AppError> {
    let key = api.spec().key;
    let deleted = event.event_type == ACTIVITY_DELETED;
    let linked = store::link_by_activity(ctx.pool(), key, activity_id).await?;
    if let Some((user_id, _)) = linked {
        return schedule_push(ctx, key, activity_id, Some(user_id), deleted).await;
    }
    if deleted {
        return Ok(());
    }
    let meeting_key = configured_meeting_key(ctx, key).await?;
    if event
        .payload
        .get("activity_type_key")
        .and_then(Value::as_str)
        != Some(meeting_key.as_str())
    {
        return Ok(());
    }
    let Some(owner_id) = uuid_field(&event.payload, "owner_id") else {
        return Ok(());
    };
    let connected = store::find_connection(ctx.pool(), key, owner_id)
        .await?
        .is_some_and(|connection| connection.syncable());
    if !connected {
        return Ok(());
    }
    schedule_push(ctx, key, activity_id, Some(owner_id), false).await
}
pub fn schedule() -> Result<ScheduledJobDefinition, AppError> {
    ScheduledJobDefinition::new(SYNC_ALL_KIND, SYNC_SCHEDULE)
}
pub fn worker(env: &WorkerEnv) -> WorkerParts {
    let state = env.runtime_state();
    WorkerParts::default()
        .handler(Arc::new(SyncAllHandler(CalendarWorker::new(&state))))
        .handler(Arc::new(SyncHandler(CalendarWorker::new(&state))))
        .handler(Arc::new(PushHandler(CalendarWorker::new(&state))))
        .consumer(Arc::new(ActivityExportConsumer { state }))
}
