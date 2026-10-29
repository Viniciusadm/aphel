use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;
use uuid::Uuid;

use crate::service::{self, ATTACH_JOB};
use crate::{job_kind, scope_for_user, store};
use aphel_kernel::Ctx;
use aphel_kernel::application::shared::{JobContext, JobHandler};
use aphel_kernel::domain::access::AccessScope;
use aphel_kernel::errors::AppError;
use aphel_kernel::modules::{WorkerEnv, WorkerParts};
use aphel_kernel::state::AppState;

pub(crate) struct AttachSignedDocument {
    state: AppState,
    kind: String,
}

#[async_trait]
impl JobHandler for AttachSignedDocument {
    fn kind(&self) -> &str {
        &self.kind
    }

    async fn handle(&self, _context: &JobContext, payload: Value) -> Result<(), AppError> {
        let Some(request_id) = payload
            .get("request_id")
            .and_then(Value::as_str)
            .and_then(|raw| Uuid::parse_str(raw).ok())
        else {
            return Ok(());
        };
        let system = AccessScope::system();
        let ctx = Ctx::from_state(&self.state, &system).await?;
        let Some(request) = store::find_request(ctx.pool(), request_id).await? else {
            return Ok(());
        };
        let Some(scope) = acting_scope(&ctx, request.created_by).await? else {
            return Ok(());
        };
        service::attach_signed(&ctx.with_scope(&scope), request_id).await
    }
}

pub(crate) fn esign_worker(env: &WorkerEnv) -> WorkerParts {
    WorkerParts::default().handler(Arc::new(AttachSignedDocument {
        state: env.runtime_state(),
        kind: job_kind(ATTACH_JOB),
    }))
}

pub(crate) async fn acting_scope(
    ctx: &Ctx<'_>,
    preferred: Uuid,
) -> Result<Option<AccessScope>, AppError> {
    if let Some(scope) = scope_for_user(ctx, preferred).await? {
        return Ok(Some(scope));
    }
    for user_id in store::recent_senders(ctx.pool()).await? {
        if user_id == preferred {
            continue;
        }
        if let Some(scope) = scope_for_user(ctx, user_id).await? {
            return Ok(Some(scope));
        }
    }
    Ok(None)
}
