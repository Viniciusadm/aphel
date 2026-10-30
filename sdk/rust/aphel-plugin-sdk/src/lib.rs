mod execution;
pub use execution::{
    AutomationExecution, EffectCommand, ExecutionActor, ExecutionContext, HookState, RecordSnapshot,
};

#[doc(hidden)]
pub mod adapter;

#[doc(hidden)]
#[allow(clippy::too_many_arguments)]
pub mod bindings {
    wit_bindgen::generate!({
        path: "wit",
        world: "plugin",
        pub_export_macro: true,
        default_bindings_module: "aphel_plugin_sdk::bindings",
    });
}

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActorKind {
    User,
    ApiToken,
    Automation,
    Plugin,
    System,
    Anonymous,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Actor {
    pub kind: ActorKind,
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub user_id: Option<String>,
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
    pub actor: Actor,
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
    pub const COMMITTED: &str = "record.committed";
    pub const DELETED: &str = "record.deleted";
    pub const RESTORED: &str = "record.restored";
    pub const PURGED: &str = "record.purged";
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
    pub actor: Actor,
    pub occurred_at: String,
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
    pub headers: Vec<(String, String)>,
    #[serde(default)]
    pub body: Value,
    pub actor: Actor,
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

    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(header, _)| header.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }

    pub fn header_values<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a str> + 'a {
        self.headers
            .iter()
            .filter(move |(header, _)| header.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
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

    pub fn records(&self) -> Records<'_> {
        Records { _ctx: self }
    }

    pub fn kv(&self, collection: &str) -> Kv<'_> {
        Kv {
            _ctx: self,
            collection: collection.to_owned(),
        }
    }

    pub fn settings(&self) -> Result<Map<String, Value>> {
        Ok(adapter::settings())
    }

    pub fn setting<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>> {
        Ok(adapter::settings()
            .get(key)
            .cloned()
            .and_then(|value| serde_json::from_value(value).ok()))
    }

    pub fn http(&self) -> Http<'_> {
        Http { _ctx: self }
    }

    pub fn notify(&self, notification: Notification) -> Result<()> {
        adapter::notify(notification)
    }

    pub fn emit(&self, name: &str, record: Option<(&str, &str)>, payload: Value) -> Result<()> {
        adapter::emit(name, record, &payload)
    }

    pub fn invoke<T: DeserializeOwned>(&self, operation: &str, input: impl Serialize) -> Result<T> {
        let input = serde_json::to_value(input)
            .map_err(|error| PluginError::new("invalid_input", error.to_string()))?;
        serde_json::from_value(adapter::invoke(operation, &input)?)
            .map_err(|error| PluginError::new("invalid_response", error.to_string()))
    }

    pub fn defer_effect(&self, key: &str, command: impl Serialize) -> Result<String> {
        let command = serde_json::to_value(command)
            .map_err(|error| PluginError::new("invalid_input", error.to_string()))?;
        adapter::defer_effect(key, &command)
    }

    pub fn next_number(&self, sequence: &str) -> Result<u64> {
        adapter::next_number(sequence)
    }

    pub fn set_audit_reason(&self, reason: Option<&str>) {
        adapter::set_audit_reason(reason);
    }

    pub fn upload_file(
        &self,
        record_type: &str,
        record_id: &str,
        filename: &str,
        body: &[u8],
    ) -> Result<Value> {
        adapter::upload_file(record_type, record_id, filename, body)
    }

    pub fn read_file(&self, file_id: &str) -> Result<(Value, Vec<u8>)> {
        adapter::read_file(file_id)
    }

    pub fn respond_bytes(
        &self,
        content_type: &str,
        filename: Option<&str>,
        body: &[u8],
    ) -> Result<()> {
        adapter::respond_bytes(content_type, filename, body)
    }

    pub fn organization(&self) -> Result<Value> {
        adapter::organization()
    }

    pub fn list_users(&self, query: Value) -> Result<Value> {
        adapter::list_users(&query)
    }

    pub fn list_roles(&self) -> Result<Value> {
        adapter::list_roles()
    }

    pub fn list_teams(&self) -> Result<Value> {
        adapter::list_teams()
    }

    pub fn hook_state(&self) -> Result<Option<HookState>> {
        serde_json::from_value(adapter::hook_state())
            .map_err(|error| PluginError::new("invalid_hook_state", error.to_string()))
    }

    pub fn execution_context(&self) -> Result<Option<ExecutionContext>> {
        serde_json::from_value(adapter::execution_context())
            .map_err(|error| PluginError::new("invalid_execution_context", error.to_string()))
    }

    pub fn secret(&self, key: &str) -> Result<Option<String>> {
        adapter::secret(key)
    }

    pub fn can(&self, permission: &str) -> bool {
        adapter::can(permission)
    }

    pub fn permissions(&self) -> Vec<String> {
        adapter::permissions()
    }

    pub fn now(&self) -> Result<String> {
        Ok(adapter::now())
    }

    pub fn log(&self, level: &str, message: impl Into<String>) {
        adapter::log(level, &message.into());
    }

    pub fn info(&self, message: impl Into<String>) {
        self.log("info", message);
    }
}

pub struct Records<'a> {
    _ctx: &'a Ctx,
}

impl Records<'_> {
    pub fn get(&self, record_type: &str, id: &str) -> Result<Option<Value>> {
        adapter::get_record(record_type, id)
    }

    pub fn list(&self, record_type: &str, query: ListQuery) -> Result<Page> {
        adapter::list_records(record_type, query)
    }

    pub fn create(&self, record_type: &str, values: Value) -> Result<Value> {
        adapter::create_record(record_type, &values)
    }

    pub fn update(&self, record_type: &str, id: &str, values: Value) -> Result<Value> {
        adapter::update_record(record_type, id, &values)
    }

    pub fn delete(&self, record_type: &str, id: &str) -> Result<()> {
        adapter::delete_record(record_type, id)
    }

    pub fn update_if(
        &self,
        record_type: &str,
        id: &str,
        expected_updated_at: &str,
        values: Value,
    ) -> Result<Value> {
        adapter::update_record_if(record_type, id, expected_updated_at, &values)
    }

    pub fn apply(&self, mutations: &[Mutation]) -> Result<Vec<Value>> {
        adapter::apply_records(mutations)
    }

    pub fn aggregate(&self, query: Value) -> Result<Value> {
        adapter::aggregate_records(&query)
    }

    pub fn iterate(
        &self,
        record_type: &str,
        query: ListQuery,
        cursor: Option<&str>,
    ) -> Result<(Vec<Value>, Option<String>)> {
        adapter::iterate_records(record_type, query, cursor)
    }
}

#[derive(Debug, Clone)]
pub struct Mutation {
    pub record_type: String,
    pub id: Option<String>,
    pub expected_updated_at: Option<String>,
    pub values: Value,
}

impl Mutation {
    pub fn create(record_type: &str, values: Value) -> Self {
        Self {
            record_type: record_type.to_owned(),
            id: None,
            expected_updated_at: None,
            values,
        }
    }

    pub fn update(record_type: &str, id: &str, values: Value) -> Self {
        Self {
            record_type: record_type.to_owned(),
            id: Some(id.to_owned()),
            expected_updated_at: None,
            values,
        }
    }

    pub fn update_if(
        record_type: &str,
        id: &str,
        expected_updated_at: &str,
        values: Value,
    ) -> Self {
        Self {
            expected_updated_at: Some(expected_updated_at.to_owned()),
            ..Self::update(record_type, id, values)
        }
    }

    pub fn reference(index: usize) -> Value {
        json!({ "$ref": index })
    }
}

pub struct Kv<'a> {
    _ctx: &'a Ctx,
    collection: String,
}

impl Kv<'_> {
    pub fn get<T: DeserializeOwned>(&self, id: &str) -> Result<Option<T>> {
        Ok(adapter::kv_get(&self.collection, id)?
            .and_then(|value| serde_json::from_value(value).ok()))
    }

    pub fn set(&self, id: &str, value: impl Serialize) -> Result<()> {
        adapter::kv_set(
            &self.collection,
            id,
            &serde_json::to_value(value).unwrap_or(Value::Null),
        )
    }

    pub fn delete(&self, id: &str) -> Result<()> {
        adapter::kv_delete(&self.collection, id)
    }

    pub fn list(&self) -> Result<Vec<(String, Value)>> {
        adapter::kv_list(&self.collection)
    }

    pub fn increment(&self, id: &str, delta: i64) -> Result<i64> {
        adapter::kv_increment(&self.collection, id, delta)
    }

    pub fn set_if_absent(&self, id: &str, value: impl Serialize) -> Result<bool> {
        adapter::kv_set_if_absent(
            &self.collection,
            id,
            &serde_json::to_value(value).unwrap_or(Value::Null),
        )
    }
}

#[derive(Debug, Clone, Default)]
pub struct HttpRequest {
    pub method: String,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Option<Vec<u8>>,
}

impl HttpRequest {
    pub fn new(method: &str, url: impl Into<String>) -> Self {
        Self {
            method: method.to_owned(),
            url: url.into(),
            ..Self::default()
        }
    }

    pub fn header(mut self, name: &str, value: impl Into<String>) -> Self {
        self.headers.push((name.to_owned(), value.into()));
        self
    }

    pub fn bearer(self, token: &str) -> Self {
        self.header("authorization", format!("Bearer {token}"))
    }

    pub fn json(mut self, body: &impl Serialize) -> Result<Self> {
        let bytes =
            serde_json::to_vec(body).map_err(|error| PluginError::internal(error.to_string()))?;
        self.body = Some(bytes);
        Ok(self.header("content-type", "application/json"))
    }

    pub fn body(mut self, body: impl Into<Vec<u8>>) -> Self {
        self.body = Some(body.into());
        self
    }
}

#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl HttpResponse {
    pub fn is_success(&self) -> bool {
        (200..300).contains(&self.status)
    }

    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(header, _)| header.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }

    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.body).into_owned()
    }

    pub fn json<T: DeserializeOwned>(&self) -> Result<T> {
        serde_json::from_slice(&self.body)
            .map_err(|error| PluginError::new("invalid_response", error.to_string()))
    }
}

pub struct Http<'a> {
    _ctx: &'a Ctx,
}

impl Http<'_> {
    pub fn send(&self, request: HttpRequest) -> Result<HttpResponse> {
        adapter::http_fetch(request)
    }

    pub fn get(&self, url: &str) -> Result<HttpResponse> {
        self.send(HttpRequest::new("GET", url))
    }
}

#[derive(Debug, Clone)]
pub struct ActionCall {
    pub name: String,
    pub input: Map<String, Value>,
    pub record_type: String,
    pub record_id: String,
    pub record: Value,
    pub automation_id: String,
    pub run_id: String,
    pub actor: Actor,
}

impl ActionCall {
    pub fn input<T: DeserializeOwned>(&self, key: &str) -> Option<T> {
        self.input
            .get(key)
            .cloned()
            .and_then(|value| serde_json::from_value(value).ok())
    }
}

#[derive(Debug, Clone)]
pub struct WidgetRequest {
    pub name: String,
    pub settings: Map<String, Value>,
    pub period_from: String,
    pub period_to: String,
    pub timezone: String,
    pub actor: Actor,
}

#[derive(Debug, Clone, Default)]
pub struct Notification {
    pub kind: String,
    pub recipient_user_id: String,
    pub title: String,
    pub body: Option<String>,
    pub record_type: Option<String>,
    pub record_id: Option<String>,
    pub link: Option<String>,
    pub data: Map<String, Value>,
    pub dedupe_key: Option<String>,
}

impl Notification {
    pub fn new(kind: &str, recipient_user_id: &str, title: impl Into<String>) -> Self {
        Self {
            kind: kind.to_owned(),
            recipient_user_id: recipient_user_id.to_owned(),
            title: title.into(),
            ..Self::default()
        }
    }

    pub fn body(mut self, body: impl Into<String>) -> Self {
        self.body = Some(body.into());
        self
    }

    pub fn record(mut self, record_type: &str, record_id: &str) -> Self {
        self.record_type = Some(record_type.to_owned());
        self.record_id = Some(record_id.to_owned());
        self
    }

    pub fn link(mut self, link: impl Into<String>) -> Self {
        self.link = Some(link.into());
        self
    }

    pub fn dedupe_key(mut self, key: impl Into<String>) -> Self {
        self.dedupe_key = Some(key.into());
        self
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

    fn on_schedule(&self, _ctx: &Ctx, _name: &str, _slot: &str) -> Result<()> {
        Ok(())
    }

    fn on_action(&self, _ctx: &Ctx, call: &ActionCall) -> Result<Value> {
        Err(PluginError::not_found(
            "action_not_found",
            format!("no action {}", call.name),
        ))
    }

    fn on_widget(&self, _ctx: &Ctx, request: &WidgetRequest) -> Result<Value> {
        Err(PluginError::not_found(
            "widget_not_found",
            format!("no widget {}", request.name),
        ))
    }
}

#[macro_export]
macro_rules! export_plugin {
    ($plugin:expr) => {
        struct AphelPluginExport;

        impl $crate::bindings::Guest for AphelPluginExport {
            fn on_hook(
                call: $crate::bindings::HookCall,
            ) -> ::std::result::Result<$crate::bindings::HookOutcome, $crate::bindings::PluginError> {
                $crate::adapter::on_hook(&$plugin, call)
            }

            fn on_event(
                event: $crate::bindings::Event,
            ) -> ::std::result::Result<(), $crate::bindings::PluginError> {
                $crate::adapter::on_event(&$plugin, event)
            }

            fn on_route(
                request: $crate::bindings::Request,
            ) -> ::std::result::Result<$crate::bindings::Response, $crate::bindings::PluginError> {
                $crate::adapter::on_route(&$plugin, request)
            }

            fn on_schedule(
                name: ::std::string::String,
                slot: ::std::string::String,
            ) -> ::std::result::Result<(), $crate::bindings::PluginError> {
                $crate::adapter::on_schedule(&$plugin, &name, &slot)
            }

            fn on_action(
                call: $crate::bindings::ActionCall,
            ) -> ::std::result::Result<::std::string::String, $crate::bindings::PluginError> {
                $crate::adapter::on_action(&$plugin, call)
            }

            fn on_widget(
                name: ::std::string::String,
                request: $crate::bindings::WidgetRequest,
            ) -> ::std::result::Result<::std::string::String, $crate::bindings::PluginError> {
                $crate::adapter::on_widget(&$plugin, name, request)
            }
        }

        $crate::bindings::export!(AphelPluginExport with_types_in $crate::bindings);
    };
}
