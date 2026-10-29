#[doc(hidden)]
pub mod abi;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
pub use serde_json::{self, Map, Value, json};

pub type Result<T> = std::result::Result<T, PluginError>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldError {
    pub field: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginError {
    pub code: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<u16>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<FieldError>,
}

impl PluginError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            status: None,
            fields: Vec::new(),
        }
    }

    pub fn field(
        code: impl Into<String>,
        field: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        let message = message.into();
        Self {
            fields: vec![FieldError {
                field: field.into(),
                message: message.clone(),
            }],
            ..Self::new(code, message)
        }
    }

    pub fn not_found(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(code, message).with_status(404)
    }

    pub fn forbidden(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(code, message).with_status(403)
    }

    pub fn with_status(mut self, status: u16) -> Self {
        self.status = Some(status);
        self
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::new("plugin_error", message)
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Change {
    pub field: String,
    #[serde(default)]
    pub old: Value,
    #[serde(default)]
    pub new: Value,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HookCall {
    pub hook: String,
    pub record_type: String,
    pub record_id: String,
    #[serde(default)]
    pub mode: Option<String>,
    #[serde(default)]
    pub changes: Vec<Change>,
    #[serde(default)]
    pub values: Map<String, Value>,
    #[serde(default)]
    pub custom_fields: Map<String, Value>,
}

impl HookCall {
    pub fn is(&self, hook: &str, record_type: &str) -> bool {
        self.hook == hook && self.record_type == record_type
    }

    pub fn changed(&self, field: &str) -> Option<&Change> {
        self.changes.iter().find(|change| change.field == field)
    }

    pub fn custom_changed(&self, field: &str) -> Option<&Change> {
        self.changed(&format!("custom_fields.{field}"))
    }

    pub fn is_create(&self) -> bool {
        self.mode.as_deref() == Some("create")
    }
}

pub mod hooks {
    pub const SAVING: &str = "record.saving";
    pub const VALIDATING: &str = "record.validating";
    pub const SAVED: &str = "record.saved";
    pub const DELETING: &str = "record.deleting";
}

#[derive(Debug, Clone, Default)]
pub struct HookOutcome {
    pub values: Map<String, Value>,
    pub custom_fields: Map<String, Value>,
}

impl HookOutcome {
    pub fn set(mut self, field: &str, value: Value) -> Self {
        self.values.insert(field.to_owned(), value);
        self
    }

    pub fn set_custom(mut self, field: &str, value: Value) -> Self {
        self.custom_fields.insert(field.to_owned(), value);
        self
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Event {
    pub id: String,
    #[serde(rename = "type")]
    pub event_type: String,
    #[serde(default)]
    pub record_type: Option<String>,
    #[serde(default)]
    pub record_id: Option<String>,
    #[serde(default)]
    pub payload: Value,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Request {
    pub method: String,
    pub path: String,
    pub route: String,
    #[serde(default)]
    pub params: Map<String, Value>,
    #[serde(default)]
    pub query: Map<String, Value>,
    #[serde(default)]
    pub body: Value,
    #[serde(default)]
    pub user_id: Option<String>,
}

impl Request {
    pub fn is(&self, method: &str, route: &str) -> bool {
        self.method == method && self.route == route
    }

    pub fn param(&self, name: &str) -> Option<&str> {
        self.params.get(name).and_then(Value::as_str)
    }

    pub fn query(&self, name: &str) -> Option<&str> {
        self.query.get(name).and_then(Value::as_str)
    }

    pub fn body<T: DeserializeOwned>(&self) -> Result<T> {
        serde_json::from_value(self.body.clone())
            .map_err(|error| PluginError::new("invalid_body", error.to_string()))
    }
}

#[derive(Debug, Clone)]
pub struct Response {
    pub status: u16,
    pub body: Value,
}

impl Response {
    pub fn ok(body: impl Serialize) -> Self {
        Self::with_status(200, body)
    }

    pub fn created(body: impl Serialize) -> Self {
        Self::with_status(201, body)
    }

    pub fn with_status(status: u16, body: impl Serialize) -> Self {
        Self {
            status,
            body: serde_json::to_value(body).unwrap_or(Value::Null),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct ListQuery {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub per_page: Option<u64>,
}

impl ListQuery {
    pub fn filter(mut self, filter: Value) -> Self {
        self.filter = Some(filter);
        self
    }

    pub fn eq(field: &str, value: impl Serialize) -> Self {
        Self::default().filter(json!({
            "field": field,
            "operator": "eq",
            "value": serde_json::to_value(value).unwrap_or(Value::Null),
        }))
    }

    pub fn per_page(mut self, per_page: u64) -> Self {
        self.per_page = Some(per_page);
        self
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Page {
    pub data: Vec<Value>,
    pub total: u64,
}

pub struct Ctx {
    _private: (),
}

impl Ctx {
    fn new() -> Self {
        Self { _private: () }
    }

    fn call(&self, request: Value) -> Result<Value> {
        abi::call_host(&request)
    }

    pub fn records(&self) -> Records<'_> {
        Records { ctx: self }
    }

    pub fn kv(&self, collection: &str) -> Kv<'_> {
        Kv {
            ctx: self,
            collection: collection.to_owned(),
        }
    }

    pub fn settings(&self) -> Result<Map<String, Value>> {
        match self.call(json!({ "op": "settings.get" }))? {
            Value::Object(settings) => Ok(settings),
            _ => Ok(Map::new()),
        }
    }

    pub fn setting<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>> {
        Ok(self
            .settings()?
            .get(key)
            .cloned()
            .and_then(|value| serde_json::from_value(value).ok()))
    }

    pub fn now(&self) -> Result<String> {
        Ok(self
            .call(json!({ "op": "now" }))?
            .as_str()
            .unwrap_or_default()
            .to_owned())
    }

    pub fn log(&self, level: &str, message: impl Into<String>) {
        let _ = self.call(json!({ "op": "log", "level": level, "message": message.into() }));
    }

    pub fn info(&self, message: impl Into<String>) {
        self.log("info", message);
    }
}

pub struct Records<'a> {
    ctx: &'a Ctx,
}

impl Records<'_> {
    pub fn get(&self, record_type: &str, id: &str) -> Result<Option<Value>> {
        let record = self
            .ctx
            .call(json!({ "op": "records.get", "record_type": record_type, "id": id }))?;
        Ok((!record.is_null()).then_some(record))
    }

    pub fn list(&self, record_type: &str, query: ListQuery) -> Result<Page> {
        let mut request = serde_json::to_value(query).unwrap_or_else(|_| json!({}));
        request["op"] = json!("records.list");
        request["record_type"] = json!(record_type);
        serde_json::from_value(self.ctx.call(request)?)
            .map_err(|error| PluginError::internal(error.to_string()))
    }

    pub fn create(&self, record_type: &str, values: Value) -> Result<Value> {
        self.ctx
            .call(json!({ "op": "records.create", "record_type": record_type, "values": values }))
    }

    pub fn update(&self, record_type: &str, id: &str, values: Value) -> Result<Value> {
        self.ctx.call(json!({
            "op": "records.update",
            "record_type": record_type,
            "id": id,
            "values": values,
        }))
    }

    pub fn delete(&self, record_type: &str, id: &str) -> Result<()> {
        self.ctx
            .call(json!({ "op": "records.delete", "record_type": record_type, "id": id }))
            .map(|_| ())
    }
}

pub struct Kv<'a> {
    ctx: &'a Ctx,
    collection: String,
}

impl Kv<'_> {
    pub fn get<T: DeserializeOwned>(&self, id: &str) -> Result<Option<T>> {
        let value = self
            .ctx
            .call(json!({ "op": "kv.get", "collection": self.collection, "id": id }))?;
        Ok(serde_json::from_value(value).ok())
    }

    pub fn set(&self, id: &str, value: impl Serialize) -> Result<()> {
        self.ctx
            .call(json!({
                "op": "kv.set",
                "collection": self.collection,
                "id": id,
                "value": serde_json::to_value(value).unwrap_or(Value::Null),
            }))
            .map(|_| ())
    }

    pub fn delete(&self, id: &str) -> Result<()> {
        self.ctx
            .call(json!({ "op": "kv.delete", "collection": self.collection, "id": id }))
            .map(|_| ())
    }

    pub fn list(&self) -> Result<Vec<(String, Value)>> {
        let items = self
            .ctx
            .call(json!({ "op": "kv.list", "collection": self.collection }))?;
        Ok(items
            .as_array()
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| {
                        Some((
                            item.get("id")?.as_str()?.to_owned(),
                            item.get("value")?.clone(),
                        ))
                    })
                    .collect()
            })
            .unwrap_or_default())
    }
}

pub trait Plugin {
    fn on_hook(&self, _ctx: &Ctx, _call: &HookCall) -> Result<HookOutcome> {
        Ok(HookOutcome::default())
    }

    fn on_event(&self, _ctx: &Ctx, _event: &Event) -> Result<()> {
        Ok(())
    }

    fn on_route(&self, _ctx: &Ctx, request: &Request) -> Result<Response> {
        Err(PluginError::not_found(
            "route_not_found",
            format!("no route for {} {}", request.method, request.path),
        ))
    }
}

#[macro_export]
macro_rules! export_plugin {
    ($plugin:expr) => {
        #[unsafe(no_mangle)]
        pub extern "C" fn aphel_alloc(length: i32) -> i32 {
            $crate::abi::alloc(length)
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn aphel_call(pointer: i32, length: i32) -> i64 {
            $crate::abi::call(&$plugin, pointer, length)
        }
    };
}
