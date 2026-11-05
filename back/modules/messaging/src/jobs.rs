use aphel_sdk::messaging::channel_provider;
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::channels::{self, ChannelContext, FetchState, InboundMessage};
use crate::sending::{self, SEND_JOB};
use crate::store::{self, StoredChannel};
use crate::{actor, error_code, inbound, job_kind, job_unique_key, text};
use aphel_kernel::Ctx;
use aphel_kernel::errors::AppError;
use aphel_kernel::modules::{WorkerEnv, WorkerParts};
use aphel_kernel::state::AppState;
use aphel_sdk::platform::application::jobs::ScheduledJobDefinition;
use aphel_sdk::platform::application::shared::{Job, JobContext, JobHandler};
use aphel_sdk::platform::domain::access::AccessScope;

pub(crate) const POLL_JOB: &str = "poll";
pub(crate) const POLL_CHANNEL_JOB: &str = "poll_channel";
pub(crate) const INBOUND_JOB: &str = "inbound";
const POLL_CRON: &str = "0 */2 * * * *";
const POLL_BATCH: usize = 25;
const INBOUND_ATTEMPTS: u32 = 8;

pub(crate) fn messaging_worker(env: &WorkerEnv) -> WorkerParts {
    let state = env.runtime_state();
    let mut parts = WorkerParts::default()
        .handler(Arc::new(SendHandler::new(state.clone())))
        .handler(Arc::new(PollHandler::new(state.clone())))
        .handler(Arc::new(PollChannelHandler::new(state.clone())))
        .handler(Arc::new(InboundHandler::new(state)));
    match ScheduledJobDefinition::new(job_kind(POLL_JOB), POLL_CRON) {
        Ok(schedule) => parts = parts.schedule(schedule),
        Err(error) => tracing::error!("module messaging: invalid schedule {POLL_JOB}: {error}"),
    }
    parts
}

fn payload_uuid(payload: &Value, key: &str) -> Result<Uuid, AppError> {
    payload
        .get(key)
        .and_then(Value::as_str)
        .and_then(|value| Uuid::parse_str(value).ok())
        .ok_or_else(|| AppError::validation_code("invalid_job_payload", format!("missing {key}")))
}

pub(crate) async fn enqueue_inbound(
    ctx: &Ctx<'_>,
    channel: &StoredChannel,
    message: &InboundMessage,
) -> Result<(), AppError> {
    ctx.jobs()
        .enqueue_now(
            Job::new(
                job_kind(INBOUND_JOB),
                json!({
                    "channel_id": channel.id,
                    "message": message,
                }),
            )
            .with_max_attempts(INBOUND_ATTEMPTS),
        )
        .await?;
    Ok(())
}

struct Handler {
    state: AppState,
    kind: String,
}

impl Handler {
    fn new(state: AppState, kind: &str) -> Self {
        Self {
            state,
            kind: job_kind(kind),
        }
    }
}

struct SendHandler(Handler);

impl SendHandler {
    fn new(state: AppState) -> Self {
        Self(Handler::new(state, SEND_JOB))
    }
}

#[async_trait]
impl JobHandler for SendHandler {
    fn kind(&self) -> &str {
        &self.0.kind
    }

    async fn handle(&self, context: &JobContext, payload: Value) -> Result<(), AppError> {
        let message_id = payload_uuid(&payload, "message_id")?;
        let scope = AccessScope::system();
        let ctx = Ctx::from_state(&self.0.state, &scope).await?;
        sending::deliver(&ctx, context, message_id).await
    }
}

struct PollHandler(Handler);

impl PollHandler {
    fn new(state: AppState) -> Self {
        Self(Handler::new(state, POLL_JOB))
    }
}

#[async_trait]
impl JobHandler for PollHandler {
    fn kind(&self) -> &str {
        &self.0.kind
    }

    async fn handle(&self, _context: &JobContext, _payload: Value) -> Result<(), AppError> {
        let scope = AccessScope::system();
        let ctx = Ctx::from_state(&self.0.state, &scope).await?;
        for channel in store::polling_channels(ctx.pool()).await? {
            ctx.jobs()
                .enqueue_now(
                    Job::new(
                        job_kind(POLL_CHANNEL_JOB),
                        json!({ "channel_id": channel.id }),
                    )
                    .with_unique_key(job_unique_key(&format!("poll:{}", channel.id)))
                    .with_max_attempts(1),
                )
                .await?;
        }
        Ok(())
    }
}

struct PollChannelHandler(Handler);

impl PollChannelHandler {
    fn new(state: AppState) -> Self {
        Self(Handler::new(state, POLL_CHANNEL_JOB))
    }
}

async fn poll_channel(ctx: &Ctx<'_>, channel: &StoredChannel) -> Result<Option<String>, AppError> {
    let pool = ctx.pool();
    let Some(scope) = actor::acting_scope(ctx, channel).await? else {
        return Ok(Some("no active member can act for this channel".to_owned()));
    };
    let credentials = channels::load_credentials(ctx, channel).await?;
    let context = ChannelContext {
        config: &channel.config,
        credentials: &credentials,
    };
    let state = FetchState {
        uid_validity: channel.imap_uid_validity,
        last_uid: channel.imap_last_uid,
    };
    let batch = match channel_provider(channel.kind.as_str())?
        .fetch(&ctx.with_scope(&scope), &context, state, POLL_BATCH)
        .await
    {
        Ok(batch) => batch,
        Err(error) => return Ok(Some(error.to_string())),
    };
    if batch.state != state {
        store::save_imap_state(
            pool,
            channel,
            batch.state.uid_validity,
            batch.state.last_uid,
        )
        .await?;
    }
    for (uid, message) in batch.messages {
        if let Some(message) = message
            && let Err(error) = inbound::ingest(ctx, channel, message).await
        {
            return Ok(Some(text::truncate(
                &format!("message {uid} not stored: {}", error_code(&error)),
                500,
            )));
        }
        store::save_imap_state(pool, channel, batch.state.uid_validity, Some(uid)).await?;
    }
    Ok(None)
}

#[async_trait]
impl JobHandler for PollChannelHandler {
    fn kind(&self) -> &str {
        &self.0.kind
    }

    async fn handle(&self, _context: &JobContext, payload: Value) -> Result<(), AppError> {
        let channel_id = payload_uuid(&payload, "channel_id")?;
        let scope = AccessScope::system();
        let ctx = Ctx::from_state(&self.0.state, &scope).await?;
        let Some(channel) = store::find_channel(ctx.pool(), channel_id).await? else {
            return Ok(());
        };
        if !channel.is_active
            || !channel_provider(channel.kind.as_str()).is_ok_and(|provider| provider.polls())
        {
            return Ok(());
        }
        let error = poll_channel(&ctx, &channel).await?;
        store::record_channel_poll(ctx.pool(), &channel, error.as_deref()).await
    }
}

struct InboundHandler(Handler);

impl InboundHandler {
    fn new(state: AppState) -> Self {
        Self(Handler::new(state, INBOUND_JOB))
    }
}

#[async_trait]
impl JobHandler for InboundHandler {
    fn kind(&self) -> &str {
        &self.0.kind
    }

    async fn handle(&self, _context: &JobContext, payload: Value) -> Result<(), AppError> {
        let channel_id = payload_uuid(&payload, "channel_id")?;
        let message: InboundMessage =
            serde_json::from_value(payload.get("message").cloned().unwrap_or(Value::Null))
                .map_err(|error| AppError::Internal(error.to_string()))?;
        let scope = AccessScope::system();
        let ctx = Ctx::from_state(&self.0.state, &scope).await?;
        let Some(channel) = store::find_channel(ctx.pool(), channel_id).await? else {
            return Ok(());
        };
        inbound::ingest(&ctx, &channel, message).await.map(|_| ())
    }
}
