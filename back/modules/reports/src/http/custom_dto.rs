use chrono::{DateTime, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use utoipa::{IntoParams, ToSchema};

use crate::domain::custom::{
    CreateReportCommand, CreateScheduleCommand, DeliveryEntry, DeliveryStatus, ReportEntry,
    ReportVisualization, RunOverrides, ScheduleEntry, ScheduleFrequency, UpdateReportCommand,
    UpdateScheduleCommand,
};
use aphel_kernel::api::dto::aggregates::AggregateQueryRequest;
use aphel_kernel::api::dto::common::double_option;
use aphel_kernel::api::dto::files::UserReferenceResponse;
use aphel_kernel::api::dto::views::ViewVisibilitySchema;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[schema(as = ReportVisualization)]
pub(crate) enum ReportVisualizationSchema {
    Table,
    Bar,
    Line,
    Pie,
    Number,
}

impl From<ReportVisualization> for ReportVisualizationSchema {
    fn from(visualization: ReportVisualization) -> Self {
        match visualization {
            ReportVisualization::Table => Self::Table,
            ReportVisualization::Bar => Self::Bar,
            ReportVisualization::Line => Self::Line,
            ReportVisualization::Pie => Self::Pie,
            ReportVisualization::Number => Self::Number,
        }
    }
}

impl From<ReportVisualizationSchema> for ReportVisualization {
    fn from(visualization: ReportVisualizationSchema) -> Self {
        match visualization {
            ReportVisualizationSchema::Table => Self::Table,
            ReportVisualizationSchema::Bar => Self::Bar,
            ReportVisualizationSchema::Line => Self::Line,
            ReportVisualizationSchema::Pie => Self::Pie,
            ReportVisualizationSchema::Number => Self::Number,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[schema(as = ReportScheduleFrequency)]
pub(crate) enum ScheduleFrequencySchema {
    Daily,
    Weekly,
    Monthly,
}

impl From<ScheduleFrequency> for ScheduleFrequencySchema {
    fn from(frequency: ScheduleFrequency) -> Self {
        match frequency {
            ScheduleFrequency::Daily => Self::Daily,
            ScheduleFrequency::Weekly => Self::Weekly,
            ScheduleFrequency::Monthly => Self::Monthly,
        }
    }
}

impl From<ScheduleFrequencySchema> for ScheduleFrequency {
    fn from(frequency: ScheduleFrequencySchema) -> Self {
        match frequency {
            ScheduleFrequencySchema::Daily => Self::Daily,
            ScheduleFrequencySchema::Weekly => Self::Weekly,
            ScheduleFrequencySchema::Monthly => Self::Monthly,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[schema(as = ReportDeliveryStatus)]
pub(crate) enum DeliveryStatusSchema {
    Sending,
    Sent,
    Failed,
}

impl From<DeliveryStatus> for DeliveryStatusSchema {
    fn from(status: DeliveryStatus) -> Self {
        match status {
            DeliveryStatus::Sending => Self::Sending,
            DeliveryStatus::Sent => Self::Sent,
            DeliveryStatus::Failed => Self::Failed,
        }
    }
}

fn timestamp(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(SecondsFormat::Secs, true)
}

fn private_visibility() -> String {
    "private".to_owned()
}

fn table_visualization() -> ReportVisualizationSchema {
    ReportVisualizationSchema::Table
}

fn active() -> bool {
    true
}

#[derive(Debug, Default, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct ListReportsParams {
    #[param(example = "deal")]
    pub(crate) record_type: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub(crate) struct CreateReportRequest {
    #[schema(example = "Won amount per month by sales rep")]
    pub(crate) name: String,
    pub(crate) description: Option<String>,
    #[serde(default = "private_visibility")]
    #[schema(value_type = Option<ViewVisibilitySchema>)]
    pub(crate) visibility: String,
    pub(crate) team_id: Option<String>,
    #[serde(default = "table_visualization")]
    #[schema(value_type = Option<ReportVisualizationSchema>)]
    pub(crate) visualization: ReportVisualizationSchema,
    pub(crate) definition: AggregateQueryRequest,
    #[serde(default)]
    #[schema(value_type = Option<Object>)]
    pub(crate) settings: Value,
}

impl CreateReportRequest {
    pub(crate) fn into_command(self, ip: Option<String>) -> CreateReportCommand {
        CreateReportCommand {
            name: self.name,
            description: self.description,
            visibility: self.visibility,
            team_id: self.team_id,
            visualization: self.visualization.into(),
            definition: self.definition.into_query(),
            settings: self.settings,
            ip,
        }
    }
}

#[derive(Debug, Default, Deserialize, ToSchema)]
pub(crate) struct UpdateReportRequest {
    pub(crate) name: Option<String>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>)]
    pub(crate) description: Option<Option<String>>,
    #[schema(value_type = Option<ViewVisibilitySchema>)]
    pub(crate) visibility: Option<String>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>)]
    pub(crate) team_id: Option<Option<String>>,
    pub(crate) visualization: Option<ReportVisualizationSchema>,
    pub(crate) definition: Option<AggregateQueryRequest>,
    #[schema(value_type = Option<Object>)]
    pub(crate) settings: Option<Value>,
}

impl UpdateReportRequest {
    pub(crate) fn into_command(
        self,
        expected_updated_at: Option<DateTime<Utc>>,
        ip: Option<String>,
    ) -> UpdateReportCommand {
        UpdateReportCommand {
            name: self.name,
            description: self.description,
            visibility: self.visibility,
            team_id: self.team_id,
            visualization: self.visualization.map(Into::into),
            definition: self.definition.map(AggregateQueryRequest::into_query),
            settings: self.settings,
            expected_updated_at,
            ip,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct ReportResponse {
    pub(crate) id: String,
    #[schema(example = "deal")]
    pub(crate) record_type: String,
    pub(crate) name: String,
    pub(crate) description: Option<String>,
    pub(crate) owner_id: Option<String>,
    #[schema(value_type = ViewVisibilitySchema)]
    pub(crate) visibility: String,
    pub(crate) team_id: Option<String>,
    pub(crate) visualization: ReportVisualizationSchema,
    pub(crate) definition: AggregateQueryRequest,
    #[schema(value_type = Object)]
    pub(crate) settings: Map<String, Value>,
    pub(crate) can_edit: bool,
    #[schema(example = "2026-10-20T13:45:00Z")]
    pub(crate) created_at: String,
    #[schema(example = "2026-10-20T13:45:00Z")]
    pub(crate) updated_at: String,
}

impl From<ReportEntry> for ReportResponse {
    fn from(entry: ReportEntry) -> Self {
        let report = entry.report;
        Self {
            id: report.id.to_string(),
            record_type: report.record_type.as_str().to_owned(),
            name: report.name,
            description: report.description,
            owner_id: report.owner_id.map(|id| id.into_inner()),
            visibility: report.visibility.as_str().to_owned(),
            team_id: report.team_id.map(|id| id.into_inner()),
            visualization: report.visualization.into(),
            definition: report.definition.into(),
            settings: report.settings,
            can_edit: entry.can_edit,
            created_at: timestamp(report.created_at),
            updated_at: timestamp(report.updated_at),
        }
    }
}

#[derive(Debug, Default, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct RunReportRequest {
    #[schema(example = "last_month")]
    pub(crate) preset: Option<String>,
    #[schema(example = "2026-01-01")]
    pub(crate) from: Option<String>,
    #[schema(example = "2026-03-31")]
    pub(crate) to: Option<String>,
    pub(crate) compare_to_previous: Option<bool>,
}

impl From<RunReportRequest> for RunOverrides {
    fn from(request: RunReportRequest) -> Self {
        Self {
            preset: request.preset,
            from: request.from,
            to: request.to,
            compare_to_previous: request.compare_to_previous,
        }
    }
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct ExportReportParams {
    #[param(example = "xlsx")]
    pub(crate) format: String,
    #[param(example = "last_month")]
    pub(crate) preset: Option<String>,
    #[param(example = "2026-01-01")]
    pub(crate) from: Option<String>,
    #[param(example = "2026-03-31")]
    pub(crate) to: Option<String>,
    pub(crate) compare_to_previous: Option<bool>,
}

impl ExportReportParams {
    pub(crate) fn overrides(&self) -> RunOverrides {
        RunOverrides {
            preset: self.preset.clone(),
            from: self.from.clone(),
            to: self.to.clone(),
            compare_to_previous: self.compare_to_previous,
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub(crate) struct CreateScheduleRequest {
    pub(crate) frequency: ScheduleFrequencySchema,
    #[schema(example = 1)]
    pub(crate) weekday: Option<u8>,
    #[schema(example = 1)]
    pub(crate) day_of_month: Option<u8>,
    #[schema(example = 8)]
    pub(crate) hour: u8,
    #[serde(default)]
    #[schema(example = 0)]
    pub(crate) minute: u8,
    #[schema(example = "last_month")]
    pub(crate) period_preset: Option<String>,
    pub(crate) recipients: Vec<String>,
    #[serde(default = "active")]
    pub(crate) is_active: bool,
}

impl CreateScheduleRequest {
    pub(crate) fn into_command(self, ip: Option<String>) -> CreateScheduleCommand {
        CreateScheduleCommand {
            frequency: self.frequency.into(),
            weekday: self.weekday,
            day_of_month: self.day_of_month,
            hour: self.hour,
            minute: self.minute,
            period_preset: self.period_preset,
            recipients: self.recipients,
            is_active: self.is_active,
            ip,
        }
    }
}

#[derive(Debug, Default, Deserialize, ToSchema)]
pub(crate) struct UpdateScheduleRequest {
    pub(crate) frequency: Option<ScheduleFrequencySchema>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<u8>)]
    pub(crate) weekday: Option<Option<u8>>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<u8>)]
    pub(crate) day_of_month: Option<Option<u8>>,
    pub(crate) hour: Option<u8>,
    pub(crate) minute: Option<u8>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>)]
    pub(crate) period_preset: Option<Option<String>>,
    pub(crate) recipients: Option<Vec<String>>,
    pub(crate) is_active: Option<bool>,
}

impl UpdateScheduleRequest {
    pub(crate) fn into_command(
        self,
        expected_updated_at: Option<DateTime<Utc>>,
        ip: Option<String>,
    ) -> UpdateScheduleCommand {
        UpdateScheduleCommand {
            frequency: self.frequency.map(Into::into),
            weekday: self.weekday,
            day_of_month: self.day_of_month,
            hour: self.hour,
            minute: self.minute,
            period_preset: self.period_preset,
            recipients: self.recipients,
            is_active: self.is_active,
            expected_updated_at,
            ip,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct ReportScheduleResponse {
    pub(crate) id: String,
    pub(crate) report_id: String,
    pub(crate) created_by: Option<String>,
    pub(crate) frequency: ScheduleFrequencySchema,
    pub(crate) weekday: Option<u8>,
    pub(crate) day_of_month: Option<u8>,
    pub(crate) hour: u8,
    pub(crate) minute: u8,
    pub(crate) period_preset: Option<String>,
    pub(crate) recipients: Vec<UserReferenceResponse>,
    pub(crate) is_active: bool,
    #[schema(example = "2026-11-01T11:00:00Z")]
    pub(crate) next_run_at: String,
    pub(crate) last_run_at: Option<String>,
    #[schema(example = "2026-10-20T13:45:00Z")]
    pub(crate) created_at: String,
    #[schema(example = "2026-10-20T13:45:00Z")]
    pub(crate) updated_at: String,
}

impl From<ScheduleEntry> for ReportScheduleResponse {
    fn from(entry: ScheduleEntry) -> Self {
        let schedule = entry.schedule;
        Self {
            id: schedule.id.to_string(),
            report_id: schedule.report_id.to_string(),
            created_by: schedule.created_by.map(|id| id.into_inner()),
            frequency: schedule.timing.frequency.into(),
            weekday: schedule.timing.weekday,
            day_of_month: schedule.timing.day_of_month,
            hour: schedule.timing.hour,
            minute: schedule.timing.minute,
            period_preset: schedule.period_preset,
            recipients: entry.recipients.into_iter().map(Into::into).collect(),
            is_active: schedule.is_active,
            next_run_at: timestamp(schedule.next_run_at),
            last_run_at: schedule.last_run_at.map(timestamp),
            created_at: timestamp(schedule.created_at),
            updated_at: timestamp(schedule.updated_at),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct ReportDeliveryResponse {
    pub(crate) id: String,
    pub(crate) schedule_id: String,
    pub(crate) user_id: String,
    pub(crate) recipient: Option<UserReferenceResponse>,
    #[schema(example = "2026-11-01T11:00:00Z")]
    pub(crate) scheduled_for: String,
    pub(crate) status: DeliveryStatusSchema,
    pub(crate) row_count: Option<u32>,
    pub(crate) failure_reason: Option<String>,
    pub(crate) finished_at: Option<String>,
}

impl From<DeliveryEntry> for ReportDeliveryResponse {
    fn from(entry: DeliveryEntry) -> Self {
        let delivery = entry.delivery;
        Self {
            id: delivery.id.to_string(),
            schedule_id: delivery.schedule_id.to_string(),
            user_id: delivery.user_id.into_inner(),
            recipient: entry.recipient.map(Into::into),
            scheduled_for: timestamp(delivery.scheduled_for),
            status: delivery.status.into(),
            row_count: delivery.row_count,
            failure_reason: delivery.failure_reason,
            finished_at: delivery.finished_at.map(timestamp),
        }
    }
}
