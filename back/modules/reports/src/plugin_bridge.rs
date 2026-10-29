use std::sync::Arc;

use serde::Deserialize;
use serde_json::{Value, json};

use crate::http::custom_dto::{ListReportsParams, ReportResponse, RunReportRequest};
use crate::service::reports;
use aphel_kernel::Ctx;
use aphel_kernel::api::dto::aggregates::AggregateResultResponse;
use aphel_kernel::errors::AppError;
use aphel_kernel::modules::{
    OperationFuture, OperationPage, PluginOperation, operation_input, operation_output,
};

#[derive(Deserialize)]
struct ListInput {
    #[serde(flatten)]
    filter: ListReportsParams,
    #[serde(flatten)]
    page: OperationPage,
}

fn list<'a>(ctx: &'a Ctx<'_>, input: Value) -> OperationFuture<'a> {
    Box::pin(async move {
        let input: ListInput = operation_input(input)?;
        let entries = reports::list_reports(ctx, input.filter.record_type.as_deref()).await?;
        let total = entries.len() as u64;
        let window = input.page.window();
        let entries: Vec<ReportResponse> = entries
            .into_iter()
            .skip(usize::try_from(window.offset).unwrap_or(usize::MAX))
            .take(usize::try_from(window.limit).unwrap_or(usize::MAX))
            .map(Into::into)
            .collect();
        Ok(json!({ "data": operation_output(entries)?, "total": total }))
    })
}

fn run<'a>(ctx: &'a Ctx<'_>, input: Value) -> OperationFuture<'a> {
    Box::pin(async move {
        let Value::Object(mut fields) = input else {
            return Err(AppError::Validation(
                "invalid operation input: expected an object".to_owned(),
            ));
        };
        let report_id = match fields.remove("id") {
            Some(Value::String(id)) => id,
            _ => {
                return Err(AppError::Validation(
                    "invalid operation input: id is required".to_owned(),
                ));
            }
        };
        let request: RunReportRequest = operation_input(Value::Object(fields))?;
        let (_, result) = reports::run_report(ctx, &report_id, request.into()).await?;
        operation_output(AggregateResultResponse::from(result))
    })
}

pub(crate) fn operations() -> Vec<Arc<PluginOperation>> {
    vec![
        Arc::new(PluginOperation::read(
            "reports.list",
            "Saved reports visible to the actor, ordered by name: { record_type?, page?, per_page? } -> { data, total }",
            list,
        )),
        Arc::new(PluginOperation::read(
            "reports.run",
            "Run a saved report with the actor's read scope and field permissions: { id, preset?, from? (YYYY-MM-DD), to?, compare_to_previous? } -> { record_type, timezone, groups, metrics, rows, totals, period, previous_period }",
            run,
        )),
    ]
}
