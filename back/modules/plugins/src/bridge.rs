use std::sync::Arc;
use std::time::Instant;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::manifest::RECORD_HOOKS;
use crate::registry::{self, LoadedPlugin};
use crate::runtime::{on_hook, scope_actor, wit};
use aphel_kernel::Ctx;
use aphel_kernel::domain::access::AccessScope;
use aphel_kernel::errors::AppError;
use aphel_kernel::modules::{
    HookCtx, PluginHook, PluginHookCall, PluginHookDispatcher, PluginOperation, app,
    operation_input,
};
use aphel_kernel::records::RecordCtx;
use aphel_kernel::state::runtime_state;

const OPERATION_UNKNOWN: &str = "plugin_operation_unknown";
const OPERATION_NOT_DECLARED: &str = "plugin_operation_not_declared";
const OPERATION_TIMEOUT: &str = "plugin_operation_timeout";
const WRITE_IN_HOOK: &str = "plugin_write_in_hook";

pub struct HostCatalogData {
    pub hooks: Vec<&'static str>,
    pub operations: Vec<&'static str>,
}

pub fn host_catalog() -> HostCatalogData {
    let app = app();
    let mut hooks: Vec<&'static str> = RECORD_HOOKS.to_vec();
    hooks.extend(
        app.all_contributions::<PluginHook>()
            .into_iter()
            .map(|(_, hook)| hook.key),
    );
    let operations = app
        .all_contributions::<PluginOperation>()
        .into_iter()
        .map(|(_, operation)| operation.key)
        .collect();
    HostCatalogData { hooks, operations }
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct HookCatalogEntry {
    pub key: &'static str,
    pub module: &'static str,
    pub record_type: &'static str,
    pub kind: &'static str,
    pub description: &'static str,
    pub enabled: bool,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct HostOperationResponse {
    pub key: &'static str,
    pub module: &'static str,
    pub description: &'static str,
    pub writes: bool,
    pub durable: bool,
    pub external_query: bool,
    pub enabled: bool,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct HostCatalogResponse {
    pub hooks: Vec<HookCatalogEntry>,
    pub operations: Vec<HostOperationResponse>,
}

pub fn host_catalog_response() -> HostCatalogResponse {
    let app = app();
    let mut hooks: Vec<HookCatalogEntry> = RECORD_HOOKS
        .iter()
        .map(|key| HookCatalogEntry {
            key,
            module: crate::KEY,
            record_type: "*",
            kind: record_hook_kind(key),
            description: "Generic record pipeline hook",
            enabled: true,
        })
        .collect();
    hooks.extend(
        app.all_contributions::<PluginHook>()
            .into_iter()
            .map(|(module, hook)| HookCatalogEntry {
                key: hook.key,
                module,
                record_type: hook.record_type,
                kind: hook.kind.as_str(),
                description: hook.description,
                enabled: app.is_enabled(module),
            }),
    );
    let mut operations: Vec<HostOperationResponse> = app
        .all_contributions::<PluginOperation>()
        .into_iter()
        .map(|(module, operation)| HostOperationResponse {
            key: operation.key,
            module,
            description: operation.description,
            writes: operation.writes,
            durable: operation.durable,
            external_query: operation.external_query,
            enabled: app.is_enabled(module),
        })
        .collect();
    operations.sort_by(|left, right| left.key.cmp(right.key));
    HostCatalogResponse { hooks, operations }
}

fn record_hook_kind(key: &str) -> &'static str {
    match key {
        "record.saving" => "filter",
        "record.validating" | "record.deleting" | "record.archiving" => "veto",
        _ => "action",
    }
}

pub(crate) async fn invoke(
    plugin: &LoadedPlugin,
    scope: &AccessScope,
    in_transaction: bool,
    deadline: Instant,
    operation: &str,
    input_json: &str,
) -> Result<String, AppError> {
    if !plugin.manifest.uses_operation(operation) {
        return Err(AppError::forbidden_code(
            OPERATION_NOT_DECLARED,
            format!("the plugin does not declare the operation {operation} in `operations`"),
        ));
    }
    let Some(found) = app()
        .contributions::<PluginOperation>()
        .into_iter()
        .find(|candidate| candidate.key == operation)
    else {
        return Err(AppError::not_found_code(
            OPERATION_UNKNOWN,
            format!("the operation {operation} does not exist or its module is disabled"),
        ));
    };
    if found.writes && in_transaction {
        return Err(AppError::conflict_code(
            WRITE_IN_HOOK,
            "plugins cannot call writing operations inside a hook; do it when the matching event arrives",
        ));
    }
    let input: Value = if input_json.trim().is_empty() {
        Value::Object(Default::default())
    } else {
        serde_json::from_str(input_json)
            .map_err(|_| AppError::Validation("the plugin sent invalid JSON".to_owned()))?
    };
    let state = runtime_state()?;
    let ctx = Ctx::from_state(&state, scope).await?;
    let remaining = deadline.saturating_duration_since(Instant::now());
    if found.external_query {
        return external_query(plugin, &ctx, &found, input, remaining).await;
    }
    let output = tokio::time::timeout(remaining, found.invoke(&ctx, input))
        .await
        .map_err(|_| {
            AppError::unavailable_code(
                OPERATION_TIMEOUT,
                format!("the operation {operation} did not finish within the plugin call time"),
            )
        })??;
    Ok(output.to_string())
}

pub struct Dispatcher;

#[async_trait]
impl PluginHookDispatcher for Dispatcher {
    fn subscribed(&self, hook: &str, record_type: &str) -> bool {
        let snapshot = registry::cached();
        snapshot.version.is_none() || !snapshot.hook_subscribers(hook, record_type).is_empty()
    }

    async fn dispatch(
        &self,
        ctx: &mut HookCtx<'_>,
        call: PluginHookCall<'_>,
    ) -> Result<Vec<Value>, AppError> {
        let snapshot = registry::current(ctx.pool).await?;
        let mut call = wit::HookCall {
            hook: call.hook.to_owned(),
            record_type: call.record_type.to_owned(),
            record_id: call.record_id,
            mode: None,
            actor: scope_actor(ctx.scope),
            changes: Vec::new(),
            values_json: call.values.to_string(),
            custom_fields_json: "{}".to_owned(),
        };
        let mut outcomes = Vec::new();
        for plugin in snapshot.hook_subscribers(&call.hook, &call.record_type) {
            let outcome = on_hook(&plugin, ctx, &call).await?;
            if let Ok(value @ Value::Object(_)) =
                serde_json::from_str::<Value>(&outcome.values_json)
                && value.as_object().is_some_and(|object| !object.is_empty())
            {
                call.values_json = merged(&call.values_json, &value);
                outcomes.push(value);
            }
        }
        Ok(outcomes)
    }
}

fn merged(current: &str, patch: &Value) -> String {
    let mut base = serde_json::from_str::<Value>(current).unwrap_or_else(|_| json!({}));
    if let (Some(base), Some(patch)) = (base.as_object_mut(), patch.as_object()) {
        for (key, value) in patch {
            base.insert(key.clone(), value.clone());
        }
    }
    base.to_string()
}

#[derive(Deserialize)]
struct RecordTarget {
    record_type: String,
    id: Uuid,
}

fn archive<'a>(ctx: &'a Ctx<'_>, input: Value) -> aphel_kernel::modules::OperationFuture<'a> {
    Box::pin(async move {
        let target: RecordTarget = operation_input(input)?;
        let state = runtime_state()?;
        let handler = ctx.registry().handler(&target.record_type)?;
        handler
            .archive(&RecordCtx::new(ctx.scope, &state), target.id)
            .await?;
        Ok(json!({ "archived": true }))
    })
}

fn restore<'a>(ctx: &'a Ctx<'_>, input: Value) -> aphel_kernel::modules::OperationFuture<'a> {
    Box::pin(async move {
        let target: RecordTarget = operation_input(input)?;
        let state = runtime_state()?;
        let handler = ctx.registry().handler(&target.record_type)?;
        handler
            .restore(&RecordCtx::new(ctx.scope, &state), target.id)
            .await?;
        Ok(json!({ "restored": true }))
    })
}

pub fn record_operations() -> Vec<Arc<PluginOperation>> {
    vec![
        Arc::new(
            PluginOperation::write(
                "records.archive",
                "Archive a record (hides it from default lists; not a deletion): { record_type, id }. Supported by contact, company, deal and custom object (co_*) records; other types fail with 422 record_operation_not_supported",
                archive,
            )
            .with_durable_mutations(),
        ),
        Arc::new(
            PluginOperation::write(
                "records.restore",
                "Unarchive an archived record (not a trash restore): { record_type, id }. Supported by contact, company, deal and custom object (co_*) records; other types fail with 422 record_operation_not_supported",
                restore,
            )
            .with_durable_mutations(),
        ),
    ]
}

async fn external_query(
    plugin: &LoadedPlugin,
    ctx: &Ctx<'_>,
    operation: &PluginOperation,
    input: Value,
    remaining: std::time::Duration,
) -> Result<String, AppError> {
    use aphel_kernel::application::shared::ExecutionContext;
    use sqlx::{Row, types::Json};
    let execution = ExecutionContext::current().ok_or_else(|| {
        AppError::Internal("external plugin queries require an execution context".to_owned())
    })?;
    let hash = aphel_kernel::application::files::sha256_hex(input.to_string().as_bytes());
    let authority_hash =
        aphel_kernel::application::files::sha256_hex(format!("{:?}", ctx.scope).as_bytes());
    let execution_id = execution.id.to_string();
    let inserted = sqlx::query("INSERT IGNORE INTO plugins_external_queries (plugin_key, execution_id, operation, input_hash, authority_hash) VALUES (?, ?, ?, ?, ?)")
        .bind(&plugin.key).bind(&execution_id).bind(operation.key).bind(&hash).bind(&authority_hash).execute(ctx.pool()).await?.rows_affected();
    if inserted == 0 {
        let row = sqlx::query("SELECT status, output, authority_hash FROM plugins_external_queries WHERE plugin_key = ? AND execution_id = ? AND operation = ? AND input_hash = ?")
            .bind(&plugin.key).bind(&execution_id).bind(operation.key).bind(&hash).fetch_one(ctx.pool()).await?;
        if row.try_get::<String, _>("authority_hash")? != authority_hash {
            return Err(AppError::forbidden_code(
                "plugin_query_authority_changed",
                "the authority of the cached external query has changed",
            ));
        }
        if row.try_get::<String, _>("status")? == "completed" {
            let Json(output): Json<Value> = row.try_get("output")?;
            return Ok(output.to_string());
        }
        return Err(AppError::conflict_code(
            "plugin_query_outcome_unknown",
            "the external query already started and has no completed result; it will not be called again automatically",
        ));
    }
    let result = tokio::time::timeout(remaining, operation.invoke(ctx, input))
        .await
        .map_err(|_| {
            AppError::unavailable_code(
                OPERATION_TIMEOUT,
                "the external query exceeded the plugin call time",
            )
        })?;
    match result {
        Ok(output) => {
            sqlx::query("UPDATE plugins_external_queries SET status = 'completed', output = ?, completed_at = UTC_TIMESTAMP(6) WHERE plugin_key = ? AND execution_id = ? AND operation = ? AND input_hash = ?")
                .bind(Json(&output)).bind(&plugin.key).bind(&execution_id).bind(operation.key).bind(&hash).execute(ctx.pool()).await?;
            Ok(output.to_string())
        }
        Err(error) => {
            sqlx::query("UPDATE plugins_external_queries SET status = 'failed', completed_at = UTC_TIMESTAMP(6) WHERE plugin_key = ? AND execution_id = ? AND operation = ? AND input_hash = ?")
                .bind(&plugin.key).bind(&execution_id).bind(operation.key).bind(&hash).execute(ctx.pool()).await?;
            Err(error)
        }
    }
}
