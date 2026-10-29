use serde_json::Value;
use uuid::Uuid;

use crate::registry::LoadedPlugin;
use crate::runtime::aphel::plugin::host::Host;
use crate::runtime::{CallState, to_plugin_error, wit};
use crate::{net, store};
use aphel_kernel::Ctx;
use aphel_kernel::application::access::Authorizer;
use aphel_kernel::application::organization_admin::ListWindow;
use aphel_kernel::application::query::{
    ArchivedFilter, Filter, RecordQuery, RecordQueryEngine, parse_sort,
};
use aphel_kernel::application::secrets::SecretKey;
use aphel_kernel::application::shared::{Actor, DomainEvent, RecordRef};
use aphel_kernel::domain::access::AccessScope;
use aphel_kernel::errors::AppError;
use aphel_kernel::records::RecordCtx;
use aphel_kernel::state::runtime_state;

const MAX_PAGE_SIZE: u64 = 100;
const MAX_KV_ITEMS: u32 = 500;
const MAX_COLLECTION_LENGTH: usize = 64;
const MAX_ITEM_LENGTH: usize = 128;

const WRITE_IN_HOOK: &str = "plugin_write_in_hook";
const MAX_EMIT_PAYLOAD_BYTES: usize = 64 * 1024;
pub const SECRET_OWNER_TYPE: &str = "plugin";

type HostResult<T> = Result<T, wit::PluginError>;

pub fn secret_key(plugin: &str, name: &str) -> SecretKey {
    SecretKey::new(SECRET_OWNER_TYPE, plugin, name)
}

fn outside_transaction(state: &CallState) -> Result<(), AppError> {
    if state.read_only {
        return Err(AppError::forbidden_code(
            "plugin_read_only",
            "this plugin execution cannot produce side effects",
        ));
    }
    if state.in_transaction {
        return Err(AppError::conflict_code(
            WRITE_IN_HOOK,
            "plugins cannot execute side effects inside a record hook; use a durable event handler",
        ));
    }
    if state.retryable {
        return Err(AppError::conflict_code(
            "plugin_effect_required",
            "retryable plugin handlers must use defer-effect for side effects",
        ));
    }
    Ok(())
}

fn parse(json: &str) -> Result<Value, AppError> {
    serde_json::from_str(json)
        .map_err(|_| AppError::Validation("the plugin sent invalid JSON".to_owned()))
}

fn record_id(id: &str) -> Result<Uuid, AppError> {
    Uuid::parse_str(id).map_err(|_| AppError::Validation("the record id is invalid".to_owned()))
}

fn checked<'a>(collection: &'a str, id: Option<&'a str>) -> Result<(), AppError> {
    if collection.is_empty() || collection.len() > MAX_COLLECTION_LENGTH {
        return Err(AppError::Validation(
            "the collection must have 1 to 64 characters".to_owned(),
        ));
    }
    if id.is_some_and(|id| id.is_empty() || id.len() > MAX_ITEM_LENGTH) {
        return Err(AppError::Validation(
            "the id must have 1 to 128 characters".to_owned(),
        ));
    }
    Ok(())
}

fn encode(value: &Value) -> String {
    value.to_string()
}

fn answer<T>(result: Result<T, AppError>) -> HostResult<T> {
    result.map_err(|error| to_plugin_error(&error))
}

async fn load(
    scope: &AccessScope,
    record_type: &str,
    id: &str,
) -> Result<Option<String>, AppError> {
    let state = runtime_state()?;
    let registry = state.registry().await?;
    scope.require(&registry.require(record_type)?.read_permission)?;
    let handler = registry.handler(record_type)?;
    let record = handler
        .load(&RecordCtx::new(scope, &state), record_id(id)?)
        .await?;
    Ok(record.as_ref().map(encode))
}

async fn list(
    scope: &AccessScope,
    record_type: &str,
    query: wit::ListQuery,
) -> Result<wit::Page, AppError> {
    let state = runtime_state()?;
    let registry = state.registry().await?;
    scope.require(&registry.require(record_type)?.read_permission)?;
    let handler = registry.handler(record_type)?;
    let per_page = u64::from(query.per_page.unwrap_or(25)).clamp(1, MAX_PAGE_SIZE);
    let page = u64::from(query.page.unwrap_or(1)).max(1);
    let filter = match query.filter_json.as_deref() {
        None => None,
        Some(filter) => Some(
            serde_json::from_value::<Filter>(parse(filter)?).map_err(|_| {
                AppError::Validation("the filter must be a filter group or condition".to_owned())
            })?,
        ),
    };
    let record_query = RecordQuery {
        filter,
        sort: query.sort.as_deref().map(parse_sort).unwrap_or_default(),
        search: query.search,
        archived: ArchivedFilter::Exclude,
        window: ListWindow {
            offset: (page - 1) * per_page,
            limit: per_page,
        },
    };
    let ctx = Ctx::from_state(&state, scope).await?;
    let listing = RecordQueryEngine::new(&ctx.query())
        .query(scope, record_type, record_query)
        .await?;
    let record_ctx = RecordCtx::new(scope, &state);
    let mut records = Vec::with_capacity(listing.items.len());
    for id in listing.items {
        if let Some(record) = handler.load(&record_ctx, id).await? {
            records.push(encode(&record));
        }
    }
    Ok(wit::Page {
        records_json: records,
        total: listing.total,
    })
}

pub(crate) async fn write(
    scope: &AccessScope,
    record_type: &str,
    id: Option<&str>,
    values_json: &str,
) -> Result<String, AppError> {
    let state = runtime_state()?;
    let handler = state.registry().await?.handler(record_type)?;
    let ctx = RecordCtx::new(scope, &state);
    let values = parse(values_json)?;
    let record = match id {
        None => handler.create(&ctx, values).await?,
        Some(id) => handler.update(&ctx, record_id(id)?, values).await?,
    };
    Ok(encode(&record))
}

pub(crate) async fn delete(
    scope: &AccessScope,
    record_type: &str,
    id: &str,
) -> Result<(), AppError> {
    let state = runtime_state()?;
    let handler = state.registry().await?.handler(record_type)?;
    handler
        .delete(&RecordCtx::new(scope, &state), record_id(id)?)
        .await
}

pub(crate) async fn emit(
    plugin: &LoadedPlugin,
    scope: &AccessScope,
    name: &str,
    record_type: Option<String>,
    record_id: Option<String>,
    payload_json: &str,
) -> Result<(), AppError> {
    let Some(declared) = plugin.manifest.emits.iter().find(|emit| emit.name == name) else {
        return Err(AppError::not_found_code(
            "plugin_event_not_declared",
            format!("the plugin does not declare the event {name} in [[emits]]"),
        ));
    };
    if payload_json.len() > MAX_EMIT_PAYLOAD_BYTES {
        return Err(AppError::Validation(format!(
            "the event payload exceeds {MAX_EMIT_PAYLOAD_BYTES} bytes"
        )));
    }
    let payload = parse(payload_json)?;
    if !payload.is_object() {
        return Err(AppError::Validation(
            "the event payload must be a JSON object".to_owned(),
        ));
    }
    let record = match (record_type, record_id) {
        (None, None) => None,
        (Some(record_type), Some(id)) => {
            if !declared.record_types.is_empty() && !declared.record_types.contains(&record_type) {
                return Err(AppError::Validation(format!(
                    "the event {name} is not declared for {record_type}"
                )));
            }
            if load(scope, &record_type, &id).await?.is_none() {
                return Err(AppError::not_found_code(
                    "record_not_found",
                    "the record does not exist or the plugin cannot read it",
                ));
            }
            Some(RecordRef::parse(&record_type, &id)?)
        }
        _ => {
            return Err(AppError::Validation(
                "record_type and record_id go together".to_owned(),
            ));
        }
    };
    let state = runtime_state()?;
    let mut event = DomainEvent::new(
        Actor::from_scope(scope),
        plugin.manifest.event_type(name),
        chrono::Utc::now(),
    )
    .with_payload(payload);
    if let Some(record) = record {
        event = event.with_record(record);
    }
    let mut tx = aphel_kernel::database::receipt::begin(&state.db()?).await?;
    let mut audit = aphel_kernel::application::shared::AuditEntry::new(
        Actor::from_scope(scope),
        event.event_type.clone(),
        event.occurred_at,
    );
    audit.record = event.record.clone();
    state.events.publish(&mut tx, event).await?;
    state.audit.record(&mut tx, audit).await?;
    tx.commit().await.map_err(AppError::from)?;
    Ok(())
}

impl wit::Host for CallState {}

impl Host for CallState {
    async fn get_record(&mut self, record_type: String, id: String) -> HostResult<Option<String>> {
        let scope = self.scope.clone();
        answer(load(&scope, &record_type, &id).await)
    }

    async fn list_records(
        &mut self,
        record_type: String,
        query: wit::ListQuery,
    ) -> HostResult<wit::Page> {
        let scope = self.scope.clone();
        answer(list(&scope, &record_type, query).await)
    }

    async fn create_record(
        &mut self,
        record_type: String,
        values_json: String,
    ) -> HostResult<String> {
        if let Err(error) = outside_transaction(self) {
            return answer(Err(error));
        }
        let scope = self.scope.clone();
        answer(write(&scope, &record_type, None, &values_json).await)
    }

    async fn update_record(
        &mut self,
        record_type: String,
        id: String,
        values_json: String,
    ) -> HostResult<String> {
        if let Err(error) = outside_transaction(self) {
            return answer(Err(error));
        }
        let scope = self.scope.clone();
        answer(write(&scope, &record_type, Some(&id), &values_json).await)
    }

    async fn delete_record(&mut self, record_type: String, id: String) -> HostResult<()> {
        if let Err(error) = outside_transaction(self) {
            return answer(Err(error));
        }
        let scope = self.scope.clone();
        answer(delete(&scope, &record_type, &id).await)
    }

    async fn kv_get(&mut self, collection: String, id: String) -> HostResult<Option<String>> {
        let plugin = self.plugin.key.clone();
        answer(
            async {
                checked(&collection, Some(&id))?;
                let pool = runtime_state()?.db()?;
                Ok(store::kv_get(&pool, &plugin, &collection, &id)
                    .await?
                    .as_ref()
                    .map(encode))
            }
            .await,
        )
    }

    async fn kv_set(
        &mut self,
        collection: String,
        id: String,
        value_json: String,
    ) -> HostResult<()> {
        if let Err(error) = outside_transaction(self) {
            return answer(Err(error));
        }
        let plugin = self.plugin.key.clone();
        answer(
            async {
                checked(&collection, Some(&id))?;
                let value = parse(&value_json)?;
                let pool = runtime_state()?.db()?;
                store::kv_set(&pool, &plugin, &collection, &id, &value).await
            }
            .await,
        )
    }

    async fn kv_delete(&mut self, collection: String, id: String) -> HostResult<()> {
        if let Err(error) = outside_transaction(self) {
            return answer(Err(error));
        }
        let plugin = self.plugin.key.clone();
        answer(
            async {
                checked(&collection, Some(&id))?;
                let pool = runtime_state()?.db()?;
                store::kv_delete(&pool, &plugin, &collection, &id).await
            }
            .await,
        )
    }

    async fn kv_list(&mut self, collection: String) -> HostResult<Vec<(String, String)>> {
        let plugin = self.plugin.key.clone();
        answer(
            async {
                checked(&collection, None)?;
                let pool = runtime_state()?.db()?;
                Ok(store::kv_list(&pool, &plugin, &collection, MAX_KV_ITEMS)
                    .await?
                    .into_iter()
                    .map(|(id, value)| (id, encode(&value)))
                    .collect())
            }
            .await,
        )
    }

    async fn http_fetch(&mut self, request: wit::HttpRequest) -> HostResult<wit::HttpResponse> {
        if let Err(error) = outside_transaction(self) {
            return answer(Err(error));
        }
        let plugin = self.plugin.clone();
        answer(net::fetch(&plugin, self.deadline, request).await)
    }

    async fn notify(&mut self, notification: wit::Notification) -> HostResult<()> {
        if let Err(error) = outside_transaction(self) {
            return answer(Err(error));
        }
        let plugin = self.plugin.clone();
        let scope = self.scope.clone();
        answer(crate::notify::notify(&plugin, &scope, notification).await)
    }

    async fn emit(
        &mut self,
        name: String,
        record_type: Option<String>,
        record_id: Option<String>,
        payload_json: String,
    ) -> HostResult<()> {
        if let Err(error) = outside_transaction(self) {
            return answer(Err(error));
        }
        let plugin = self.plugin.clone();
        let scope = self.scope.clone();
        answer(
            emit(
                &plugin,
                &scope,
                &name,
                record_type,
                record_id,
                &payload_json,
            )
            .await,
        )
    }

    async fn invoke(&mut self, operation: String, input_json: String) -> HostResult<String> {
        if self.retryable
            && aphel_kernel::modules::app()
                .contributions::<aphel_kernel::modules::PluginOperation>()
                .iter()
                .any(|candidate| {
                    candidate.key == operation && candidate.writes && !candidate.external_query
                })
        {
            return answer(Err(AppError::conflict_code(
                "plugin_effect_required",
                "retryable plugin handlers must defer writing operations",
            )));
        }
        let plugin = self.plugin.clone();
        let scope = self.scope.clone();
        answer(
            crate::bridge::invoke(
                &plugin,
                &scope,
                self.in_transaction || self.read_only,
                self.deadline,
                &operation,
                &input_json,
            )
            .await,
        )
    }

    async fn secret(&mut self, key: String) -> HostResult<Option<String>> {
        let plugin = self.plugin.clone();
        answer(
            async {
                if !plugin
                    .manifest
                    .secret_settings()
                    .any(|setting| setting.key == key)
                {
                    return Err(AppError::not_found_code(
                        "plugin_secret_not_declared",
                        format!("the plugin does not declare a secret setting {key}"),
                    ));
                }
                let state = runtime_state()?;
                Ok(state
                    .secrets
                    .reveal(&AccessScope::system(), &secret_key(&plugin.key, &key))
                    .await?
                    .map(|secret| secret.expose().to_owned()))
            }
            .await,
        )
    }

    async fn defer_effect(&mut self, key: String, command_json: String) -> HostResult<String> {
        answer(crate::effects::defer(self, key, command_json).await)
    }

    async fn hook_state_json(&mut self) -> String {
        self.hook_state
            .as_ref()
            .map(ToString::to_string)
            .unwrap_or_else(|| "null".to_owned())
    }

    async fn execution_context_json(&mut self) -> String {
        serde_json::to_string(&aphel_kernel::application::shared::ExecutionContext::current())
            .unwrap_or_else(|_| "null".to_owned())
    }

    async fn settings_json(&mut self) -> String {
        Value::Object(self.plugin.settings.clone()).to_string()
    }

    async fn now(&mut self) -> String {
        chrono::Utc::now().to_rfc3339()
    }

    async fn log(&mut self, level: String, message: String) {
        let plugin = &self.plugin.key;
        match level.as_str() {
            "error" => tracing::error!(plugin = %plugin, "{message}"),
            "warn" => tracing::warn!(plugin = %plugin, "{message}"),
            "debug" => tracing::debug!(plugin = %plugin, "{message}"),
            _ => tracing::info!(plugin = %plugin, "{message}"),
        }
    }
}
