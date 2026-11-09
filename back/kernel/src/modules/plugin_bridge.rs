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
    pub idempotent: bool,
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
            idempotent: false,
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

    pub const fn with_idempotent_retries(mut self) -> Self {
        self.idempotent = true;
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

#[derive(Clone, Copy)]
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

pub trait PluginHookArgs {
    fn plugin_record_id(&self) -> String;

    fn plugin_payload(&self) -> Value;

    fn plugin_record_type(&self) -> Option<String> {
        None
    }
}

pub trait PluginHookFilter: PluginHookArgs + Sized {
    fn apply_plugin_patches(self, patches: &[Value]) -> Result<Self, AppError>;
}

pub type PatchApplier<A> = fn(A, &[Value]) -> Result<A, AppError>;

pub struct HookForwarding<A> {
    pub record_id: fn(&A) -> String,
    pub payload: fn(&A) -> Value,
    pub record_type: fn(&A) -> Option<String>,
    pub apply: Option<PatchApplier<A>>,
}

impl<A> Clone for HookForwarding<A> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<A> Copy for HookForwarding<A> {}

impl<A: PluginHookArgs> HookForwarding<A> {
    pub fn of() -> Self {
        Self {
            record_id: A::plugin_record_id,
            payload: A::plugin_payload,
            record_type: A::plugin_record_type,
            apply: None,
        }
    }
}

impl<A: PluginHookFilter> HookForwarding<A> {
    pub fn filtering() -> Self {
        Self {
            apply: Some(A::apply_plugin_patches),
            ..Self::of()
        }
    }
}

pub(crate) async fn forward_args<A>(
    ctx: &mut HookCtx<'_>,
    hook: PluginHook,
    forwarding: HookForwarding<A>,
    args: &A,
) -> Result<Vec<Value>, AppError> {
    let record_type = (forwarding.record_type)(args).unwrap_or_else(|| hook.record_type.to_owned());
    forward_plugin_hook(
        ctx,
        hook.key,
        &record_type,
        (forwarding.record_id)(args),
        || (forwarding.payload)(args),
    )
    .await
}

pub(crate) async fn forward_filtered<A>(
    ctx: &mut HookCtx<'_>,
    hook: PluginHook,
    forwarding: HookForwarding<A>,
    args: A,
) -> Result<A, AppError> {
    let patches = forward_args(ctx, hook, forwarding, &args).await?;
    match forwarding.apply {
        Some(apply) => apply(args, &patches),
        None => Ok(args),
    }
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
