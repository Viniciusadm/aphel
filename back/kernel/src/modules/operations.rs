use std::marker::PhantomData;
use std::sync::Arc;

use aphel_contract::OperationKey;
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::Ctx;
use crate::application::access::Authorizer;
use crate::domain::access::Permission;
use crate::errors::AppError;
use crate::modules::hooks::BoxFuture;
use crate::modules::plugin_bridge::{
    OperationFuture, OperationHandler, PluginOperation, operation_input, operation_output,
};

pub const CONTRACT_UNAVAILABLE: &str = "contract_unavailable";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Effect {
    Read,
    Write,
    External,
}

impl Effect {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Write => "write",
            Self::External => "external",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Exposure {
    pub bulk: bool,
    pub ai: bool,
}

#[derive(Debug, Clone)]
pub struct AutomationBinding {
    pub action_key: &'static str,
    pub label: &'static str,
    pub config_schema: Value,
    pub reference: Option<ReferenceCheck>,
    pub required_entity_trait: Option<&'static str>,
    pub supports_custom_records: bool,
    pub record_permission: Option<&'static str>,
}

impl AutomationBinding {
    pub fn supports_record_type(&self, record_type: &str) -> bool {
        self.required_entity_trait.is_none_or(|key| {
            (self.supports_custom_records
                && crate::domain::custom_objects::is_custom_record_type(record_type))
                || crate::modules::try_app().is_some_and(|app| {
                    app.record_types()
                        .capabilities(record_type)
                        .is_some_and(|capabilities| capabilities.has_trait(key))
                })
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReferenceCheck {
    pub field: &'static str,
    pub kind: aphel_contract::ReferenceKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TargetSelector {
    pub record_type: Option<&'static str>,
    pub trait_key: Option<&'static str>,
}

#[derive(Debug, Clone)]
pub struct OperationSpec {
    pub description: &'static str,
    pub permission: Option<&'static str>,
    pub effect: Effect,
    pub target: Option<TargetSelector>,
    pub idempotent: bool,
    pub durable: bool,
    pub expose: Exposure,
    pub automation: Option<AutomationBinding>,
    pub input_schema: Option<Value>,
    pub output_schema: Option<Value>,
}

impl OperationSpec {
    pub fn read(description: &'static str) -> Self {
        Self::new(description, Effect::Read)
    }

    pub fn write(description: &'static str) -> Self {
        Self::new(description, Effect::Write)
    }

    pub fn external(description: &'static str) -> Self {
        Self::new(description, Effect::External)
    }

    fn new(description: &'static str, effect: Effect) -> Self {
        Self {
            description,
            permission: None,
            effect,
            target: None,
            idempotent: false,
            durable: false,
            expose: Exposure::default(),
            automation: None,
            input_schema: None,
            output_schema: None,
        }
    }

    pub fn requiring(mut self, permission: &'static str) -> Self {
        self.permission = Some(permission);
        self
    }

    pub fn idempotent(mut self) -> Self {
        self.idempotent = true;
        self
    }

    pub fn durable(mut self) -> Self {
        self.durable = true;
        self
    }

    pub fn targeting(mut self, target: TargetSelector) -> Self {
        self.target = Some(target);
        self
    }

    pub fn exposed(mut self, expose: Exposure) -> Self {
        self.expose = expose;
        self
    }

    pub fn automatable(mut self, binding: AutomationBinding) -> Self {
        self.automation = Some(binding);
        self
    }

    pub fn with_schemas(mut self, input: Value, output: Value) -> Self {
        self.input_schema = Some(input);
        self.output_schema = Some(output);
        self
    }
}

pub struct OperationEntry {
    pub key: &'static str,
    pub spec: OperationSpec,
    handler: Arc<dyn OperationHandler>,
}

impl OperationEntry {
    pub fn runtime(
        key: &'static str,
        spec: OperationSpec,
        handler: Arc<dyn OperationHandler>,
    ) -> Self {
        Self { key, spec, handler }
    }

    pub fn call<'a>(&'a self, ctx: &'a Ctx<'_>, input: Value) -> OperationFuture<'a> {
        self.handler.call(ctx, input)
    }

    pub fn plugin_operation(&self) -> PluginOperation {
        let mut operation = PluginOperation::with_handler(
            self.key,
            self.spec.description,
            self.spec.effect != Effect::Read,
            self.handler.clone(),
        );
        operation.durable = self.spec.durable;
        operation.external_query = self.spec.effect == Effect::External;
        operation
    }
}

pub(crate) struct TypedHandler<I, O, F> {
    permission: Option<Permission>,
    handler: F,
    marker: PhantomData<fn(I) -> O>,
}

impl<I, O, F> OperationHandler for TypedHandler<I, O, F>
where
    I: DeserializeOwned + 'static,
    O: Serialize + 'static,
    F: for<'a, 'b> Fn(&'a Ctx<'b>, I) -> BoxFuture<'a, Result<O, AppError>> + Send + Sync,
{
    fn call<'a>(&'a self, ctx: &'a Ctx<'_>, input: Value) -> OperationFuture<'a> {
        Box::pin(async move {
            if let Some(permission) = &self.permission {
                ctx.scope.require(permission)?;
            }
            let input: I = operation_input(input)?;
            let output = (self.handler)(ctx, input).await?;
            operation_output(output)
        })
    }
}

pub(crate) fn entry<I, O, F>(
    key: OperationKey<I, O>,
    spec: OperationSpec,
    handler: F,
) -> Result<OperationEntry, AppError>
where
    I: DeserializeOwned + 'static,
    O: Serialize + 'static,
    F: for<'a, 'b> Fn(&'a Ctx<'b>, I) -> BoxFuture<'a, Result<O, AppError>> + Send + Sync + 'static,
{
    let permission = spec
        .permission
        .map(|value| {
            Permission::parse(value).map_err(|error| {
                AppError::Configuration(format!(
                    "operation `{}` requires an invalid permission: {error:?}",
                    key.key()
                ))
            })
        })
        .transpose()?;
    Ok(OperationEntry {
        key: key.key(),
        spec,
        handler: Arc::new(TypedHandler {
            permission,
            handler,
            marker: PhantomData,
        }),
    })
}

impl<'a> Ctx<'a> {
    pub fn has_operation<I, O>(&self, key: OperationKey<I, O>) -> bool {
        self.app().operation(key.key()).is_some()
    }

    pub async fn call<I, O>(&self, key: OperationKey<I, O>, input: I) -> Result<O, AppError>
    where
        I: Serialize,
        O: DeserializeOwned,
    {
        let entry = self.app().operation(key.key()).ok_or_else(|| {
            AppError::unavailable_code(
                CONTRACT_UNAVAILABLE,
                format!("no enabled provider for operation `{}`", key.key()),
            )
        })?;
        let input = serde_json::to_value(input)
            .map_err(|error| AppError::Internal(format!("invalid operation input: {error}")))?;
        let output = entry.call(self, input).await?;
        serde_json::from_value(output)
            .map_err(|error| AppError::Internal(format!("invalid operation output: {error}")))
    }
}
