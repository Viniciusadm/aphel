use aphel_sdk::dashboard::{DashboardReferenceCatalog, WidgetReferenceSetting};
use sqlx::MySqlPool;
use std::collections::{BTreeSet, HashMap};

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
use aphel_kernel::errors::AppError;
use aphel_sdk::configuration::{
    ApplyContext, ConfigPackage, ConfigSection, Decisions, PackageIssue, PlanChange, PlanItem,
    parse_section,
};
use aphel_sdk::platform::database::transaction::MySqlTx;
use aphel_sdk::platform::domain::access::AccessScope;

pub(crate) const DASHBOARD_SECTION: &str = "dashboard";
const DASHBOARD_KIND: &str = "dashboard_layout";

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

pub(crate) struct DashboardSection {
    widgets: HashMap<String, Vec<WidgetReferenceSetting>>,
    layouts: LayoutStore,
    pool: MySqlPool,
}

impl DashboardSection {
    pub(crate) fn new(
        widgets: HashMap<String, Vec<WidgetReferenceSetting>>,
        layouts: LayoutStore,
        pool: MySqlPool,
    ) -> Self {
        Self {
            widgets,
            layouts,
            pool,
        }
    }
    fn catalog(
        kind: aphel_sdk::ReferenceKind,
    ) -> Result<Option<std::sync::Arc<dyn DashboardReferenceCatalog>>, AppError> {
        let catalogs: Vec<_> = aphel_sdk::platform::modules::app()
            .contributions::<dyn DashboardReferenceCatalog>()
            .into_iter()
            .filter(|catalog| catalog.kind() == kind)
            .collect();
        if catalogs.len() > 1 {
            return Err(AppError::Configuration(
                "duplicate dashboard reference catalog".into(),
            ));
        }
        Ok(catalogs.into_iter().next())
    }
    async fn layout_for(
        &self,
        package: &PackageDashboard,
        context: Option<&ApplyContext>,
    ) -> Result<Value, AppError> {
        let mut widgets = Vec::new();
        for (index, widget) in package.widgets.iter().enumerate() {
            let mut settings = widget.settings.clone();
            for reference in self.widgets.get(&widget.key).into_iter().flatten() {
                let Some(package_field) = reference.package_field else {
                    continue;
                };
                if let Some(value) = settings.remove(package_field) {
                    let catalog = Self::catalog(reference.kind)?;
                    let keys = match catalog {
                        Some(catalog) => catalog.keys(&self.pool).await?,
                        None => HashMap::new(),
                    };
                    let id = value
                        .as_str()
                        .and_then(|key| {
                            context
                                .and_then(|context| context.resolved(reference.kind, key))
                                .or_else(|| {
                                    keys.iter()
                                        .find(|(_, stored)| stored.as_str() == key)
                                        .map(|(id, _)| *id)
                                })
                        })
                        .map(|id| id.to_string());
                    settings.insert(reference.field.into(), json!(id));
                }
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
        for widget in &mut dashboard.widgets {
            for reference in self.widgets.get(&widget.key).into_iter().flatten() {
                let Some(value) = widget.settings.remove(reference.field) else {
                    continue;
                };
                if let Some(package_field) = reference.package_field {
                    let keys = match Self::catalog(reference.kind)? {
                        Some(catalog) => catalog.keys(&self.pool).await?,
                        None => HashMap::new(),
                    };
                    let key = value
                        .as_str()
                        .and_then(|id| Uuid::parse_str(id).ok())
                        .and_then(|id| keys.get(&id).cloned());
                    widget.settings.insert(package_field.into(), json!(key));
                } else {
                    widget.settings.insert(reference.field.into(), Value::Null);
                }
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
impl ConfigSection<MySqlTx> for DashboardSection {
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
        for (index, widget) in dashboard.widgets.iter().enumerate() {
            let path = format!("widgets.{index}");
            if !self.widgets.contains_key(&widget.key) {
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
            for reference in self.widgets.get(&widget.key).into_iter().flatten() {
                if widget
                    .settings
                    .get(reference.field)
                    .is_some_and(|value| !value.is_null())
                {
                    issues.push(PackageIssue::new(
                        format!("{path}.settings.{}", reference.field),
                        "not_portable",
                        "use the portable reference key or null in a package",
                    ));
                }
                if let Some(package_field) = reference.package_field {
                    let mut keys = BTreeSet::new();
                    if let Some(catalog) = Self::catalog(reference.kind)? {
                        keys.extend(catalog.package_keys(&package.sections));
                        keys.extend(catalog.keys(&self.pool).await?.into_values());
                    }
                    match widget.settings.get(package_field) {
                        None | Some(Value::Null) => {}
                        Some(Value::String(key)) if keys.contains(key) => {}
                        Some(_) => issues.push(PackageIssue::new(
                            format!("{path}.settings.{package_field}"),
                            "unknown_reference",
                            "must be an available reference key of the package or installation",
                        )),
                    }
                }
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
