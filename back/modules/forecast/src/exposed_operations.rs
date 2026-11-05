use aphel_kernel::modules::OperationSpec;
use aphel_sdk::{OperationKey, operation};

use serde::Deserialize;
use serde_json::{Value, json};

use crate::http::{
    ForecastDealResponse, ForecastDealsParams, ForecastReportParams, ForecastReportResponse,
};
use crate::service;
use aphel_kernel::Ctx;
use aphel_kernel::modules::{OperationFuture, OperationPage, operation_input, operation_output};

#[derive(Deserialize)]
struct DealsInput {
    #[serde(flatten)]
    filter: ForecastDealsParams,
    #[serde(flatten)]
    page: OperationPage,
}

fn summary<'a>(ctx: &'a Ctx<'_>, input: Value) -> OperationFuture<'a> {
    Box::pin(async move {
        let params: ForecastReportParams = operation_input(input)?;
        let report = service::report(ctx, params.into_query()).await?;
        operation_output(ForecastReportResponse::from(report))
    })
}

fn deals<'a>(ctx: &'a Ctx<'_>, input: Value) -> OperationFuture<'a> {
    Box::pin(async move {
        let input: DealsInput = operation_input(input)?;
        let page = service::deals(ctx, input.filter.into_query(input.page.window())).await?;
        let deals: Vec<ForecastDealResponse> =
            page.listing.items.into_iter().map(Into::into).collect();
        Ok(json!({ "data": operation_output(deals)?, "total": page.listing.total }))
    })
}

const OP_FORECAST_SUMMARY: OperationKey<Value, Value> = operation!("forecast.summary");
const OP_FORECAST_DEALS: OperationKey<Value, Value> = operation!("forecast.deals");

pub(crate) fn register(app: &mut aphel_kernel::modules::AppBuilder) {
    app.operation(OP_FORECAST_SUMMARY, OperationSpec::read("Revenue forecast per month from the current month, within the actor's deals.read scope: { owner_id?, team_id?, months? (at most 18) } -> { currency, timezone, from, to, probability_source, settings, months: [..], totals }"), summary);
    app.operation(OP_FORECAST_DEALS, OperationSpec::read("Open deals expected to close in a month, largest first: { month: YYYY-MM, category?: pipeline|best_case|commit, owner_id?, team_id?, page?, per_page? } -> { data, total }"), deals);
}
