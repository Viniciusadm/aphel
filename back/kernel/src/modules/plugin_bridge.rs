use std::sync::Arc;

use async_trait::async_trait;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::Ctx;
use crate::errors::AppError;
use crate::modules::hooks::{BoxFuture, HookCtx};

pub const PLUGIN_HOOK_PRIORITY: i32 = 50;

pub type OperationFuture<'a> = BoxFuture<'a, Result<Value, AppError>>;
pub type OperationFn = for<'a, 'b> fn(&'a Ctx<'b>, Value) -> OperationFuture<'a>;

pub trait OperationHandler: Send + Sync {
    fn call<'a>(&'a self, ctx: &'a Ctx<'_>, input: Value) -> OperationFuture<'a>;
}

impl OperationHandler for OperationFn {
    fn call<'a>(&'a self, ctx: &'a Ctx<'_>, input: Value) -> OperationFuture<'a> {
        self(ctx, input)
    }
}

pub struct PluginOperation {
    pub key: &'static str,
    pub description: &'static str,
    pub writes: bool,
    pub durable: bool,
    pub external_query: bool,
    handler: Arc<dyn OperationHandler>,
}

impl PluginOperation {
    pub fn read(key: &'static str, description: &'static str, run: OperationFn) -> Self {
        Self::with_handler(key, description, false, Arc::new(run))
    }

    pub fn with_handler(
        key: &'static str,
        description: &'static str,
        writes: bool,
        handler: Arc<dyn OperationHandler>,
    ) -> Self {
        Self {
            key,
            description,
            writes,
            durable: false,
            external_query: false,
            handler,
        }
    }

    pub fn invoke<'a>(&'a self, ctx: &'a Ctx<'_>, input: Value) -> OperationFuture<'a> {
        self.handler.call(ctx, input)
    }

    pub const fn with_durable_mutations(mut self) -> Self {
        self.durable = true;
        self
    }

    pub const fn with_external_query(mut self) -> Self {
        self.external_query = true;
        self
    }

    pub fn write(key: &'static str, description: &'static str, run: OperationFn) -> Self {
        Self::with_handler(key, description, true, Arc::new(run))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginHookKind {
    Veto,
    Filter,
    Action,
}

impl PluginHookKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Veto => "veto",
            Self::Filter => "filter",
            Self::Action => "action",
        }
    }
}

pub struct PluginHook {
    pub key: &'static str,
    pub record_type: &'static str,
    pub kind: PluginHookKind,
    pub description: &'static str,
}

impl PluginHook {
    pub const fn new(
        key: &'static str,
        record_type: &'static str,
        kind: PluginHookKind,
        description: &'static str,
    ) -> Self {
        Self {
            key,
            record_type,
            kind,
            description,
        }
    }
}

pub struct PluginHookCall<'a> {
    pub hook: &'static str,
    pub record_type: &'a str,
    pub record_id: String,
    pub values: Value,
}

#[async_trait]
pub trait PluginHookDispatcher: Send + Sync {
    fn subscribed(&self, hook: &str, record_type: &str) -> bool;

    async fn dispatch(
        &self,
        ctx: &mut HookCtx<'_>,
        call: PluginHookCall<'_>,
    ) -> Result<Vec<Value>, AppError>;
}

fn dispatcher(ctx: &HookCtx<'_>) -> Option<Arc<dyn PluginHookDispatcher>> {
    ctx.services.get::<dyn PluginHookDispatcher>()
}

pub fn plugin_hook_subscribed(ctx: &Ctx<'_>, hook: &str, record_type: &str) -> bool {
    ctx.services()
        .get::<dyn PluginHookDispatcher>()
        .is_some_and(|dispatcher| dispatcher.subscribed(hook, record_type))
}

pub async fn forward_plugin_hook(
    ctx: &mut HookCtx<'_>,
    hook: &'static str,
    record_type: &str,
    record_id: impl ToString,
    values: impl FnOnce() -> Value,
) -> Result<Vec<Value>, AppError> {
    let Some(dispatcher) = dispatcher(ctx) else {
        return Ok(Vec::new());
    };
    if !dispatcher.subscribed(hook, record_type) {
        return Ok(Vec::new());
    }
    let call = PluginHookCall {
        hook,
        record_type,
        record_id: record_id.to_string(),
        values: values(),
    };
    dispatcher.dispatch(ctx, call).await
}

pub fn operation_input<T: DeserializeOwned>(input: Value) -> Result<T, AppError> {
    serde_json::from_value(input)
        .map_err(|error| AppError::Validation(format!("invalid operation input: {error}")))
}

pub fn operation_output<T: serde::Serialize>(output: T) -> Result<Value, AppError> {
    serde_json::to_value(output).map_err(|error| AppError::Internal(error.to_string()))
}

const MAX_OPERATION_PAGE_SIZE: u64 = 100;

#[derive(Debug, Clone, Copy, Default, serde::Deserialize)]
pub struct OperationPage {
    #[serde(default)]
    pub page: Option<u64>,
    #[serde(default)]
    pub per_page: Option<u64>,
}

impl OperationPage {
    pub fn window(self) -> crate::application::organization_admin::ListWindow {
        let per_page = self
            .per_page
            .unwrap_or(25)
            .clamp(1, MAX_OPERATION_PAGE_SIZE);
        let page = self.page.unwrap_or(1).max(1);
        crate::application::organization_admin::ListWindow {
            offset: (page - 1) * per_page,
            limit: per_page,
        }
    }
}
