use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;

use crate::manifest::WidgetSpec;
use crate::registry::LoadedPlugin;
use crate::runtime::{self, scope_actor, wit};
use aphel_kernel::domain::access::Permission;
use aphel_kernel::errors::{AppError, FieldViolation};
use aphel_sdk::dashboard::DashboardWidget;
use aphel_sdk::dashboard::WidgetPorts;
use aphel_sdk::dashboard::{WidgetContext, WidgetProvider, WidgetSize};

const DEFAULT_CATEGORY: &str = "plugins";
const MAX_SETTINGS_BYTES: usize = 16 * 1024;

pub fn widget_key(plugin: &str, name: &str) -> String {
    format!("{plugin}_{name}")
}

pub struct PluginWidget {
    plugin: Arc<LoadedPlugin>,
    name: String,
    key: String,
    title: String,
    category: String,
    permissions: Vec<Permission>,
    size: WidgetSize,
    uses_period: bool,
    default_settings: Value,
}

impl PluginWidget {
    pub fn shared(plugin: &Arc<LoadedPlugin>, spec: &WidgetSpec) -> DashboardWidget {
        Arc::new(Self {
            plugin: plugin.clone(),
            name: spec.name.clone(),
            key: widget_key(&plugin.key, &spec.name),
            title: spec.title.clone(),
            category: if spec.category.is_empty() {
                DEFAULT_CATEGORY.to_owned()
            } else {
                spec.category.clone()
            },
            permissions: spec
                .permission
                .iter()
                .filter_map(|permission| Permission::parse(permission.clone()).ok())
                .collect(),
            size: WidgetSize::parse(&spec.size).unwrap_or(WidgetSize::Medium),
            uses_period: spec.uses_period,
            default_settings: spec.default_settings.clone(),
        })
    }
}

#[async_trait]
impl WidgetProvider<WidgetPorts> for PluginWidget {
    fn key(&self) -> &str {
        &self.key
    }

    fn title(&self) -> &str {
        &self.title
    }

    fn category(&self) -> &str {
        &self.category
    }

    fn required_permissions(&self) -> Vec<Permission> {
        self.permissions.clone()
    }

    fn default_size(&self) -> WidgetSize {
        self.size
    }

    fn uses_period(&self) -> bool {
        self.uses_period
    }

    fn default_settings(&self) -> Value {
        self.default_settings.clone()
    }

    fn normalize_settings(&self, settings: &Value) -> Result<Value, Vec<FieldViolation>> {
        let too_large = serde_json::to_vec(settings)
            .map(|bytes| bytes.len() > MAX_SETTINGS_BYTES)
            .unwrap_or(true);
        match settings {
            Value::Object(_) if !too_large => Ok(settings.clone()),
            Value::Null => Ok(self.default_settings.clone()),
            _ => Err(vec![FieldViolation::new(
                "settings",
                "must be an object of at most 16 KB",
            )]),
        }
    }

    async fn data(
        &self,
        context: &WidgetContext<'_, WidgetPorts>,
        settings: &Value,
    ) -> Result<Value, AppError> {
        let request = wit::WidgetRequest {
            settings_json: settings.to_string(),
            period_from: context.period.starts_at.to_rfc3339(),
            period_to: context.period.ends_at.to_rfc3339(),
            timezone: context.timezone.name().to_owned(),
            actor: scope_actor(context.scope),
        };
        let json = runtime::on_widget(&self.plugin, context.scope, &self.name, &request).await?;
        Ok(serde_json::from_str(&json).unwrap_or(Value::Null))
    }
}
