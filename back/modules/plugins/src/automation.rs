use std::collections::HashSet;
use std::sync::{Arc, Mutex, OnceLock};

use async_trait::async_trait;
use serde_json::{Map, Value, json};

use crate::manifest::{ActionSpec, EmitSpec, InputSpec, PluginManifest};
use crate::registry::{self, LoadedPlugin};
use crate::runtime::{self, wit};
use aphel_kernel::application::automations::actions::{ActionContext, ValidationContext};
use aphel_kernel::application::automations::context::TriggerContext;
use aphel_kernel::application::automations::triggers::{
    TriggerCandidate, event_context, is_automatable,
};
use aphel_kernel::application::automations::{
    AutomationAction, AutomationActionSource, AutomationTrigger, AutomationTriggerSource,
};
use aphel_kernel::application::shared::DomainEvent;
use aphel_kernel::domain::access::Permission;
use aphel_kernel::errors::{AppError, FieldViolation};

const PLUGIN_UNAVAILABLE: &str = "plugin_unavailable";

fn leak(text: &str) -> &'static str {
    static TEXTS: OnceLock<Mutex<HashSet<&'static str>>> = OnceLock::new();
    let Ok(mut texts) = TEXTS.get_or_init(Default::default).lock() else {
        return Box::leak(text.to_owned().into_boxed_str());
    };
    if let Some(existing) = texts.get(text) {
        return existing;
    }
    let leaked: &'static str = Box::leak(text.to_owned().into_boxed_str());
    texts.insert(leaked);
    leaked
}

fn applies_to(record_types: &[String], record_type: &str) -> bool {
    if record_types.is_empty() {
        is_automatable(record_type)
    } else {
        record_types
            .iter()
            .any(|candidate| candidate == record_type)
    }
}

fn input_schema(input: &InputSpec) -> Value {
    let mut schema = match input.kind.as_str() {
        "number" => json!({ "type": "number" }),
        "boolean" => json!({ "type": "boolean" }),
        "select" => json!({ "type": "string", "enum": input.options }),
        _ => json!({ "type": "string" }),
    };
    schema["title"] = json!(input.label);
    schema
}

fn input_valid(input: &InputSpec, value: &Value) -> bool {
    match input.kind.as_str() {
        "number" => value.is_number(),
        "boolean" => value.is_boolean(),
        "select" => value
            .as_str()
            .is_some_and(|text| input.options.iter().any(|option| option == text)),
        _ => value.is_string(),
    }
}

struct PluginAction {
    plugin: String,
    key: &'static str,
    label: &'static str,
    description: &'static str,
    spec: ActionSpec,
}

impl PluginAction {
    fn new(manifest: &PluginManifest, spec: &ActionSpec) -> Self {
        Self {
            plugin: manifest.key.clone(),
            key: leak(&manifest.action_key(&spec.key)),
            label: leak(&spec.label),
            description: leak(spec.description.as_deref().unwrap_or_default()),
            spec: spec.clone(),
        }
    }

    fn plugin(&self) -> Result<Arc<LoadedPlugin>, AppError> {
        registry::cached()
            .plugin(&self.plugin)
            .filter(|plugin| plugin.backend.is_some() || plugin.backend_error.is_some())
            .cloned()
            .ok_or_else(|| {
                AppError::unavailable_code(
                    PLUGIN_UNAVAILABLE,
                    format!("the plugin {} is not installed or is disabled", self.plugin),
                )
            })
    }
}

#[async_trait]
impl AutomationAction for PluginAction {
    fn key(&self) -> &'static str {
        self.key
    }

    fn label(&self) -> &'static str {
        self.label
    }

    fn description(&self) -> &'static str {
        self.description
    }

    fn config_schema(&self) -> Value {
        let properties: Map<String, Value> = self
            .spec
            .inputs
            .iter()
            .map(|input| (input.key.clone(), input_schema(input)))
            .collect();
        let required: Vec<&str> = self
            .spec
            .inputs
            .iter()
            .filter(|input| input.required)
            .map(|input| input.key.as_str())
            .collect();
        json!({
            "type": "object",
            "properties": properties,
            "required": required,
            "additionalProperties": false,
        })
    }

    fn supports(&self, record_type: &str) -> bool {
        applies_to(&self.spec.record_types, record_type)
    }

    fn required_permissions(&self, _record_type: &str, _config: &Value) -> Vec<Permission> {
        Vec::new()
    }

    fn retry_safe(&self, _config: &Value) -> bool {
        self.spec.retry_safe
    }

    async fn validate(
        &self,
        context: &ValidationContext<'_>,
        config: &Value,
    ) -> Result<Vec<FieldViolation>, AppError> {
        let mut violations = Vec::new();
        let empty = Map::new();
        let values = config.as_object().unwrap_or(&empty);
        for key in values.keys() {
            if !self.spec.inputs.iter().any(|input| &input.key == key) {
                violations.push(context.violation(key, "is not an input of this action"));
            }
        }
        for input in &self.spec.inputs {
            match values.get(&input.key).filter(|value| !value.is_null()) {
                None if input.required => {
                    violations.push(context.violation(&input.key, "is required"));
                }
                Some(value) if !input_valid(input, value) => {
                    violations.push(
                        context.violation(&input.key, format!("must be a valid {}", input.kind)),
                    );
                }
                _ => {}
            }
        }
        Ok(violations)
    }

    async fn execute(
        &self,
        context: &ActionContext<'_>,
        config: &Value,
    ) -> Result<Value, AppError> {
        let plugin = self.plugin()?;
        let call = wit::ActionCall {
            name: self.spec.key.clone(),
            input_json: config.to_string(),
            record_type: context.record.record_type.as_str().to_owned(),
            record_id: context.record.record_id.to_string(),
            record_json: context.record_data.to_string(),
            automation_id: context.automation_id.to_string(),
            run_id: context.run_id.to_string(),
            actor: runtime::scope_actor(context.scope),
        };
        let output = runtime::on_action(&plugin, context.scope, &call).await?;
        Ok(serde_json::from_str(&output).unwrap_or(Value::Null))
    }
}

struct PluginTrigger {
    key: &'static str,
    label: &'static str,
    description: &'static str,
    record_types: Vec<String>,
}

impl PluginTrigger {
    fn new(manifest: &PluginManifest, spec: &EmitSpec) -> Self {
        Self {
            key: leak(&manifest.event_type(&spec.name)),
            label: leak(&spec.label),
            description: leak(spec.description.as_deref().unwrap_or_default()),
            record_types: spec.record_types.clone(),
        }
    }
}

impl AutomationTrigger for PluginTrigger {
    fn key(&self) -> &'static str {
        self.key
    }

    fn label(&self) -> &'static str {
        self.label
    }

    fn description(&self) -> &'static str {
        self.description
    }

    fn supports(&self, record_type: &str) -> bool {
        applies_to(&self.record_types, record_type)
    }

    fn listens_to(&self, event_type: &str) -> bool {
        event_type == self.key
    }

    fn candidates(&self, event: &DomainEvent) -> Vec<TriggerCandidate> {
        let Some(record) = event.record.as_ref() else {
            return Vec::new();
        };
        if !self.supports(record.record_type.as_str()) {
            return Vec::new();
        }
        let context: TriggerContext = event_context(self.key, event);
        vec![TriggerCandidate {
            record: record.clone(),
            context,
        }]
    }
}

fn running_plugins() -> Vec<Arc<LoadedPlugin>> {
    registry::cached()
        .plugins
        .iter()
        .filter(|plugin| plugin.backend.is_some() || plugin.backend_error.is_some())
        .cloned()
        .collect()
}

pub struct PluginActions;

impl AutomationActionSource for PluginActions {
    fn actions(&self) -> Vec<Arc<dyn AutomationAction>> {
        running_plugins()
            .iter()
            .flat_map(|plugin| {
                plugin
                    .manifest
                    .actions
                    .iter()
                    .map(|spec| {
                        Arc::new(PluginAction::new(&plugin.manifest, spec))
                            as Arc<dyn AutomationAction>
                    })
                    .collect::<Vec<_>>()
            })
            .collect()
    }
}

pub struct PluginTriggers;

impl AutomationTriggerSource for PluginTriggers {
    fn triggers(&self) -> Vec<Arc<dyn AutomationTrigger>> {
        running_plugins()
            .iter()
            .flat_map(|plugin| {
                plugin
                    .manifest
                    .emits
                    .iter()
                    .map(|spec| {
                        Arc::new(PluginTrigger::new(&plugin.manifest, spec))
                            as Arc<dyn AutomationTrigger>
                    })
                    .collect::<Vec<_>>()
            })
            .collect()
    }
}
