use chrono::{DateTime, SecondsFormat, Utc};
use chrono_tz::Tz;
use serde_json::{Map, Value, json};
use uuid::Uuid;

use aphel_kernel::application::access::{Authorizer, PERMISSION_DENIED};
use aphel_kernel::application::files::UserReference;
use aphel_kernel::application::query::{AggregatePeriod, AggregateQuery};
use aphel_kernel::application::shared::RecordType;
use aphel_kernel::domain::access::{AccessScope, Permission};
use aphel_kernel::domain::{TeamId, UserId};
use aphel_kernel::errors::{AppError, FieldViolation};
use aphel_sdk::views::ViewVisibility;

pub(crate) const REPORT_NOT_FOUND: &str = "report_not_found";
pub(crate) const REPORT_PERIOD_REQUIRED: &str = "report_period_required";
pub(crate) const REPORT_CREATED: &str = "report.created";
pub(crate) const REPORT_UPDATED: &str = "report.updated";
pub(crate) const REPORT_DELETED: &str = "report.deleted";
pub(crate) const REPORT_RECORD_TYPE: RecordType = RecordType::from_static("reports.report");

pub(crate) const MAX_REPORT_NAME_LENGTH: usize = 140;
pub(crate) const MAX_REPORT_DESCRIPTION_LENGTH: usize = 1000;
pub(crate) const MAX_REPORT_SETTINGS_BYTES: usize = 16 * 1024;
pub(crate) const CSV_DELIMITER: u8 = b';';

pub(crate) const REPORT_SCHEDULE_NOT_FOUND: &str = "report_schedule_not_found";
pub(crate) const REPORT_SCHEDULE_CREATED: &str = "report_schedule.created";
pub(crate) const REPORT_SCHEDULE_UPDATED: &str = "report_schedule.updated";
pub(crate) const REPORT_SCHEDULE_DELETED: &str = "report_schedule.deleted";
pub(crate) const REPORT_SCHEDULE_RECORD_TYPE: RecordType =
    RecordType::from_static("reports.report_schedule");
pub(crate) const MAX_SCHEDULE_RECIPIENTS: usize = 50;
pub(crate) const MAX_SCHEDULES_PER_REPORT: usize = 20;

pub(crate) const REPORT_SCHEDULES_DISPATCH_JOB: &str = "report_schedules.dispatch";
pub(crate) const REPORT_SCHEDULES_DISPATCH_CRON: &str = "0 */5 * * * *";
pub(crate) const REPORT_DELIVERY_JOB: &str = "report_schedule.deliver";
pub(crate) const REPORT_SCHEDULE_DELIVERED: &str = "report_schedule.delivered";
pub(crate) const REPORT_SCHEDULE_DELIVERY_FAILED: &str = "report_schedule.delivery_failed";
pub(crate) const MAX_DUE_SCHEDULES_PER_RUN: u32 = 200;
pub(crate) const MAX_FAILURE_REASON_LENGTH: usize = 500;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ReportVisualization {
    Table,
    Bar,
    Line,
    Pie,
    Number,
}

impl ReportVisualization {
    pub(crate) const ALL: [Self; 5] = [Self::Table, Self::Bar, Self::Line, Self::Pie, Self::Number];

    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Table => "table",
            Self::Bar => "bar",
            Self::Line => "line",
            Self::Pie => "pie",
            Self::Number => "number",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|visualization| visualization.as_str() == value)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SavedReport {
    pub(crate) id: Uuid,
    pub(crate) record_type: RecordType,
    pub(crate) name: String,
    pub(crate) description: Option<String>,
    pub(crate) owner_id: Option<UserId>,
    pub(crate) visibility: ViewVisibility,
    pub(crate) team_id: Option<TeamId>,
    pub(crate) visualization: ReportVisualization,
    pub(crate) definition: AggregateQuery,
    pub(crate) settings: Map<String, Value>,
    pub(crate) created_at: DateTime<Utc>,
    pub(crate) updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ReportEntry {
    pub(crate) report: SavedReport,
    pub(crate) can_edit: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ScheduleFrequency {
    Daily,
    Weekly,
    Monthly,
}

impl ScheduleFrequency {
    pub(crate) const ALL: [Self; 3] = [Self::Daily, Self::Weekly, Self::Monthly];

    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Daily => "daily",
            Self::Weekly => "weekly",
            Self::Monthly => "monthly",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|frequency| frequency.as_str() == value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ScheduleTiming {
    pub(crate) frequency: ScheduleFrequency,
    pub(crate) weekday: Option<u8>,
    pub(crate) day_of_month: Option<u8>,
    pub(crate) hour: u8,
    pub(crate) minute: u8,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ReportSchedule {
    pub(crate) id: Uuid,
    pub(crate) report_id: Uuid,
    pub(crate) created_by: Option<UserId>,
    pub(crate) timing: ScheduleTiming,
    pub(crate) period_preset: Option<String>,
    pub(crate) recipients: Vec<UserId>,
    pub(crate) is_active: bool,
    pub(crate) next_run_at: DateTime<Utc>,
    pub(crate) last_run_at: Option<DateTime<Utc>>,
    pub(crate) created_at: DateTime<Utc>,
    pub(crate) updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ScheduleEntry {
    pub(crate) schedule: ReportSchedule,
    pub(crate) recipients: Vec<UserReference>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DeliveryStatus {
    Sending,
    Sent,
    Failed,
}

impl DeliveryStatus {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Sending => "sending",
            Self::Sent => "sent",
            Self::Failed => "failed",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "sending" => Some(Self::Sending),
            "sent" => Some(Self::Sent),
            "failed" => Some(Self::Failed),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ReportDelivery {
    pub(crate) id: Uuid,
    pub(crate) schedule_id: Uuid,
    pub(crate) report_id: Uuid,
    pub(crate) user_id: UserId,
    pub(crate) scheduled_for: DateTime<Utc>,
    pub(crate) status: DeliveryStatus,
    pub(crate) row_count: Option<u32>,
    pub(crate) failure_reason: Option<String>,
    pub(crate) created_at: DateTime<Utc>,
    pub(crate) finished_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DeliveryEntry {
    pub(crate) delivery: ReportDelivery,
    pub(crate) recipient: Option<UserReference>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ReportRecipient {
    pub(crate) user_id: UserId,
    pub(crate) display_name: String,
    pub(crate) email: String,
    pub(crate) locale: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DueSchedule {
    pub(crate) schedule_id: Uuid,
    pub(crate) timezone: Tz,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CreateReportCommand {
    pub(crate) name: String,
    pub(crate) description: Option<String>,
    pub(crate) visibility: String,
    pub(crate) team_id: Option<String>,
    pub(crate) visualization: ReportVisualization,
    pub(crate) definition: AggregateQuery,
    pub(crate) settings: Value,
    pub(crate) ip: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct UpdateReportCommand {
    pub(crate) name: Option<String>,
    pub(crate) description: Option<Option<String>>,
    pub(crate) visibility: Option<String>,
    pub(crate) team_id: Option<Option<String>>,
    pub(crate) visualization: Option<ReportVisualization>,
    pub(crate) definition: Option<AggregateQuery>,
    pub(crate) settings: Option<Value>,
    pub(crate) expected_updated_at: Option<DateTime<Utc>>,
    pub(crate) ip: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct RunOverrides {
    pub(crate) preset: Option<String>,
    pub(crate) from: Option<String>,
    pub(crate) to: Option<String>,
    pub(crate) compare_to_previous: Option<bool>,
}

impl RunOverrides {}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CreateScheduleCommand {
    pub(crate) frequency: ScheduleFrequency,
    pub(crate) weekday: Option<u8>,
    pub(crate) day_of_month: Option<u8>,
    pub(crate) hour: u8,
    pub(crate) minute: u8,
    pub(crate) period_preset: Option<String>,
    pub(crate) recipients: Vec<String>,
    pub(crate) is_active: bool,
    pub(crate) ip: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct UpdateScheduleCommand {
    pub(crate) frequency: Option<ScheduleFrequency>,
    pub(crate) weekday: Option<Option<u8>>,
    pub(crate) day_of_month: Option<Option<u8>>,
    pub(crate) hour: Option<u8>,
    pub(crate) minute: Option<u8>,
    pub(crate) period_preset: Option<Option<String>>,
    pub(crate) recipients: Option<Vec<String>>,
    pub(crate) is_active: Option<bool>,
    pub(crate) expected_updated_at: Option<DateTime<Utc>>,
    pub(crate) ip: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ReportExport {
    pub(crate) file_name: String,
    pub(crate) mime_type: &'static str,
    pub(crate) content: bytes::Bytes,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DeliveryRequest {
    pub(crate) schedule_id: Uuid,
    pub(crate) report_id: Uuid,
    pub(crate) user_id: UserId,
    pub(crate) scheduled_for: DateTime<Utc>,
}

impl DeliveryRequest {
    pub(crate) fn to_payload(&self) -> Value {
        json!({
            "schedule_id": self.schedule_id.to_string(),
            "report_id": self.report_id.to_string(),
            "user_id": self.user_id.as_str(),
            "scheduled_for": self.scheduled_for.to_rfc3339_opts(SecondsFormat::Micros, true),
        })
    }

    pub(crate) fn from_payload(payload: &Value) -> Result<Self, AppError> {
        let text = |key: &str| payload.get(key).and_then(Value::as_str);
        let invalid = || AppError::Internal(format!("invalid {REPORT_DELIVERY_JOB} payload"));
        Ok(Self {
            schedule_id: text("schedule_id")
                .and_then(|id| Uuid::parse_str(id).ok())
                .ok_or_else(invalid)?,
            report_id: text("report_id")
                .and_then(|id| Uuid::parse_str(id).ok())
                .ok_or_else(invalid)?,
            user_id: text("user_id")
                .and_then(|id| UserId::new(id).ok())
                .ok_or_else(invalid)?,
            scheduled_for: text("scheduled_for")
                .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
                .map(|value| value.with_timezone(&Utc))
                .ok_or_else(invalid)?,
        })
    }

    pub(crate) fn unique_key(&self) -> String {
        format!(
            "report_delivery:{}:{}:{}",
            self.schedule_id,
            self.user_id.as_str(),
            self.scheduled_for.timestamp()
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DeliveryOutcome {
    Sent,
    AlreadySent,
    Failed,
    Skipped,
}

pub(crate) fn is_report_visible(scope: &AccessScope, report: &SavedReport) -> bool {
    match report.visibility {
        ViewVisibility::Organization => true,
        ViewVisibility::Private => report.owner_id.as_ref() == Some(scope.user_id()),
        ViewVisibility::Team => {
            report.owner_id.as_ref() == Some(scope.user_id())
                || report
                    .team_id
                    .as_ref()
                    .is_some_and(|team| scope.team_ids().contains(team))
        }
    }
}

pub(crate) fn authorize_edit(scope: &AccessScope, report: &SavedReport) -> Result<(), AppError> {
    let is_owner = report.owner_id.as_ref() == Some(scope.user_id());
    let manages = scope.require(&Permission::SETTINGS_MANAGE).is_ok();
    if manages || (is_owner && report.visibility != ViewVisibility::Organization) {
        return Ok(());
    }
    Err(AppError::forbidden_code(
        PERMISSION_DENIED,
        "only the owner or someone with settings.manage can change this report",
    ))
}

pub(crate) fn with_overrides(
    definition: &AggregateQuery,
    overrides: &RunOverrides,
) -> Result<AggregateQuery, AppError> {
    let mut query = definition.clone();
    if let Some(compare) = overrides.compare_to_previous {
        query.compare_to_previous = compare;
    }
    let preset = overrides
        .preset
        .as_deref()
        .map(str::trim)
        .filter(|preset| !preset.is_empty());
    if preset.is_none() && overrides.from.is_none() && overrides.to.is_none() {
        return Ok(query);
    }
    let Some(field) = definition
        .period
        .as_ref()
        .map(|period| period.field.clone())
    else {
        return Err(AppError::invalid_fields_code(
            REPORT_PERIOD_REQUIRED,
            "the report has no period field to override".to_owned(),
            vec![FieldViolation::new(
                "period",
                "the report has no period field",
            )],
        ));
    };
    let preset = preset
        .map(str::to_owned)
        .unwrap_or_else(|| "custom".to_owned());
    query.period = Some(AggregatePeriod {
        field,
        from: if preset == "custom" {
            overrides.from.clone()
        } else {
            None
        },
        to: if preset == "custom" {
            overrides.to.clone()
        } else {
            None
        },
        preset: Some(preset),
    });
    Ok(query)
}

pub(crate) fn prefixed(error: AppError, prefix: &str) -> AppError {
    let rename = |fields: Vec<FieldViolation>| {
        fields
            .into_iter()
            .map(|violation| {
                FieldViolation::new(format!("{prefix}.{}", violation.field), violation.message)
            })
            .collect()
    };
    match error {
        AppError::InvalidFields { message, fields } => AppError::InvalidFields {
            message,
            fields: rename(fields),
        },
        AppError::CodedFields {
            code,
            message,
            fields,
        } => AppError::CodedFields {
            code,
            message,
            fields: rename(fields),
        },
        other => other,
    }
}

pub(crate) fn report_not_found() -> AppError {
    AppError::not_found_code(REPORT_NOT_FOUND, "report was not found")
}

pub(crate) fn schedule_not_found() -> AppError {
    AppError::not_found_code(REPORT_SCHEDULE_NOT_FOUND, "report schedule was not found")
}
