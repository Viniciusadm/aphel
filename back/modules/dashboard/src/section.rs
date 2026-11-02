use ::config_packages::application::ConfigReferenceKind;
use std::collections::BTreeSet;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use uuid::Uuid;

use crate::engine::WidgetSize;
use crate::engine_domain::PeriodPreset;
use crate::layout::{
    DashboardLayout, LAYOUT_VERSION, LayoutPeriod, LayoutWidget, MAX_LAYOUT_WIDGETS,
    MAX_WIDGET_SETTINGS_BYTES,
};
use crate::store::LayoutStore;
use ::config_packages::application::{
    ApplyContext, ConfigKeyRepository, ConfigPackage, ConfigSection, Decisions, PackageIssue,
    PlanChange, PlanItem, parse_section,
};
use aphel_kernel::database::transaction::MySqlTx;
use aphel_kernel::domain::access::AccessScope;
use aphel_kernel::errors::AppError;
use pipelines::section::{keyed_pipelines, package_pipeline_keys};
use pipelines::store::PipelineStore;

pub(crate) const DASHBOARD_SECTION: &str = "dashboard";
const DASHBOARD_KIND: &str = "dashboard_layout";
const PIPELINE_SETTING: &str = "pipeline";
const PIPELINE_ID_SETTING: &str = "pipeline_id";
const NON_PORTABLE_SETTINGS: [&str; 1] = ["report_id"];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PackageDashboard {
    pub(crate) widgets: Vec<PackageWidget>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) period: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PackageWidget {
    pub(crate) key: String,
    pub(crate) size: String,
    #[serde(default, skip_serializing_if = "Map::is_empty")]
    pub(crate) settings: Map<String, Value>,
}

pub(crate) struct DashboardSection<K> {
    widget_keys: Vec<String>,
    layouts: LayoutStore,
    pipelines: PipelineStore,
    keys: K,
}

impl<K> DashboardSection<K>
where
    K: ConfigKeyRepository,
{
    pub(crate) fn new(
        widget_keys: Vec<String>,
        layouts: LayoutStore,
        pipelines: PipelineStore,
        keys: K,
    ) -> Self {
        Self {
            widget_keys,
            layouts,
            pipelines,
            keys,
        }
    }

    async fn layout_for(
        &self,
        package: &PackageDashboard,
        context: Option<&ApplyContext>,
    ) -> Result<Value, AppError> {
        let existing = keyed_pipelines(&self.pipelines, &self.keys).await?;
        let mut widgets = Vec::new();
        for (index, widget) in package.widgets.iter().enumerate() {
            let mut settings = widget.settings.clone();
            if let Some(pipeline) = settings.remove(PIPELINE_SETTING) {
                let pipeline_id = pipeline.as_str().and_then(|key| {
                    context
                        .and_then(|context| {
                            context.resolved(ConfigReferenceKind::new("workflow.pipeline"), key)
                        })
                        .or_else(|| {
                            existing
                                .iter()
                                .find(|(_, keyed)| keyed.key == key)
                                .map(|(pipeline, _)| pipeline.id)
                        })
                        .map(|id| id.to_string())
                });
                settings.insert(PIPELINE_ID_SETTING.to_owned(), json!(pipeline_id));
            }
            widgets.push(LayoutWidget {
                id: format!(
                    "widget-{}-{}",
                    index + 1,
                    widget.key.replace(['_', '.'], "-")
                ),
                key: widget.key.clone(),
                size: widget.size.clone(),
                settings: Value::Object(settings),
            });
        }
        let period = LayoutPeriod {
            preset: package
                .period
                .clone()
                .unwrap_or_else(|| LayoutPeriod::default().preset),
            from: None,
            to: None,
        };
        let mut layout = serde_json::to_value(DashboardLayout {
            widgets,
            period,
            subject: None,
            compare_to_previous: false,
        })
        .map_err(|error| AppError::Internal(error.to_string()))?;
        if let Value::Object(map) = &mut layout {
            map.insert("version".to_owned(), Value::from(LAYOUT_VERSION));
        }
        Ok(layout)
    }

    async fn to_package(&self, stored: &Value) -> Result<Option<PackageDashboard>, AppError> {
        let Ok(mut dashboard) = serde_json::from_value::<PackageDashboard>(export_value(stored))
        else {
            return Ok(None);
        };
        let existing = keyed_pipelines(&self.pipelines, &self.keys).await?;
        for widget in &mut dashboard.widgets {
            for setting in NON_PORTABLE_SETTINGS {
                if widget.settings.contains_key(setting) {
                    widget.settings.insert(setting.to_owned(), Value::Null);
                }
            }
            if let Some(pipeline_id) = widget.settings.remove(PIPELINE_ID_SETTING) {
                let key = pipeline_id
                    .as_str()
                    .and_then(|id| Uuid::parse_str(id).ok())
                    .and_then(|id| {
                        existing
                            .iter()
                            .find(|(pipeline, _)| pipeline.id == id)
                            .map(|(_, keyed)| keyed.key.clone())
                    });
                widget
                    .settings
                    .insert(PIPELINE_SETTING.to_owned(), json!(key));
            }
        }
        Ok(Some(dashboard))
    }

    async fn reconcile(
        &self,
        package: &PackageDashboard,
        context: Option<&ApplyContext>,
    ) -> Result<(PlanItem, Value), AppError> {
        let proposed = self.layout_for(package, context).await?;
        let item = PlanItem::new(DASHBOARD_SECTION, DASHBOARD_KIND, "default", "Dashboard");
        let widget_keys = |dashboard: &PackageDashboard| {
            json!(
                dashboard
                    .widgets
                    .iter()
                    .map(|widget| widget.key.clone())
                    .collect::<Vec<_>>()
            )
        };
        let normalized = normalize(package.clone());
        let item = match self.layouts.find_default().await? {
            None => item.created(vec![PlanChange::new(
                "widgets",
                Value::Null,
                widget_keys(&normalized),
            )]),
            Some(stored) => match self.to_package(&stored).await?.map(normalize) {
                Some(current) if current == normalized => item.compared(Vec::new()),
                current => {
                    let mut changes = Vec::new();
                    let current_widgets = current.as_ref().map_or(Value::Null, widget_keys);
                    let proposed_widgets = widget_keys(&normalized);
                    changes.push(PlanChange::new(
                        "widgets",
                        current_widgets,
                        proposed_widgets,
                    ));
                    if current.as_ref().map(|current| &current.period) != Some(&normalized.period) {
                        changes.push(PlanChange::new(
                            "period",
                            json!(current.and_then(|current| current.period)),
                            json!(normalized.period),
                        ));
                    }
                    item.compared(changes)
                }
            },
        };
        Ok((item, proposed))
    }
}

fn normalize(mut dashboard: PackageDashboard) -> PackageDashboard {
    if dashboard.period.is_none() {
        dashboard.period = Some(LayoutPeriod::default().preset);
    }
    for widget in &mut dashboard.widgets {
        if widget.settings.get(PIPELINE_SETTING) == Some(&Value::Null) {
            widget.settings.remove(PIPELINE_SETTING);
        }
    }
    dashboard
}

fn export_value(layout: &Value) -> Value {
    let Ok(layout) = serde_json::from_value::<DashboardLayout>(layout.clone()) else {
        return Value::Null;
    };
    json!(PackageDashboard {
        widgets: layout
            .widgets
            .into_iter()
            .map(|widget| PackageWidget {
                key: widget.key,
                size: widget.size,
                settings: widget.settings.as_object().cloned().unwrap_or_default(),
            })
            .collect(),
        period: (layout.period.preset != PeriodPreset::Custom.as_str())
            .then_some(layout.period.preset),
    })
}

fn parse(content: &Value) -> Result<PackageDashboard, AppError> {
    parse_section(content).map_err(|issue| AppError::Validation(issue.message))
}

#[async_trait]
impl<K> ConfigSection<MySqlTx> for DashboardSection<K>
where
    K: ConfigKeyRepository,
{
    fn key(&self) -> &'static str {
        DASHBOARD_SECTION
    }

    async fn export(&self, _scope: &AccessScope) -> Result<Option<Value>, AppError> {
        let Some(stored) = self.layouts.find_default().await? else {
            return Ok(None);
        };
        Ok(self
            .to_package(&stored)
            .await?
            .map(|dashboard| json!(dashboard)))
    }

    async fn validate(
        &self,
        _scope: &AccessScope,
        package: &ConfigPackage,
        content: &Value,
    ) -> Result<Vec<PackageIssue>, AppError> {
        let dashboard: PackageDashboard = match parse_section(content) {
            Ok(dashboard) => dashboard,
            Err(issue) => return Ok(vec![issue]),
        };
        let mut issues = Vec::new();
        if dashboard.widgets.len() > MAX_LAYOUT_WIDGETS {
            issues.push(PackageIssue::new(
                "widgets",
                "too_many_widgets",
                format!("at most {MAX_LAYOUT_WIDGETS} widgets"),
            ));
        }
        if let Some(period) = &dashboard.period
            && PeriodPreset::parse(period).is_none_or(|preset| preset == PeriodPreset::Custom)
        {
            issues.push(PackageIssue::new(
                "period",
                "invalid_period",
                "must be today, last_7_days, month or quarter",
            ));
        }
        let mut pipeline_keys: BTreeSet<String> =
            package_pipeline_keys(package).into_iter().collect();
        pipeline_keys.extend(
            keyed_pipelines(&self.pipelines, &self.keys)
                .await?
                .into_iter()
                .map(|(_, keyed)| keyed.key),
        );
        for (index, widget) in dashboard.widgets.iter().enumerate() {
            let path = format!("widgets.{index}");
            if !self.widget_keys.contains(&widget.key) {
                issues.push(PackageIssue::new(
                    format!("{path}.key"),
                    "unknown_widget",
                    "is not a widget known by this Aphel",
                ));
            }
            if WidgetSize::parse(&widget.size).is_none() {
                issues.push(PackageIssue::new(
                    format!("{path}.size"),
                    "invalid_size",
                    "must be small, medium or large",
                ));
            }
            if Value::Object(widget.settings.clone()).to_string().len() > MAX_WIDGET_SETTINGS_BYTES
            {
                issues.push(PackageIssue::new(
                    format!("{path}.settings"),
                    "settings_too_large",
                    format!("must have at most {MAX_WIDGET_SETTINGS_BYTES} bytes"),
                ));
            }
            if widget.settings.contains_key(PIPELINE_ID_SETTING) {
                issues.push(PackageIssue::new(
                    format!("{path}.settings.{PIPELINE_ID_SETTING}"),
                    "not_portable",
                    "reference the pipeline by its key in settings.pipeline",
                ));
            }
            for setting in NON_PORTABLE_SETTINGS {
                if widget
                    .settings
                    .get(setting)
                    .is_some_and(|value| !value.is_null())
                {
                    issues.push(PackageIssue::new(
                        format!("{path}.settings.{setting}"),
                        "not_portable",
                        "references a record of the organization and must be null in a package",
                    ));
                }
            }
            match widget.settings.get(PIPELINE_SETTING) {
                None | Some(Value::Null) => {}
                Some(Value::String(key)) if pipeline_keys.contains(key) => {}
                Some(_) => issues.push(PackageIssue::new(
                    format!("{path}.settings.{PIPELINE_SETTING}"),
                    "unknown_pipeline",
                    "must be the key of a pipeline of the package or of the organization",
                )),
            }
        }
        Ok(issues)
    }

    async fn plan(
        &self,
        _scope: &AccessScope,
        _package: &ConfigPackage,
        content: &Value,
    ) -> Result<Vec<PlanItem>, AppError> {
        let (item, _) = self.reconcile(&parse(content)?, None).await?;
        Ok(vec![item])
    }

    async fn apply(
        &self,
        tx: &mut MySqlTx,
        _scope: &AccessScope,
        _package: &ConfigPackage,
        content: &Value,
        decisions: &Decisions,
        context: &mut ApplyContext,
    ) -> Result<(), AppError> {
        let (item, layout) = self.reconcile(&parse(content)?, Some(context)).await?;
        if decisions.executes(&item) {
            self.layouts.save_default(tx, &layout, context.at).await?;
        }
        Ok(())
    }
}
