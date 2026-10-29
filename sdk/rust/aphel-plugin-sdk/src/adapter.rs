use serde_json::{Map, Value};

use crate::bindings::aphel::plugin::host;
use crate::bindings::aphel::plugin::types as wit;
use crate::{
    Change, Ctx, Event, FieldError, HookCall, ListQuery, Page, Plugin, PluginError, Request,
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
        payload: value(&event.payload_json),
    };
    Ok(plugin.on_event(&Ctx::new(), &event)?)
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
        body: value(&request.body_json),
        user_id: request.user_id,
    };
    let response = plugin.on_route(&Ctx::new(), &request)?;
    Ok(wit::Response {
        status: response.status,
        body_json: response.body.to_string(),
    })
}
