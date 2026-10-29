use async_trait::async_trait;
use serde_json::{Map, Value, json};

use crate::domain::custom::{ReportEntry, RunOverrides, authorize_edit};
use crate::http::custom_dto::ReportResponse;
use crate::service::reports::run_report;
use aphel_kernel::Ctx;
use aphel_kernel::api::dto::aggregates::AggregateResultResponse;
use aphel_kernel::application::dashboard::{
    DashboardPorts, SettingsReader, WidgetContext, WidgetProvider, WidgetSize,
};
use aphel_kernel::application::shared::is_access_denied;
use aphel_kernel::domain::access::{AccessScope, Permission};
use aphel_kernel::domain::custom_objects::is_custom_record_type;
use aphel_kernel::domain::dashboard::ResolvedPeriod;
use aphel_kernel::errors::{AppError, CodedKind, FieldViolation};
use aphel_kernel::state::runtime_state;

pub(crate) const SAVED_REPORT_WIDGET: &str = "saved_report";
pub(crate) const REPORT_ID_SETTING: &str = "report_id";
pub(crate) const USE_DASHBOARD_PERIOD_SETTING: &str = "use_dashboard_period";

pub(crate) struct SavedReportWidget;

#[async_trait]
impl<P> WidgetProvider<P> for SavedReportWidget
where
    P: DashboardPorts,
{
    fn key(&self) -> &str {
        SAVED_REPORT_WIDGET
    }

    fn title(&self) -> &str {
        "Relatório salvo"
    }

    fn category(&self) -> &str {
        "reports"
    }

    fn required_permissions(&self) -> Vec<Permission> {
        Vec::new()
    }

    fn is_available(&self, scope: &AccessScope) -> bool {
        let permissions = scope.permissions();
        if permissions.grants_everything() {
            return true;
        }
        let record_types = aphel_kernel::modules::app().record_types();
        permissions.grants().any(|grant| {
            grant
                .permission
                .as_str()
                .strip_suffix(".read")
                .is_some_and(is_custom_record_type)
                || record_types
                    .definitions()
                    .any(|definition| definition.read_permission == grant.permission)
        })
    }

    fn default_size(&self) -> WidgetSize {
        WidgetSize::Large
    }

    fn default_settings(&self) -> Value {
        json!({ REPORT_ID_SETTING: null, USE_DASHBOARD_PERIOD_SETTING: true })
    }

    fn normalize_settings(&self, settings: &Value) -> Result<Value, Vec<FieldViolation>> {
        let mut reader = SettingsReader::new(settings)?;
        let report_id = reader.uuid(REPORT_ID_SETTING);
        let use_period = reader.boolean(USE_DASHBOARD_PERIOD_SETTING, true);
        let mut normalized = Map::new();
        normalized.insert(
            REPORT_ID_SETTING.to_owned(),
            report_id.map_or(Value::Null, |id| Value::String(id.to_string())),
        );
        normalized.insert(
            USE_DASHBOARD_PERIOD_SETTING.to_owned(),
            Value::Bool(use_period),
        );
        reader.finish(normalized)
    }

    async fn data(
        &self,
        context: &WidgetContext<'_, P>,
        settings: &Value,
    ) -> Result<Value, AppError> {
        let normalized = <Self as WidgetProvider<P>>::normalize_settings(self, settings).map_err(
            |violations| AppError::invalid_fields("invalid widget settings", violations),
        )?;
        let report_id = normalized
            .get(REPORT_ID_SETTING)
            .and_then(Value::as_str)
            .map(str::to_owned);
        let use_period = normalized
            .get(USE_DASHBOARD_PERIOD_SETTING)
            .and_then(Value::as_bool)
            .unwrap_or(true);
        let mut data = json!({
            "report_id": report_id,
            "use_dashboard_period": use_period,
            "period": period_json(&context.period),
            "status": "no_report",
            "report": Value::Null,
            "result": Value::Null,
        });
        let Some(report_id) = report_id else {
            return Ok(data);
        };
        let state = runtime_state()?;
        let ctx = Ctx::from_state(&state, context.scope).await?;
        let overrides = if use_period {
            dashboard_overrides(&context.period)
        } else {
            RunOverrides::default()
        };
        let outcome = match run_report(&ctx, &report_id, overrides).await {
            Err(error) if is_period_error(&error) => {
                run_report(&ctx, &report_id, RunOverrides::default()).await
            }
            outcome => outcome,
        };
        match outcome {
            Ok((report, result)) => {
                let can_edit = authorize_edit(context.scope, &report).is_ok();
                data["status"] = json!("ok");
                data["report"] =
                    serde_json::to_value(ReportResponse::from(ReportEntry { report, can_edit }))
                        .map_err(|error| AppError::Internal(error.to_string()))?;
                data["result"] = serde_json::to_value(AggregateResultResponse::from(result))
                    .map_err(|error| AppError::Internal(error.to_string()))?;
            }
            Err(error) if is_unavailable(&error) => {
                data["status"] = json!("unavailable");
            }
            Err(error) => return Err(error),
        }
        Ok(data)
    }
}

fn period_json(period: &ResolvedPeriod) -> Value {
    json!({
        "preset": period.preset.as_str(),
        "from": period.from.format("%Y-%m-%d").to_string(),
        "to": period.to.format("%Y-%m-%d").to_string(),
    })
}

fn dashboard_overrides(period: &ResolvedPeriod) -> RunOverrides {
    let preset = period.preset.as_str();
    if preset == "custom" {
        RunOverrides {
            preset: Some(preset.to_owned()),
            from: Some(period.from.format("%Y-%m-%d").to_string()),
            to: Some(period.to.format("%Y-%m-%d").to_string()),
            compare_to_previous: None,
        }
    } else {
        RunOverrides {
            preset: Some(preset.to_owned()),
            ..RunOverrides::default()
        }
    }
}

fn is_period_error(error: &AppError) -> bool {
    matches!(error, AppError::CodedFields { code, .. } if *code == crate::domain::custom::REPORT_PERIOD_REQUIRED)
}

fn is_unavailable(error: &AppError) -> bool {
    is_access_denied(error)
        || matches!(error, AppError::NotFound(_))
        || matches!(
            error,
            AppError::Coded {
                kind: CodedKind::NotFound,
                ..
            }
        )
}
