use axum::Json;
use axum::extract::rejection::QueryRejection;
use axum::extract::{Query, State};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::domain::{
    DATE_FORMAT, ForecastBucket, ForecastCategory, ForecastDealView, ForecastDealsQuery,
    ForecastMethod, ForecastMonth, ForecastOwnerFilter, ForecastReport, ForecastReportQuery,
    ForecastSettings, ForecastSummary, MONTH_FORMAT, ProbabilitySource,
    UpdateForecastSettingsCommand,
};
use crate::service;
use aphel_kernel::Ctx;
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::api::dto::common::{Data, ErrorEnvelope, Page, timestamp};
use aphel_sdk::platform::api::dto::files::UserReferenceResponse;
use aphel_sdk::platform::api::error::ApiError;
use aphel_sdk::platform::api::extractors::{
    AuthContext, ClientInfo, IfMatch, JsonBody, Pagination, PaginationQuery,
};
use aphel_sdk::platform::api::routes::v1::members::window;
use aphel_sdk::platform::api::state::AppState;
use aphel_sdk::platform::application::organization_admin::ListWindow;

pub(crate) const TAG: &str = "forecast";

fn money(value: Decimal) -> String {
    format!("{:.2}", value.round_dp(2))
}

pub(crate) fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(get_forecast))
        .routes(routes!(list_forecast_deals))
        .routes(routes!(get_forecast_settings, update_forecast_settings))
}

#[utoipa::path(
    get,
    path = "/forecast",
    tag = TAG,
    operation_id = "forecast_get",
    security(("session_cookie" = [])),
    params(ForecastReportParams),
    responses(
        (status = 200, description = "Revenue forecast per month, from the current month (organization timezone) through `months` (default: the configured horizon, at most 18). Open deals are grouped by the month of `expected_close_date`; `forecast` follows the configured method: `stage_probability` = amount × stage probability, `deal_probability` = amount × deal probability (stage probability when empty), `commit_only` = amount of the commit category. Categories (pipeline, best case, commit) come from the probability ranges of the settings and are listed when enabled or with `commit_only`. `won` is the amount won in each month (`closed_at`), `null` when the user cannot read it. Everything respects the `deals.read` scope and field permissions of the caller; when `probability` is hidden the stage probability is used (`probability_source: stage`). Amounts ignore the currency of each deal", body = Data<ForecastReportResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `deals.read` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid `months`, `owner_id` or `team_id` (`validation_error`), or `amount`/`expected_close_date` hidden for the role (`field_not_accessible`)", body = ErrorEnvelope),
    )
)]
async fn get_forecast(
    State(state): State<AppState>,
    tenant: AuthContext,
    Query(params): Query<ForecastReportParams>,
) -> Result<Json<Data<ForecastReportResponse>>, ApiError> {
    let report = service::report(
        &Ctx::from_state(&state, &tenant.scope).await?,
        params.into_query(),
    )
    .await?;
    Ok(Json(Data::new(report.into())))
}

#[utoipa::path(
    get,
    path = "/forecast/deals",
    tag = TAG,
    operation_id = "forecast_deals",
    security(("session_cookie" = [])),
    params(ForecastDealsParams, PaginationQuery),
    responses(
        (status = 200, description = "Open deals expected to close in `month`, optionally of one forecast `category`, by amount (largest first), with the probability used and the weighted amount of the configured method. Same scope and field rules as the forecast", body = Page<ForecastDealResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `deals.read` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid month, category, owner or pagination (`validation_error`), or `amount`/`expected_close_date` hidden for the role (`field_not_accessible`)", body = ErrorEnvelope),
    )
)]
async fn list_forecast_deals(
    State(state): State<AppState>,
    tenant: AuthContext,
    params: Result<Query<ForecastDealsParams>, QueryRejection>,
    pagination: Pagination,
) -> Result<Json<Page<ForecastDealResponse>>, ApiError> {
    let Query(params) = params.map_err(|rejection| AppError::Validation(rejection.body_text()))?;
    let page = service::deals(
        &Ctx::from_state(&state, &tenant.scope).await?,
        params.into_query(window(pagination)),
    )
    .await?;
    Ok(Json(Page::new(
        page.listing.items.into_iter().map(Into::into).collect(),
        page.listing.total,
        pagination,
    )))
}

#[utoipa::path(
    get,
    path = "/forecast/settings",
    tag = TAG,
    operation_id = "forecast_settings_get",
    security(("session_cookie" = [])),
    responses(
        (status = 200, description = "Forecast settings of the organization (defaults while never saved: `deal_probability`, categories off, best case from 50%, commit from 90%, 6 months, `updated_at: null`)", body = Data<ForecastSettingsResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
    )
)]
async fn get_forecast_settings(
    State(state): State<AppState>,
    tenant: AuthContext,
) -> Result<Json<Data<ForecastSettingsResponse>>, ApiError> {
    let settings = service::settings(&Ctx::from_state(&state, &tenant.scope).await?).await?;
    Ok(Json(Data::new(settings.into())))
}

#[utoipa::path(
    patch,
    path = "/forecast/settings",
    tag = TAG,
    operation_id = "forecast_settings_update",
    security(("session_cookie" = [])),
    params(
        ("If-Match" = Option<String>, Header, description = "`updated_at` of the settings as read (ISO-8601, quotes optional); ignored while the settings were never saved"),
    ),
    request_body = UpdateForecastSettingsRequest,
    responses(
        (status = 200, description = "Settings updated (`forecast_settings.updated`); omitted properties stay unchanged. Probabilities are percentages from 1 to 100 and `commit_min_probability` must be greater than `best_case_min_probability`; deals below `best_case_min_probability` are pipeline. Needs `settings.manage`", body = Data<ForecastSettingsResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 409, description = "The settings changed since the `If-Match` version (`version_conflict`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid method, probability or horizon, or malformed `If-Match` (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn update_forecast_settings(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    IfMatch(expected): IfMatch,
    JsonBody(request): JsonBody<UpdateForecastSettingsRequest>,
) -> Result<Json<Data<ForecastSettingsResponse>>, ApiError> {
    let settings = service::update_settings(
        &Ctx::from_state(&state, &tenant.scope).await?,
        request.into_command(client.0.ip, expected),
    )
    .await?;
    Ok(Json(Data::new(settings.into())))
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[schema(as = ForecastMethod)]
pub(crate) enum ForecastMethodSchema {
    StageProbability,
    DealProbability,
    CommitOnly,
}

impl From<ForecastMethod> for ForecastMethodSchema {
    fn from(value: ForecastMethod) -> Self {
        match value {
            ForecastMethod::StageProbability => Self::StageProbability,
            ForecastMethod::DealProbability => Self::DealProbability,
            ForecastMethod::CommitOnly => Self::CommitOnly,
        }
    }
}

impl From<ForecastMethodSchema> for ForecastMethod {
    fn from(value: ForecastMethodSchema) -> Self {
        match value {
            ForecastMethodSchema::StageProbability => Self::StageProbability,
            ForecastMethodSchema::DealProbability => Self::DealProbability,
            ForecastMethodSchema::CommitOnly => Self::CommitOnly,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[schema(as = ForecastCategory)]
pub(crate) enum ForecastCategorySchema {
    Pipeline,
    BestCase,
    Commit,
}

impl From<ForecastCategory> for ForecastCategorySchema {
    fn from(value: ForecastCategory) -> Self {
        match value {
            ForecastCategory::Pipeline => Self::Pipeline,
            ForecastCategory::BestCase => Self::BestCase,
            ForecastCategory::Commit => Self::Commit,
        }
    }
}

impl From<ForecastCategorySchema> for ForecastCategory {
    fn from(value: ForecastCategorySchema) -> Self {
        match value {
            ForecastCategorySchema::Pipeline => Self::Pipeline,
            ForecastCategorySchema::BestCase => Self::BestCase,
            ForecastCategorySchema::Commit => Self::Commit,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[schema(as = ForecastProbabilitySource)]
pub(crate) enum ProbabilitySourceSchema {
    Stage,
    DealOrStage,
}

impl From<ProbabilitySource> for ProbabilitySourceSchema {
    fn from(value: ProbabilitySource) -> Self {
        match value {
            ProbabilitySource::Stage => Self::Stage,
            ProbabilitySource::DealOrStage => Self::DealOrStage,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct ForecastSettingsResponse {
    pub(crate) method: ForecastMethodSchema,
    pub(crate) categories_enabled: bool,
    #[schema(example = 50)]
    pub(crate) best_case_min_probability: u8,
    #[schema(example = 90)]
    pub(crate) commit_min_probability: u8,
    #[schema(example = 6)]
    pub(crate) horizon_months: u8,
    #[schema(example = "2026-10-20T13:45:00Z")]
    pub(crate) updated_at: Option<String>,
}

impl From<ForecastSettings> for ForecastSettingsResponse {
    fn from(settings: ForecastSettings) -> Self {
        Self {
            method: settings.method.into(),
            categories_enabled: settings.categories_enabled,
            best_case_min_probability: settings.best_case_min_probability,
            commit_min_probability: settings.commit_min_probability,
            horizon_months: settings.horizon_months,
            updated_at: settings.updated_at.map(timestamp),
        }
    }
}

#[derive(Debug, Default, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct UpdateForecastSettingsRequest {
    pub(crate) method: Option<ForecastMethodSchema>,
    pub(crate) categories_enabled: Option<bool>,
    #[schema(example = 50)]
    pub(crate) best_case_min_probability: Option<i64>,
    #[schema(example = 90)]
    pub(crate) commit_min_probability: Option<i64>,
    #[schema(example = 6)]
    pub(crate) horizon_months: Option<i64>,
}

impl UpdateForecastSettingsRequest {
    pub(crate) fn into_command(
        self,
        ip: Option<String>,
        expected_updated_at: Option<DateTime<Utc>>,
    ) -> UpdateForecastSettingsCommand {
        UpdateForecastSettingsCommand {
            method: self
                .method
                .map(|method| ForecastMethod::from(method).as_str().to_owned()),
            categories_enabled: self.categories_enabled,
            best_case_min_probability: self.best_case_min_probability,
            commit_min_probability: self.commit_min_probability,
            horizon_months: self.horizon_months,
            ip,
            expected_updated_at,
        }
    }
}

#[derive(Debug, Default, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct ForecastReportParams {
    pub(crate) owner_id: Option<String>,
    pub(crate) team_id: Option<String>,
    #[param(example = 6)]
    pub(crate) months: Option<i64>,
}

impl ForecastReportParams {
    pub(crate) fn into_query(self) -> ForecastReportQuery {
        ForecastReportQuery {
            owners: ForecastOwnerFilter {
                owner_id: self.owner_id,
                team_id: self.team_id,
            },
            months: self.months,
        }
    }
}

#[derive(Debug, Default, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct ForecastDealsParams {
    #[param(example = "2026-11")]
    pub(crate) month: String,
    pub(crate) category: Option<ForecastCategorySchema>,
    pub(crate) owner_id: Option<String>,
    pub(crate) team_id: Option<String>,
}

impl ForecastDealsParams {
    pub(crate) fn into_query(self, window: ListWindow) -> ForecastDealsQuery {
        ForecastDealsQuery {
            owners: ForecastOwnerFilter {
                owner_id: self.owner_id,
                team_id: self.team_id,
            },
            month: self.month,
            category: self.category.map(Into::into),
            window,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct ForecastBucketResponse {
    pub(crate) count: u64,
    #[schema(example = "120000.00")]
    pub(crate) amount: String,
}

impl From<ForecastBucket> for ForecastBucketResponse {
    fn from(bucket: ForecastBucket) -> Self {
        Self {
            count: bucket.count,
            amount: money(bucket.amount),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct ForecastCategoryResponse {
    pub(crate) category: ForecastCategorySchema,
    pub(crate) count: u64,
    #[schema(example = "45000.00")]
    pub(crate) amount: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct ForecastSummaryResponse {
    pub(crate) open: ForecastBucketResponse,
    #[schema(example = "61500.00")]
    pub(crate) forecast: String,
    pub(crate) categories: Option<Vec<ForecastCategoryResponse>>,
    pub(crate) won: Option<ForecastBucketResponse>,
}

impl From<ForecastSummary> for ForecastSummaryResponse {
    fn from(summary: ForecastSummary) -> Self {
        Self {
            open: summary.open.into(),
            forecast: money(summary.forecast),
            categories: summary.categories.map(|categories| {
                categories
                    .into_iter()
                    .map(|(category, bucket)| ForecastCategoryResponse {
                        category: category.into(),
                        count: bucket.count,
                        amount: money(bucket.amount),
                    })
                    .collect()
            }),
            won: summary.won.map(Into::into),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct ForecastMonthResponse {
    #[schema(example = "2026-11")]
    pub(crate) month: String,
    #[schema(example = "2026-11-01")]
    pub(crate) starts_on: String,
    #[serde(flatten)]
    pub(crate) summary: ForecastSummaryResponse,
}

impl From<ForecastMonth> for ForecastMonthResponse {
    fn from(month: ForecastMonth) -> Self {
        Self {
            month: month.month.format(MONTH_FORMAT).to_string(),
            starts_on: month.month.format(DATE_FORMAT).to_string(),
            summary: month.summary.into(),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct ForecastReportResponse {
    #[schema(example = "BRL")]
    pub(crate) currency: String,
    #[schema(example = "America/Sao_Paulo")]
    pub(crate) timezone: String,
    #[schema(example = "2026-10-01")]
    pub(crate) from: String,
    #[schema(example = "2027-03-31")]
    pub(crate) to: String,
    pub(crate) probability_source: ProbabilitySourceSchema,
    pub(crate) settings: ForecastSettingsResponse,
    pub(crate) months: Vec<ForecastMonthResponse>,
    pub(crate) totals: ForecastSummaryResponse,
}

impl From<ForecastReport> for ForecastReportResponse {
    fn from(report: ForecastReport) -> Self {
        Self {
            currency: report.currency,
            timezone: report.timezone,
            from: report.from.format(DATE_FORMAT).to_string(),
            to: report.to.format(DATE_FORMAT).to_string(),
            probability_source: report.probability_source.into(),
            settings: report.settings.into(),
            months: report.months.into_iter().map(Into::into).collect(),
            totals: report.totals.into(),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct ForecastDealResponse {
    pub(crate) id: String,
    pub(crate) title: String,
    #[schema(example = "15000.00")]
    pub(crate) amount: Option<String>,
    #[schema(example = "BRL")]
    pub(crate) currency: String,
    #[schema(example = 60)]
    pub(crate) probability: u8,
    pub(crate) stage_name: String,
    pub(crate) owner: Option<UserReferenceResponse>,
    #[schema(example = "2026-11-15")]
    pub(crate) expected_close_date: Option<String>,
    pub(crate) category: Option<ForecastCategorySchema>,
    #[schema(example = "9000.00")]
    pub(crate) weighted_amount: Option<String>,
}

impl From<ForecastDealView> for ForecastDealResponse {
    fn from(view: ForecastDealView) -> Self {
        let deal = view.deal;
        Self {
            id: deal.id.to_string(),
            title: deal.title,
            amount: deal.amount.map(money),
            currency: deal.currency,
            probability: deal.probability,
            stage_name: deal.stage_name,
            owner: deal.owner.map(Into::into),
            expected_close_date: deal
                .expected_close_date
                .map(|day| day.format(DATE_FORMAT).to_string()),
            category: view.category.map(Into::into),
            weighted_amount: view.weighted_amount.map(|amount: Decimal| money(amount)),
        }
    }
}
