use serde_json::{Map, Value};

use crate::bindings::aphel::plugin::host;
use crate::bindings::aphel::plugin::types as wit;
use crate::{
    ActionCall, Actor, ActorKind, Change, Ctx, Event, FieldError, HookCall, HttpRequest,
    HttpResponse, ListQuery, Notification, Page, Plugin, PluginError, Request, WidgetRequest,
};

fn object(json: &str) -> Map<String, Value> {
    match serde_json::from_str(json) {
        Ok(Value::Object(map)) => map,
        _ => Map::new(),
    }
}

fn value(json: &str) -> Value {
    serde_json::from_str(json).unwrap_or(Value::Null)
}

fn actor(actor: wit::Actor) -> Actor {
    Actor {
        kind: match actor.kind {
            wit::ActorKind::User => ActorKind::User,
            wit::ActorKind::ApiToken => ActorKind::ApiToken,
            wit::ActorKind::Automation => ActorKind::Automation,
            wit::ActorKind::Plugin => ActorKind::Plugin,
            wit::ActorKind::System => ActorKind::System,
            wit::ActorKind::Anonymous => ActorKind::Anonymous,
        },
        id: actor.id,
        user_id: actor.user_id,
    }
}

fn pairs(pairs: Vec<(String, String)>) -> Map<String, Value> {
    pairs
        .into_iter()
        .map(|(name, text)| (name, Value::String(text)))
        .collect()
}

impl From<wit::PluginError> for PluginError {
    fn from(error: wit::PluginError) -> Self {
        Self {
            code: error.code,
            message: error.message,
            status: error.status,
            fields: error
                .fields
                .into_iter()
                .map(|field| FieldError {
                    field: field.field,
                    message: field.message,
                })
                .collect(),
        }
    }
}

impl From<PluginError> for wit::PluginError {
    fn from(error: PluginError) -> Self {
        Self {
            code: error.code,
            message: error.message,
            status: error.status,
            fields: error
                .fields
                .into_iter()
                .map(|field| wit::FieldError {
                    field: field.field,
                    message: field.message,
                })
                .collect(),
        }
    }
}

pub(crate) fn get_record(record_type: &str, id: &str) -> Result<Option<Value>, PluginError> {
    Ok(host::get_record(record_type, id)?.map(|json| value(&json)))
}

pub(crate) fn list_records(record_type: &str, query: ListQuery) -> Result<Page, PluginError> {
    let page = host::list_records(
        record_type,
        &wit::ListQuery {
            filter_json: query.filter.map(|filter| filter.to_string()),
            sort: query.sort,
            search: query.search,
            page: query.page.map(|page| page as u32),
            per_page: query.per_page.map(|per_page| per_page as u32),
        },
    )?;
    Ok(Page {
        data: page.records_json.iter().map(|json| value(json)).collect(),
        total: page.total,
    })
}

pub(crate) fn create_record(record_type: &str, values: &Value) -> Result<Value, PluginError> {
    Ok(value(&host::create_record(
        record_type,
        &values.to_string(),
    )?))
}

pub(crate) fn update_record(
    record_type: &str,
    id: &str,
    values: &Value,
) -> Result<Value, PluginError> {
    Ok(value(&host::update_record(
        record_type,
        id,
        &values.to_string(),
    )?))
}

pub(crate) fn delete_record(record_type: &str, id: &str) -> Result<(), PluginError> {
    Ok(host::delete_record(record_type, id)?)
}

pub(crate) fn kv_get(collection: &str, id: &str) -> Result<Option<Value>, PluginError> {
    Ok(host::kv_get(collection, id)?.map(|json| value(&json)))
}

pub(crate) fn kv_set(collection: &str, id: &str, item: &Value) -> Result<(), PluginError> {
    Ok(host::kv_set(collection, id, &item.to_string())?)
}

pub(crate) fn kv_delete(collection: &str, id: &str) -> Result<(), PluginError> {
    Ok(host::kv_delete(collection, id)?)
}

pub(crate) fn kv_list(collection: &str) -> Result<Vec<(String, Value)>, PluginError> {
    Ok(host::kv_list(collection)?
        .into_iter()
        .map(|(id, json)| (id, value(&json)))
        .collect())
}

pub(crate) fn http_fetch(request: HttpRequest) -> Result<HttpResponse, PluginError> {
    let response = host::http_fetch(&wit::HttpRequest {
        method: request.method,
        url: request.url,
        headers: request.headers,
        body: request.body,
    })?;
    Ok(HttpResponse {
        status: response.status,
        headers: response.headers,
        body: response.body,
    })
}

pub(crate) fn notify(notification: Notification) -> Result<(), PluginError> {
    Ok(host::notify(&wit::Notification {
        kind: notification.kind,
        recipient_user_id: notification.recipient_user_id,
        title: notification.title,
        body: notification.body,
        record_type: notification.record_type,
        record_id: notification.record_id,
        link: notification.link,
        data_json: Value::Object(notification.data).to_string(),
        dedupe_key: notification.dedupe_key,
    })?)
}

pub(crate) fn emit(
    name: &str,
    record: Option<(&str, &str)>,
    payload: &Value,
) -> Result<(), PluginError> {
    Ok(host::emit(
        name,
        record.map(|(record_type, _)| record_type),
        record.map(|(_, record_id)| record_id),
        &payload.to_string(),
    )?)
}

pub(crate) fn invoke(operation: &str, input: &Value) -> Result<Value, PluginError> {
    Ok(value(&host::invoke(operation, &input.to_string())?))
}

pub(crate) fn secret(key: &str) -> Result<Option<String>, PluginError> {
    Ok(host::secret(key)?)
}

pub(crate) fn settings() -> Map<String, Value> {
    object(&host::settings_json())
}

pub(crate) fn now() -> String {
    host::now()
}

pub(crate) fn log(level: &str, message: &str) {
    host::log(level, message);
}

pub fn on_hook<P: Plugin>(
    plugin: &P,
    call: wit::HookCall,
) -> Result<wit::HookOutcome, wit::PluginError> {
    let call = HookCall {
        hook: call.hook,
        record_type: call.record_type,
        record_id: call.record_id,
        mode: call.mode,
        actor: actor(call.actor),
        changes: call
            .changes
            .into_iter()
            .map(|change| Change {
                field: change.field,
                old: value(&change.old_json),
                new: value(&change.new_json),
            })
            .collect(),
        values: object(&call.values_json),
        custom_fields: object(&call.custom_fields_json),
    };
    let outcome = plugin.on_hook(&Ctx::new(), &call)?;
    Ok(wit::HookOutcome {
        values_json: Value::Object(outcome.values).to_string(),
        custom_fields_json: Value::Object(outcome.custom_fields).to_string(),
    })
}

pub fn on_event<P: Plugin>(plugin: &P, event: wit::Event) -> Result<(), wit::PluginError> {
    let event = Event {
        id: event.id,
        event_type: event.event_type,
        record_type: event.record_type,
        record_id: event.record_id,
        actor: actor(event.actor),
        occurred_at: event.occurred_at,
        payload: value(&event.payload_json),
    };
    Ok(plugin.on_event(&Ctx::new(), &event)?)
}

pub fn on_schedule<P: Plugin>(plugin: &P, name: &str, slot: &str) -> Result<(), wit::PluginError> {
    Ok(plugin.on_schedule(&Ctx::new(), name, slot)?)
}

pub fn on_action<P: Plugin>(plugin: &P, call: wit::ActionCall) -> Result<String, wit::PluginError> {
    let call = ActionCall {
        name: call.name,
        input: object(&call.input_json),
        record_type: call.record_type,
        record_id: call.record_id,
        record: value(&call.record_json),
        automation_id: call.automation_id,
        run_id: call.run_id,
        actor: actor(call.actor),
    };
    Ok(plugin.on_action(&Ctx::new(), &call)?.to_string())
}

pub fn on_widget<P: Plugin>(
    plugin: &P,
    name: String,
    request: wit::WidgetRequest,
) -> Result<String, wit::PluginError> {
    let request = WidgetRequest {
        name,
        settings: object(&request.settings_json),
        period_from: request.period_from,
        period_to: request.period_to,
        timezone: request.timezone,
        actor: actor(request.actor),
    };
    Ok(plugin.on_widget(&Ctx::new(), &request)?.to_string())
}

pub fn on_route<P: Plugin>(
    plugin: &P,
    request: wit::Request,
) -> Result<wit::Response, wit::PluginError> {
    let request = Request {
        method: request.method,
        path: request.path,
        route: request.route,
        params: pairs(request.params),
        query: pairs(request.query),
        headers: request.headers,
        body: value(&request.body_json),
        actor: actor(request.actor),
    };
    let response = plugin.on_route(&Ctx::new(), &request)?;
    Ok(wit::Response {
        status: response.status,
        body_json: response.body.to_string(),
    })
}

pub(crate) fn defer_effect(key: &str, command: &Value) -> Result<String, PluginError> {
    Ok(host::defer_effect(key, &command.to_string())?)
}

pub(crate) fn execution_context() -> Value {
    value(&host::execution_context_json())
}

pub(crate) fn hook_state() -> Value {
    value(&host::hook_state_json())
}
