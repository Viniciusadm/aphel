use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::engine::{LayoutSubject, PeriodQuery};
use crate::engine_domain::PeriodSelection;

pub(crate) const LAYOUT_VERSION: u32 = 1;
pub(crate) const MAX_LAYOUT_WIDGETS: usize = 24;
pub(crate) const MAX_WIDGET_ID_LENGTH: usize = 64;
pub(crate) const MAX_WIDGET_SETTINGS_BYTES: usize = 2048;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct LayoutPeriod {
    pub(crate) preset: String,
    #[serde(default)]
    pub(crate) from: Option<String>,
    #[serde(default)]
    pub(crate) to: Option<String>,
}

impl Default for LayoutPeriod {
    fn default() -> Self {
        Self::from_selection(&PeriodSelection::default())
    }
}

impl LayoutPeriod {
    pub(crate) fn from_selection(selection: &PeriodSelection) -> Self {
        Self {
            preset: selection.preset.as_str().to_owned(),
            from: selection.from.map(|day| day.format("%Y-%m-%d").to_string()),
            to: selection.to.map(|day| day.format("%Y-%m-%d").to_string()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct LayoutWidget {
    pub(crate) id: String,
    pub(crate) key: String,
    pub(crate) size: String,
    #[serde(default)]
    pub(crate) settings: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct DashboardLayout {
    #[serde(default)]
    pub(crate) widgets: Vec<LayoutWidget>,
    #[serde(default)]
    pub(crate) period: LayoutPeriod,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) subject: Option<LayoutSubject>,
    #[serde(default)]
    pub(crate) compare_to_previous: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct LayoutView {
    pub(crate) layout: DashboardLayout,
    pub(crate) is_default: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct SubjectQuery {
    pub(crate) user_id: Option<String>,
    pub(crate) team_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct WidgetDataQuery {
    pub(crate) period: PeriodQuery,
    pub(crate) settings: Option<Value>,
    pub(crate) subject: SubjectQuery,
    pub(crate) compare_to_previous: bool,
}
