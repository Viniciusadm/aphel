use std::sync::Arc;

use serde_json::{Value, json};
use uuid::Uuid;

use crate::registry::LoadedPlugin;
use crate::runtime::error_json;
use crate::store;
use aphel_kernel::Ctx;
use aphel_kernel::application::organization_admin::ListWindow;
use aphel_kernel::application::query::{
    ArchivedFilter, Filter, RecordQuery, RecordQueryEngine, parse_sort,
};
use aphel_kernel::domain::access::AccessScope;
use aphel_kernel::errors::AppError;
use aphel_kernel::records::RecordCtx;
use aphel_kernel::state::runtime_state;

const MAX_PAGE_SIZE: u64 = 100;
const MAX_KV_ITEMS: u32 = 500;

pub async fn dispatch(plugin: &Arc<LoadedPlugin>, scope: &AccessScope, request: Value) -> Value {
    match run(plugin, scope, &request).await {
        Ok(value) => json!({ "ok": value }),
        Err(error) => error_json(&error),
    }
}

fn text<'a>(request: &'a Value, key: &str) -> Result<&'a str, AppError> {
    request
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| AppError::Validation(format!("the host call needs {key}")))
}

fn record_id(request: &Value) -> Result<Uuid, AppError> {
    Uuid::parse_str(text(request, "id")?)
        .map_err(|_| AppError::Validation("the host call needs a valid id".to_owned()))
}

async fn run(
    plugin: &Arc<LoadedPlugin>,
    scope: &AccessScope,
    request: &Value,
) -> Result<Value, AppError> {
    let operation = text(request, "op")?;
    match operation {
        "log" => {
            let message = request
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or_default();
            match request.get("level").and_then(Value::as_str) {
                Some("error") => tracing::error!(plugin = %plugin.key, "{message}"),
                Some("warn") => tracing::warn!(plugin = %plugin.key, "{message}"),
                Some("debug") => tracing::debug!(plugin = %plugin.key, "{message}"),
                _ => tracing::info!(plugin = %plugin.key, "{message}"),
            }
            Ok(Value::Null)
        }
        "now" => Ok(json!(chrono::Utc::now().to_rfc3339())),
        "settings.get" => Ok(Value::Object(plugin.settings.clone())),
        "records.get" | "records.list" | "records.create" | "records.update" | "records.delete" => {
            records(operation, scope, request).await
        }
        "kv.get" | "kv.set" | "kv.delete" | "kv.list" => kv(operation, plugin, request).await,
        other => Err(AppError::Validation(format!("unknown host call {other}"))),
    }
}

async fn records(operation: &str, scope: &AccessScope, request: &Value) -> Result<Value, AppError> {
    let state = runtime_state()?;
    let record_type = text(request, "record_type")?;
    let registry = state.registry().await?;
    let handler = registry.handler(record_type)?;
    let ctx = RecordCtx::new(scope, &state);
    let values = request.get("values").cloned().unwrap_or_else(|| json!({}));
    match operation {
        "records.get" => Ok(handler
            .load(&ctx, record_id(request)?)
            .await?
            .unwrap_or(Value::Null)),
        "records.create" => handler.create(&ctx, values).await,
        "records.update" => handler.update(&ctx, record_id(request)?, values).await,
        "records.delete" => {
            handler.delete(&ctx, record_id(request)?).await?;
            Ok(Value::Null)
        }
        _ => {
            let per_page = request
                .get("per_page")
                .and_then(Value::as_u64)
                .unwrap_or(25)
                .clamp(1, MAX_PAGE_SIZE);
            let page = request
                .get("page")
                .and_then(Value::as_u64)
                .unwrap_or(1)
                .max(1);
            let filter = match request.get("filter") {
                None | Some(Value::Null) => None,
                Some(filter) => Some(serde_json::from_value::<Filter>(filter.clone()).map_err(
                    |_| {
                        AppError::Validation(
                            "the filter must be a filter group or condition".to_owned(),
                        )
                    },
                )?),
            };
            let query = RecordQuery {
                filter,
                sort: request
                    .get("sort")
                    .and_then(Value::as_str)
                    .map(parse_sort)
                    .unwrap_or_default(),
                search: request
                    .get("search")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
                archived: ArchivedFilter::Exclude,
                window: ListWindow {
                    offset: (page - 1) * per_page,
                    limit: per_page,
                },
            };
            let module_ctx = Ctx::from_state(&state, scope).await?;
            let listing = RecordQueryEngine::new(&module_ctx.query())
                .query(scope, record_type, query)
                .await?;
            let mut data = Vec::with_capacity(listing.items.len());
            for id in listing.items {
                if let Some(record) = handler.load(&ctx, id).await? {
                    data.push(record);
                }
            }
            Ok(json!({ "data": data, "total": listing.total }))
        }
    }
}

async fn kv(
    operation: &str,
    plugin: &Arc<LoadedPlugin>,
    request: &Value,
) -> Result<Value, AppError> {
    let pool = runtime_state()?.db()?;
    let collection = text(request, "collection")?;
    if collection.is_empty() || collection.len() > 64 {
        return Err(AppError::Validation(
            "the collection must have 1 to 64 characters".to_owned(),
        ));
    }
    match operation {
        "kv.list" => {
            let items = store::kv_list(&pool, &plugin.key, collection, MAX_KV_ITEMS).await?;
            Ok(Value::Array(
                items
                    .into_iter()
                    .map(|(id, value)| json!({ "id": id, "value": value }))
                    .collect(),
            ))
        }
        _ => {
            let item = text(request, "id")?;
            if item.is_empty() || item.len() > 128 {
                return Err(AppError::Validation(
                    "the id must have 1 to 128 characters".to_owned(),
                ));
            }
            match operation {
                "kv.get" => Ok(store::kv_get(&pool, &plugin.key, collection, item)
                    .await?
                    .unwrap_or(Value::Null)),
                "kv.set" => {
                    let value = request.get("value").cloned().unwrap_or(Value::Null);
                    store::kv_set(&pool, &plugin.key, collection, item, &value).await?;
                    Ok(Value::Null)
                }
                _ => {
                    store::kv_delete(&pool, &plugin.key, collection, item).await?;
                    Ok(Value::Null)
                }
            }
        }
    }
}
