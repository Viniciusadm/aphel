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
const SEQUENCE_COLLECTION: &str = "_sequences";
const MAX_RAW_BODY_BYTES: usize = 10 * 1024 * 1024;
const RAW_CONTENT_TYPES: &[&str] = &[
    "text/csv",
    "text/plain",
    "text/calendar",
    "application/pdf",
    "application/json",
    "application/zip",
    "application/octet-stream",
    "image/png",
    "image/jpeg",
];

const WRITE_IN_HOOK: &str = "plugin_write_in_hook";
const MAX_BATCH_MUTATIONS: usize = 100;
const MAX_BATCH_BYTES: usize = 1024 * 1024;
const BATCH_INVALID: &str = "plugin_batch_invalid";
const BATCH_REF: &str = "$ref";
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

async fn aggregate(scope: &AccessScope, query_json: &str) -> Result<String, AppError> {
    let request: aphel_kernel::api::dto::aggregates::AggregateQueryRequest =
        serde_json::from_str(query_json)
            .map_err(|error| AppError::Validation(format!("invalid aggregate query: {error}")))?;
    let state = runtime_state()?;
    let ports = aphel_kernel::api::routes::v1::aggregates::aggregate_ports(&state).await?;
    let result = aphel_kernel::application::query::AggregateEngine::new(&ports)
        .run(scope, request.into_query())
        .await?;
    let response: aphel_kernel::api::dto::aggregates::AggregateResultResponse = result.into();
    serde_json::to_string(&response).map_err(|error| AppError::Internal(error.to_string()))
}

const ITERATE_CURSOR_PREFIX: &str = "o:";

async fn iterate(
    scope: &AccessScope,
    record_type: &str,
    mut query: wit::ListQuery,
    cursor: Option<String>,
) -> Result<(Vec<String>, Option<String>), AppError> {
    let offset: u64 = match cursor.as_deref() {
        None => 0,
        Some(cursor) => cursor
            .strip_prefix(ITERATE_CURSOR_PREFIX)
            .and_then(|offset| offset.parse().ok())
            .ok_or_else(|| AppError::Validation("the cursor is invalid".to_owned()))?,
    };
    let per_page = u64::from(query.per_page.unwrap_or(100)).clamp(1, MAX_PAGE_SIZE);
    if query.sort.is_none() {
        query.sort = Some("created_at".to_owned());
    }
    let state = runtime_state()?;
    let registry = state.registry().await?;
    scope.require(&registry.require(record_type)?.read_permission)?;
    let handler = registry.handler(record_type)?;
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
            offset,
            limit: per_page,
        },
    };
    let ctx = Ctx::from_state(&state, scope).await?;
    let listing = RecordQueryEngine::new(&ctx.query())
        .query(scope, record_type, record_query)
        .await?;
    let fetched = listing.items.len() as u64;
    let record_ctx = RecordCtx::new(scope, &state);
    let mut records = Vec::with_capacity(listing.items.len());
    for id in listing.items {
        if let Some(record) = handler.load(&record_ctx, id).await? {
            records.push(encode(&record));
        }
    }
    let next = (fetched == per_page && offset + fetched < listing.total)
        .then(|| format!("{ITERATE_CURSOR_PREFIX}{}", offset + fetched));
    Ok((records, next))
}

fn parse_version(value: &str) -> Result<chrono::DateTime<chrono::Utc>, AppError> {
    chrono::DateTime::parse_from_rfc3339(value.trim())
        .map(|value| value.with_timezone(&chrono::Utc))
        .map_err(|_| {
            AppError::Validation("expected-updated-at must be an RFC 3339 timestamp".to_owned())
        })
}

async fn loaded(
    scope: &AccessScope,
    record_type: &str,
    id: uuid::Uuid,
) -> Result<String, AppError> {
    load(scope, record_type, &id.to_string())
        .await?
        .ok_or_else(|| {
            AppError::not_found_code(
                "record_not_found",
                "the record does not exist or the plugin cannot read it",
            )
        })
}

pub(crate) async fn update_if(
    scope: &AccessScope,
    record_type: &str,
    id: &str,
    expected_updated_at: &str,
    values_json: &str,
) -> Result<String, AppError> {
    let state = runtime_state()?;
    let registry = state.registry().await?;
    let table = registry.require(record_type)?.table.clone();
    let handler = registry.handler(record_type)?;
    let ctx = RecordCtx::new(scope, &state);
    let id = record_id(id)?;
    let expected = parse_version(expected_updated_at)?;
    let values = parse(values_json)?;
    let mut tx = aphel_kernel::database::receipt::begin(&state.db()?).await?;
    aphel_kernel::records::lock_version(&mut tx, &table, id, Some(expected)).await?;
    let pending = handler.update_in_tx(&ctx, &mut tx, id, values).await?;
    tx.commit().await.map_err(AppError::from)?;
    if let Some(pending) = pending {
        pending
            .committed(&Ctx::from_state(&state, scope).await?)
            .await;
    }
    loaded(scope, record_type, id).await
}

fn resolve_refs(value: &mut Value, ids: &[uuid::Uuid]) -> Result<(), String> {
    match value {
        Value::Object(map) if map.len() == 1 && map.contains_key(BATCH_REF) => {
            let index = map
                .get(BATCH_REF)
                .and_then(Value::as_u64)
                .ok_or_else(|| "$ref must be the index of an earlier mutation".to_owned())?;
            let id = usize::try_from(index)
                .ok()
                .and_then(|index| ids.get(index))
                .ok_or_else(|| format!("$ref {index} does not point to an earlier mutation"))?;
            *value = Value::String(id.to_string());
            Ok(())
        }
        Value::Object(map) => map
            .values_mut()
            .try_for_each(|nested| resolve_refs(nested, ids)),
        Value::Array(items) => items
            .iter_mut()
            .try_for_each(|nested| resolve_refs(nested, ids)),
        _ => Ok(()),
    }
}

fn at_index(index: usize, error: AppError) -> AppError {
    let prefix = format!("mutations[{index}]");
    match error {
        AppError::InvalidFields { message, fields } => AppError::invalid_fields_code(
            BATCH_INVALID,
            format!("{prefix}: {message}"),
            fields
                .into_iter()
                .map(|mut field| {
                    field.field = format!("{prefix}.{}", field.field);
                    field
                })
                .collect(),
        ),
        AppError::CodedFields {
            code,
            message,
            fields,
        } => AppError::invalid_fields_code(
            code,
            format!("{prefix}: {message}"),
            fields
                .into_iter()
                .map(|mut field| {
                    field.field = format!("{prefix}.{}", field.field);
                    field
                })
                .collect(),
        ),
        AppError::Coded {
            kind,
            code,
            message,
        } => AppError::Coded {
            kind,
            code,
            message: format!("{prefix}: {message}"),
        },
        AppError::CodedDetails {
            kind,
            code,
            message,
            details,
        } => AppError::CodedDetails {
            kind,
            code,
            message: format!("{prefix}: {message}"),
            details,
        },
        AppError::NotFound(message) => AppError::NotFound(format!("{prefix}: {message}")),
        AppError::Forbidden(message) => AppError::Forbidden(format!("{prefix}: {message}")),
        AppError::Conflict(message) => AppError::Conflict(format!("{prefix}: {message}")),
        AppError::Validation(message) => AppError::Validation(format!("{prefix}: {message}")),
        other => other,
    }
}

pub(crate) async fn apply(
    scope: &AccessScope,
    mutations: Vec<wit::RecordMutation>,
) -> Result<Vec<String>, AppError> {
    if mutations.is_empty() || mutations.len() > MAX_BATCH_MUTATIONS {
        return Err(AppError::validation_code(
            BATCH_INVALID,
            format!("a batch has 1 to {MAX_BATCH_MUTATIONS} mutations"),
        ));
    }
    let size: usize = mutations
        .iter()
        .map(|mutation| mutation.values_json.len())
        .sum();
    if size > MAX_BATCH_BYTES {
        return Err(AppError::validation_code(
            BATCH_INVALID,
            format!("the values of a batch exceed {MAX_BATCH_BYTES} bytes"),
        ));
    }
    let state = runtime_state()?;
    let registry = state.registry().await?;
    let ctx = RecordCtx::new(scope, &state);
    let mut tx = aphel_kernel::database::receipt::begin(&state.db()?).await?;
    let mut ids: Vec<uuid::Uuid> = Vec::with_capacity(mutations.len());
    let mut pending = Vec::with_capacity(mutations.len());
    let mut types = Vec::with_capacity(mutations.len());
    for (index, mutation) in mutations.into_iter().enumerate() {
        let step = async {
            let definition = registry.require(&mutation.record_type)?;
            let handler = registry.handler(&mutation.record_type)?;
            let mut values = parse(&mutation.values_json)?;
            resolve_refs(&mut values, &ids).map_err(AppError::Validation)?;
            match (&mutation.id, &mutation.expected_updated_at) {
                (None, Some(_)) => Err(AppError::Validation(
                    "expected-updated-at only applies to updates".to_owned(),
                )),
                (None, None) => {
                    let commit = handler.create_in_tx(&ctx, &mut tx, values).await?;
                    Ok((commit.record.record_id, Some(commit)))
                }
                (Some(id), expected) => {
                    let id = record_id(id)?;
                    if let Some(expected) = expected {
                        aphel_kernel::records::lock_version(
                            &mut tx,
                            &definition.table,
                            id,
                            Some(parse_version(expected)?),
                        )
                        .await?;
                    }
                    Ok((id, handler.update_in_tx(&ctx, &mut tx, id, values).await?))
                }
            }
        };
        let (id, commit) = step.await.map_err(|error| at_index(index, error))?;
        ids.push(id);
        types.push(mutation.record_type);
        pending.extend(commit);
    }
    tx.commit().await.map_err(AppError::from)?;
    let after = Ctx::from_state(&state, scope).await?;
    for commit in &pending {
        commit.committed(&after).await;
    }
    let mut records = Vec::with_capacity(ids.len());
    for (record_type, id) in types.iter().zip(&ids) {
        records.push(loaded(scope, record_type, *id).await?);
    }
    Ok(records)
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

fn raw_body(
    state: &mut CallState,
    content_type: String,
    filename: Option<String>,
    body: Vec<u8>,
) -> Result<(), AppError> {
    if !state.route_call {
        return Err(AppError::conflict_code(
            "plugin_raw_body_outside_route",
            "respond-bytes is only available while handling a route",
        ));
    }
    let content_type = content_type.trim().to_ascii_lowercase();
    let base = content_type.split(';').next().unwrap_or_default().trim();
    if !RAW_CONTENT_TYPES.contains(&base) {
        return Err(AppError::validation_code(
            "plugin_raw_body_invalid",
            format!(
                "the content type must be one of {}",
                RAW_CONTENT_TYPES.join(", ")
            ),
        ));
    }
    if body.len() > MAX_RAW_BODY_BYTES {
        return Err(AppError::payload_too_large_code(
            "plugin_raw_body_too_large",
            format!("a route body is limited to {MAX_RAW_BODY_BYTES} bytes"),
        ));
    }
    let filename = filename
        .map(|name| {
            name.chars()
                .filter(|character| {
                    character.is_ascii_alphanumeric() || matches!(character, '.' | '-' | '_')
                })
                .take(120)
                .collect::<String>()
        })
        .filter(|name| !name.is_empty());
    state.raw_body = Some(crate::runtime::RawBody {
        content_type,
        filename,
        bytes: body,
    });
    Ok(())
}

async fn organization() -> Result<String, AppError> {
    use aphel_kernel::application::organizations::OrganizationRepository;
    let pool = runtime_state()?.db()?;
    let organization =
        aphel_kernel::infrastructure::repositories::mysql_organizations::MySqlOrganizationRepository::new(pool)
            .find()
            .await?
            .ok_or_else(|| {
                AppError::not_found_code(
                    "organization_not_installed",
                    "the organization is not installed yet",
                )
            })?;
    Ok(serde_json::json!({
        "name": organization.name.as_str(),
        "locale": organization.locale,
        "time_zone": organization.timezone,
        "currency": organization.currency,
        "primary_color": organization.primary_color,
    })
    .to_string())
}

pub(crate) fn effective_permissions(scope: &AccessScope) -> Vec<String> {
    let permissions = scope.permissions();
    if permissions.grants_everything() {
        let mut keys: Vec<String> = aphel_kernel::domain::access::permission_catalog()
            .into_iter()
            .map(|definition| definition.permission.as_str().to_owned())
            .chain(
                permissions
                    .catalog_extension()
                    .iter()
                    .map(|definition| definition.permission.as_str().to_owned()),
            )
            .collect();
        keys.sort();
        keys.dedup();
        return keys;
    }
    permissions
        .grants()
        .map(|grant| grant.permission.as_str().to_owned())
        .collect()
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

    async fn aggregate_records(&mut self, query_json: String) -> HostResult<String> {
        let scope = self.scope.clone();
        answer(aggregate(&scope, &query_json).await)
    }

    async fn iterate_records(
        &mut self,
        record_type: String,
        query: wit::ListQuery,
        cursor: Option<String>,
    ) -> HostResult<(Vec<String>, Option<String>)> {
        let scope = self.scope.clone();
        answer(iterate(&scope, &record_type, query, cursor).await)
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
        let reason = self.audit_reason.clone();
        answer(
            aphel_kernel::application::shared::with_audit_reason(
                reason,
                write(&scope, &record_type, None, &values_json),
            )
            .await,
        )
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
        let reason = self.audit_reason.clone();
        answer(
            aphel_kernel::application::shared::with_audit_reason(
                reason,
                write(&scope, &record_type, Some(&id), &values_json),
            )
            .await,
        )
    }

    async fn delete_record(&mut self, record_type: String, id: String) -> HostResult<()> {
        if let Err(error) = outside_transaction(self) {
            return answer(Err(error));
        }
        let scope = self.scope.clone();
        let reason = self.audit_reason.clone();
        answer(
            aphel_kernel::application::shared::with_audit_reason(
                reason,
                delete(&scope, &record_type, &id),
            )
            .await,
        )
    }

    async fn update_record_if(
        &mut self,
        record_type: String,
        id: String,
        expected_updated_at: String,
        values_json: String,
    ) -> HostResult<String> {
        if let Err(error) = outside_transaction(self) {
            return answer(Err(error));
        }
        let scope = self.scope.clone();
        let reason = self.audit_reason.clone();
        answer(
            aphel_kernel::application::shared::with_audit_reason(
                reason,
                update_if(
                    &scope,
                    &record_type,
                    &id,
                    &expected_updated_at,
                    &values_json,
                ),
            )
            .await,
        )
    }

    async fn apply_records(
        &mut self,
        mutations: Vec<wit::RecordMutation>,
    ) -> HostResult<Vec<String>> {
        if let Err(error) = outside_transaction(self) {
            return answer(Err(error));
        }
        let scope = self.scope.clone();
        let reason = self.audit_reason.clone();
        answer(
            aphel_kernel::application::shared::with_audit_reason(reason, apply(&scope, mutations))
                .await,
        )
    }

    async fn upload_file(
        &mut self,
        record_type: String,
        record_id: String,
        filename: String,
        body: Vec<u8>,
    ) -> HostResult<String> {
        if let Err(error) = outside_transaction(self) {
            return answer(Err(error));
        }
        let scope = self.scope.clone();
        answer(crate::files::upload(&scope, &record_type, &record_id, filename, body).await)
    }

    async fn read_file(&mut self, file_id: String) -> HostResult<(String, Vec<u8>)> {
        let scope = self.scope.clone();
        answer(crate::files::read(&scope, &file_id).await)
    }

    async fn respond_bytes(
        &mut self,
        content_type: String,
        filename: Option<String>,
        body: Vec<u8>,
    ) -> HostResult<()> {
        answer(raw_body(self, content_type, filename, body))
    }

    async fn set_audit_reason(&mut self, reason: Option<String>) {
        self.audit_reason = reason
            .map(|reason| reason.trim().to_owned())
            .filter(|reason| !reason.is_empty());
    }

    async fn can(&mut self, permission: String) -> bool {
        aphel_kernel::domain::access::Permission::parse(permission)
            .ok()
            .is_some_and(|permission| self.scope.permissions().scope_for(&permission).is_some())
    }

    async fn permissions(&mut self) -> Vec<String> {
        effective_permissions(&self.scope)
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

    async fn kv_increment(
        &mut self,
        collection: String,
        id: String,
        delta: i64,
    ) -> HostResult<i64> {
        if let Err(error) = outside_transaction(self) {
            return answer(Err(error));
        }
        let plugin = self.plugin.key.clone();
        answer(
            async {
                checked(&collection, Some(&id))?;
                let pool = runtime_state()?.db()?;
                store::kv_increment(&pool, &plugin, &collection, &id, delta).await
            }
            .await,
        )
    }

    async fn kv_set_if_absent(
        &mut self,
        collection: String,
        id: String,
        value_json: String,
    ) -> HostResult<bool> {
        if let Err(error) = outside_transaction(self) {
            return answer(Err(error));
        }
        let plugin = self.plugin.key.clone();
        answer(
            async {
                checked(&collection, Some(&id))?;
                let value = parse(&value_json)?;
                let pool = runtime_state()?.db()?;
                store::kv_set_if_absent(&pool, &plugin, &collection, &id, &value).await
            }
            .await,
        )
    }

    async fn next_number(&mut self, sequence: String) -> HostResult<u64> {
        if let Err(error) = outside_transaction(self) {
            return answer(Err(error));
        }
        let plugin = self.plugin.key.clone();
        answer(
            async {
                checked(SEQUENCE_COLLECTION, Some(&sequence))?;
                let pool = runtime_state()?.db()?;
                let next =
                    store::kv_increment(&pool, &plugin, SEQUENCE_COLLECTION, &sequence, 1).await?;
                u64::try_from(next).map_err(|_| {
                    AppError::conflict_code(
                        "plugin_kv_not_a_counter",
                        "the sequence was set to a negative value",
                    )
                })
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

    async fn list_users(&mut self, query_json: String) -> HostResult<String> {
        let anonymous = self.scope.is_system() && self.scope.plugin_key().is_none();
        answer(
            async {
                if anonymous {
                    return Err(AppError::forbidden_code(
                        "plugin_directory_unavailable",
                        "the directory needs a signed-in actor or the plugin installer",
                    ));
                }
                let query: crate::directory::UserQuery = if query_json.trim().is_empty() {
                    crate::directory::UserQuery::default()
                } else {
                    serde_json::from_str(&query_json).map_err(|error| {
                        AppError::Validation(format!("invalid user query: {error}"))
                    })?
                };
                let pool = runtime_state()?.db()?;
                Ok(crate::directory::users(&pool, query).await?.to_string())
            }
            .await,
        )
    }

    async fn list_roles(&mut self) -> HostResult<String> {
        answer(
            async {
                let pool = runtime_state()?.db()?;
                Ok(crate::directory::roles(&pool).await?.to_string())
            }
            .await,
        )
    }

    async fn list_teams(&mut self) -> HostResult<String> {
        answer(
            async {
                let pool = runtime_state()?.db()?;
                Ok(crate::directory::teams(&pool).await?.to_string())
            }
            .await,
        )
    }

    async fn organization_json(&mut self) -> HostResult<String> {
        answer(organization().await)
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
