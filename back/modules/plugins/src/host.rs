use serde_json::Value;
use uuid::Uuid;

use crate::runtime::aphel::plugin::host::Host;
use crate::runtime::{CallState, to_plugin_error, wit};
use crate::{net, store};
use aphel_kernel::Ctx;
use aphel_kernel::application::organization_admin::ListWindow;
use aphel_kernel::application::query::{
    ArchivedFilter, Filter, RecordQuery, RecordQueryEngine, parse_sort,
};
use aphel_kernel::application::secrets::SecretKey;
use aphel_kernel::domain::access::AccessScope;
use aphel_kernel::errors::AppError;
use aphel_kernel::records::RecordCtx;
use aphel_kernel::state::runtime_state;

const MAX_PAGE_SIZE: u64 = 100;
const MAX_KV_ITEMS: u32 = 500;
const MAX_COLLECTION_LENGTH: usize = 64;
const MAX_ITEM_LENGTH: usize = 128;

const WRITE_IN_HOOK: &str = "plugin_write_in_hook";
pub const SECRET_OWNER_TYPE: &str = "plugin";

type HostResult<T> = Result<T, wit::PluginError>;

pub fn secret_key(plugin: &str, name: &str) -> SecretKey {
    SecretKey::new(SECRET_OWNER_TYPE, plugin, name)
}

fn outside_transaction(state: &CallState) -> Result<(), AppError> {
    if state.in_transaction {
        return Err(AppError::conflict_code(
            WRITE_IN_HOOK,
            "plugins cannot create, update or delete records inside a record hook; do it when the matching event arrives",
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
    let handler = state.registry().await?.handler(record_type)?;
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
    let handler = state.registry().await?.handler(record_type)?;
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

async fn write(
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

async fn delete(scope: &AccessScope, record_type: &str, id: &str) -> Result<(), AppError> {
    let state = runtime_state()?;
    let handler = state.registry().await?.handler(record_type)?;
    handler
        .delete(&RecordCtx::new(scope, &state), record_id(id)?)
        .await
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
        let plugin = self.plugin.clone();
        answer(net::fetch(&plugin, self.deadline, request).await)
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
