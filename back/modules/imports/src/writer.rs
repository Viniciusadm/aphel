use std::sync::Arc;

use serde_json::{Map, Value};
use uuid::Uuid;

use crate::is_importable;
use aphel_kernel::application::shared::{EventOrigin, OriginEventPublisher};
use aphel_kernel::context::Ctx;
use aphel_kernel::errors::AppError;
use aphel_kernel::records::{RecordCtx, RecordTypeHandler};
use aphel_kernel::state::AppState;

pub(crate) const IMPORT_EVENT_ORIGIN: &str = "import";

#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct RecordValues {
    pub(crate) system: Map<String, Value>,
    pub(crate) custom: Map<String, Value>,
    pub(crate) relations: Map<String, Value>,
}

impl RecordValues {
    pub(crate) fn text(&self, key: &str) -> Option<&str> {
        self.system
            .get(key)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|text| !text.is_empty())
    }
}

pub(crate) fn import_body(values: &RecordValues) -> Value {
    let mut body = values.system.clone();
    if !values.custom.is_empty() {
        body.insert(
            "custom_fields".to_owned(),
            Value::Object(values.custom.clone()),
        );
    }
    if !values.relations.is_empty() {
        body.insert(
            "relations".to_owned(),
            Value::Object(values.relations.clone()),
        );
    }
    Value::Object(body)
}

fn created_id(record: &Value) -> Result<Uuid, AppError> {
    record
        .get("id")
        .and_then(Value::as_str)
        .and_then(|id| Uuid::parse_str(id).ok())
        .ok_or_else(|| AppError::Internal("the saved record has no id".to_owned()))
}

fn unsupported(record_type: &str) -> AppError {
    AppError::Validation(format!("{record_type} records cannot be imported"))
}

fn import_handler(
    ctx: &Ctx<'_>,
    record_type: &str,
) -> Result<Arc<dyn RecordTypeHandler>, AppError> {
    let registry = ctx.registry();
    if !is_importable(ctx, record_type) || registry.require(record_type).is_err() {
        return Err(unsupported(record_type));
    }
    registry.handler(record_type)
}

fn import_state(ctx: &Ctx<'_>, import_id: Uuid) -> AppState {
    let state = ctx.state();
    let events = state.events.clone();
    state.with_events(Arc::new(OriginEventPublisher::new(
        events,
        EventOrigin::new(IMPORT_EVENT_ORIGIN, Some(import_id.to_string())),
    )))
}

pub(crate) async fn create_record(
    ctx: &Ctx<'_>,
    import_id: Uuid,
    record_type: &str,
    values: &RecordValues,
) -> Result<Uuid, AppError> {
    let handler = import_handler(ctx, record_type)?;
    let state = import_state(ctx, import_id);
    let record = handler
        .create(&RecordCtx::new(ctx.scope, &state), import_body(values))
        .await?;
    created_id(&record)
}

pub(crate) async fn update_record(
    ctx: &Ctx<'_>,
    import_id: Uuid,
    record_type: &str,
    record_id: Uuid,
    values: &RecordValues,
) -> Result<Uuid, AppError> {
    let handler = import_handler(ctx, record_type)?;
    let state = import_state(ctx, import_id);
    let body = import_body(values);
    handler
        .update(&RecordCtx::new(ctx.scope, &state), record_id, body)
        .await?;
    Ok(record_id)
}
