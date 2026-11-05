use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use serde_json::{Map, Value};
use uuid::Uuid;

use aphel_kernel::context::Ctx;
use aphel_kernel::errors::{AppError, CodedKind};
use aphel_kernel::records::RecordCtx;
use aphel_sdk::platform::api::rate_limit::{SlidingWindowLimiter, shared_hit};
use aphel_sdk::platform::application::fields::FieldSet;
use aphel_sdk::platform::domain::access::AccessScope;

pub(crate) fn actor_id(scope: &AccessScope) -> Uuid {
    if scope.is_system() {
        return Uuid::nil();
    }
    Uuid::parse_str(scope.user_id().as_str()).unwrap_or_else(|_| Uuid::nil())
}

pub(crate) async fn allow(
    bucket: &str,
    key: &str,
    limit: u32,
    window: Duration,
) -> Result<bool, AppError> {
    static LIMITERS: OnceLock<Mutex<HashMap<String, Arc<SlidingWindowLimiter>>>> = OnceLock::new();
    let limiter = {
        let mut limiters = LIMITERS
            .get_or_init(|| Mutex::new(HashMap::new()))
            .lock()
            .map_err(|_| AppError::Internal("rate limiter lock poisoned".to_owned()))?;
        limiters
            .entry(format!("{bucket}:{limit}:{}", window.as_secs()))
            .or_insert_with(|| Arc::new(SlidingWindowLimiter::new(limit, window)))
            .clone()
    };
    Ok(
        shared_hit(bucket, &format!("{bucket}:{key}"), limit, window, &limiter)
            .await?
            .allowed,
    )
}

fn is_missing(error: &AppError) -> bool {
    matches!(
        error,
        AppError::NotFound(_)
            | AppError::Forbidden(_)
            | AppError::Coded {
                kind: CodedKind::NotFound | CodedKind::Forbidden,
                ..
            }
    )
}

pub(crate) async fn load_record(
    ctx: &Ctx<'_>,
    record_type: &str,
    id: Uuid,
) -> Result<Option<Value>, AppError> {
    let Ok(handler) = ctx.registry().handler(record_type) else {
        return Ok(None);
    };
    let state = ctx.state();
    match handler.load(&RecordCtx::new(ctx.scope, &state), id).await {
        Ok(value) => Ok(value),
        Err(error) if is_missing(&error) => Ok(None),
        Err(error) => Err(error),
    }
}

pub(crate) async fn fields(ctx: &Ctx<'_>, record_type: &str) -> Result<FieldSet, AppError> {
    ctx.field_set(ctx.scope, record_type).await
}

pub(crate) async fn update_record_in_tx(
    ctx: &Ctx<'_>,
    tx: &mut aphel_sdk::platform::database::transaction::MySqlTx,
    record_type: &str,
    id: Uuid,
    values: Map<String, Value>,
) -> Result<Option<aphel_kernel::records::PendingCommit>, AppError> {
    let fields = fields(ctx, record_type).await?;
    let mut body = Map::new();
    let mut custom = Map::new();
    for (key, value) in values {
        match fields.find(&key) {
            Some(field) if field.is_system() => {
                body.insert(key, value);
            }
            Some(_) => {
                custom.insert(key, value);
            }
            None => {
                return Err(AppError::Validation(format!(
                    "{key} is not a field of {record_type} records"
                )));
            }
        }
    }
    if !custom.is_empty() {
        body.insert("custom_fields".to_owned(), Value::Object(custom));
    }
    let handler = ctx.registry().handler(record_type)?;
    let state = ctx.state();
    handler
        .update_in_tx(
            &RecordCtx::new(ctx.scope, &state),
            tx,
            id,
            Value::Object(body),
        )
        .await
}
