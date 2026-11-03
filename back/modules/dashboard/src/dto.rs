pub use aphel_sdk::dashboard::dto::{DashboardPeriodResponse, PeriodPresetSchema};
use chrono::{DateTime, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::{IntoParams, ToSchema};

use crate::engine::{
    LayoutSubject, PeriodQuery, SubjectKind, SubjectOptions, WidgetDescriptor, WidgetSize,
    WidgetSubject,
};
use crate::layout::{
    DashboardLayout, LayoutPeriod, LayoutView, LayoutWidget, SubjectQuery, WidgetDataQuery,
};
use crate::service::WidgetData;
use aphel_kernel::domain::access::RecordScope;
use aphel_kernel::errors::AppError;

fn instant(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(SecondsFormat::Secs, true)
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[schema(as = DashboardWidgetSize)]
pub(crate) enum WidgetSizeSchema {
    Small,
    Medium,
    Large,
}

impl From<WidgetSize> for WidgetSizeSchema {
    fn from(size: WidgetSize) -> Self {
        match size {
            WidgetSize::Small => Self::Small,
            WidgetSize::Medium => Self::Medium,
            WidgetSize::Large => Self::Large,
        }
    }
}

impl From<WidgetSizeSchema> for WidgetSize {
    fn from(size: WidgetSizeSchema) -> Self {
        match size {
            WidgetSizeSchema::Small => Self::Small,
            WidgetSizeSchema::Medium => Self::Medium,
            WidgetSizeSchema::Large => Self::Large,
        }
    }
}

impl WidgetSizeSchema {
    fn stored(value: &str) -> Self {
        WidgetSize::parse(value)
            .unwrap_or(WidgetSize::Medium)
            .into()
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct WidgetDescriptorResponse {
    #[schema(example = "sales_indicators")]
    pub(crate) key: String,
    #[schema(example = "Sales indicators")]
    pub(crate) title: String,
    #[schema(example = "sales")]
    pub(crate) category: String,
    pub(crate) required_permissions: Vec<String>,
    pub(crate) default_size: WidgetSizeSchema,
    pub(crate) uses_period: bool,
    #[schema(value_type = Object)]
    pub(crate) default_settings: Value,
}

impl From<WidgetDescriptor> for WidgetDescriptorResponse {
    fn from(descriptor: WidgetDescriptor) -> Self {
        Self {
            key: descriptor.key,
            title: descriptor.title,
            category: descriptor.category,
            required_permissions: descriptor
                .required_permissions
                .iter()
                .map(|permission| permission.as_str().to_owned())
                .collect(),
            default_size: descriptor.default_size.into(),
            uses_period: descriptor.uses_period,
            default_settings: descriptor.default_settings,
        }
    }
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct WidgetDataParams {
    #[param(example = "month")]
    pub(crate) period: Option<PeriodPresetSchema>,
    #[param(example = "2026-01-01")]
    pub(crate) from: Option<String>,
    #[param(example = "2026-01-31")]
    pub(crate) to: Option<String>,
    #[param(example = "{\"stale_days\":14}")]
    pub(crate) settings: Option<String>,
    #[param(example = "0190a1b2-0000-7000-8000-000000000001")]
    pub(crate) user_id: Option<String>,
    #[param(example = "0190a1b2-0000-7000-8000-000000000002")]
    pub(crate) team_id: Option<String>,
    pub(crate) compare_to_previous: Option<bool>,
}

impl WidgetDataParams {
    pub(crate) fn into_query(self) -> Result<WidgetDataQuery, AppError> {
        let settings = match self
            .settings
            .as_deref()
            .filter(|value| !value.trim().is_empty())
        {
            None => None,
            Some(text) => Some(serde_json::from_str::<Value>(text).map_err(|_| {
                AppError::invalid_fields(
                    "invalid widget settings",
                    vec![aphel_kernel::errors::FieldViolation::new(
                        "settings",
                        "must be a JSON object",
                    )],
                )
            })?),
        };
        Ok(WidgetDataQuery {
            period: PeriodQuery {
                period: self.period.map(|preset| preset.as_str().to_owned()),
                from: self.from,
                to: self.to,
            },
            settings,
            subject: SubjectQuery {
                user_id: self.user_id,
                team_id: self.team_id,
            },
            compare_to_previous: self.compare_to_previous.unwrap_or(false),
        })
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[schema(as = DashboardSubjectType)]
pub(crate) enum SubjectKindSchema {
    User,
    Team,
}

impl From<SubjectKind> for SubjectKindSchema {
    fn from(kind: SubjectKind) -> Self {
        match kind {
            SubjectKind::User => Self::User,
            SubjectKind::Team => Self::Team,
        }
    }
}

impl From<SubjectKindSchema> for SubjectKind {
    fn from(kind: SubjectKindSchema) -> Self {
        match kind {
            SubjectKindSchema::User => Self::User,
            SubjectKindSchema::Team => Self::Team,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub(crate) struct DashboardSubject {
    #[serde(rename = "type")]
    pub(crate) kind: SubjectKindSchema,
    #[schema(example = "0190a1b2-0000-7000-8000-000000000001")]
    pub(crate) id: String,
}

impl From<LayoutSubject> for DashboardSubject {
    fn from(subject: LayoutSubject) -> Self {
        Self {
            kind: subject.kind.into(),
            id: subject.id,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct WidgetSubjectResponse {
    #[serde(rename = "type")]
    pub(crate) kind: SubjectKindSchema,
    pub(crate) id: String,
    pub(crate) name: String,
}

impl From<WidgetSubject> for WidgetSubjectResponse {
    fn from(subject: WidgetSubject) -> Self {
        Self {
            kind: subject.kind.into(),
            id: subject.id,
            name: subject.name,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct WidgetDataResponse {
    pub(crate) key: String,
    pub(crate) period: Option<DashboardPeriodResponse>,
    pub(crate) previous_period: Option<DashboardPeriodResponse>,
    pub(crate) subject: Option<WidgetSubjectResponse>,
    #[schema(value_type = Object)]
    pub(crate) settings: Value,
    pub(crate) generated_at: String,
    #[schema(value_type = Object)]
    pub(crate) data: Value,
}

impl From<WidgetData> for WidgetDataResponse {
    fn from(data: WidgetData) -> Self {
        Self {
            key: data.key,
            period: data.period.map(Into::into),
            previous_period: data.previous_period.map(Into::into),
            subject: data.subject.map(Into::into),
            settings: data.settings,
            generated_at: instant(data.generated_at),
            data: data.data,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub(crate) struct DashboardLayoutPeriod {
    pub(crate) preset: PeriodPresetSchema,
    #[serde(default)]
    #[schema(example = "2026-01-01")]
    pub(crate) from: Option<String>,
    #[serde(default)]
    #[schema(example = "2026-01-31")]
    pub(crate) to: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub(crate) struct DashboardLayoutWidget {
    #[schema(example = "w-1")]
    pub(crate) id: String,
    #[schema(example = "sales_indicators")]
    pub(crate) key: String,
    pub(crate) size: WidgetSizeSchema,
    #[serde(default)]
    #[schema(value_type = Object)]
    pub(crate) settings: Value,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct SaveDashboardLayoutRequest {
    pub(crate) widgets: Vec<DashboardLayoutWidget>,
    pub(crate) period: DashboardLayoutPeriod,
    #[serde(default)]
    pub(crate) subject: Option<DashboardSubject>,
    #[serde(default)]
    pub(crate) compare_to_previous: bool,
}

impl SaveDashboardLayoutRequest {
    pub(crate) fn into_layout(self) -> DashboardLayout {
        let subject = match self.subject {
            None => None,
            Some(subject) => Some(LayoutSubject {
                kind: subject.kind.into(),
                id: subject.id,
            }),
        };
        DashboardLayout {
            widgets: self
                .widgets
                .into_iter()
                .map(|widget| LayoutWidget {
                    id: widget.id,
                    key: widget.key,
                    size: WidgetSize::from(widget.size).as_str().to_owned(),
                    settings: widget.settings,
                })
                .collect(),
            period: LayoutPeriod {
                preset: self.period.preset.as_str().to_owned(),
                from: self.period.from,
                to: self.period.to,
            },
            subject,
            compare_to_previous: self.compare_to_previous,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct DashboardLayoutResponse {
    pub(crate) widgets: Vec<DashboardLayoutWidget>,
    pub(crate) period: DashboardLayoutPeriod,
    pub(crate) subject: Option<DashboardSubject>,
    pub(crate) compare_to_previous: bool,
    pub(crate) is_default: bool,
}

impl From<LayoutView> for DashboardLayoutResponse {
    fn from(view: LayoutView) -> Self {
        Self {
            widgets: view
                .layout
                .widgets
                .into_iter()
                .map(|widget| DashboardLayoutWidget {
                    id: widget.id,
                    key: widget.key,
                    size: WidgetSizeSchema::stored(&widget.size),
                    settings: widget.settings,
                })
                .collect(),
            period: DashboardLayoutPeriod {
                preset: PeriodPresetSchema::stored(&view.layout.period.preset),
                from: view.layout.period.from,
                to: view.layout.period.to,
            },
            subject: view.layout.subject.map(Into::into),
            compare_to_previous: view.layout.compare_to_previous,
            is_default: view.is_default,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct DashboardSubjectUser {
    pub(crate) id: String,
    pub(crate) display_name: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct DashboardSubjectTeam {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) member_count: usize,
}

#[derive(Debug, Clone, Copy, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[schema(as = DashboardSubjectScope)]
pub(crate) enum SubjectScopeSchema {
    Own,
    Team,
    Organization,
}

impl From<RecordScope> for SubjectScopeSchema {
    fn from(scope: RecordScope) -> Self {
        match scope {
            RecordScope::Own => Self::Own,
            RecordScope::Team => Self::Team,
            RecordScope::Organization => Self::Organization,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct DashboardSubjectsResponse {
    pub(crate) scope: SubjectScopeSchema,
    pub(crate) users: Vec<DashboardSubjectUser>,
    pub(crate) teams: Vec<DashboardSubjectTeam>,
}

impl From<SubjectOptions> for DashboardSubjectsResponse {
    fn from(options: SubjectOptions) -> Self {
        Self {
            scope: options.scope.into(),
            users: options
                .users
                .into_iter()
                .map(|user| DashboardSubjectUser {
                    id: user.id.into_inner(),
                    display_name: user.display_name,
                })
                .collect(),
            teams: options
                .teams
                .into_iter()
                .map(|team| DashboardSubjectTeam {
                    member_count: team.member_ids.len(),
                    id: team.id.into_inner(),
                    name: team.name,
                })
                .collect(),
        }
    }
}
