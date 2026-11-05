use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::domain::{
    CreateExchangeRateCommand, CurrencySettings, CurrencyView, ExchangeRate, ExchangeRateFilter,
    RateSource, RatesProviderSettings, SetCurrenciesCommand, SyncOutcome,
    UpdateExchangeRateCommand, UpdateRatesProviderCommand,
};
use crate::service;
use aphel_kernel::Ctx;
use aphel_sdk::platform::api::dto::common::{Data, ErrorEnvelope, Page, timestamp as instant};
use aphel_sdk::platform::api::error::ApiError;
use aphel_sdk::platform::api::extractors::{
    AuthContext, ClientInfo, IfMatch, JsonBody, Pagination, PaginationQuery,
};
use aphel_sdk::platform::api::routes::v1::members::window;
use aphel_sdk::platform::api::state::AppState;

pub(crate) const TAG: &str = "currencies";

const UNAUTHORIZED: &str = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)";
const FORBIDDEN_MANAGE: &str =
    "Missing `settings.manage` (`permission_denied`, `insufficient_scope`)";
const FORBIDDEN_READ: &str =
    "Missing `money.read` or `settings.manage` (`permission_denied`, `insufficient_scope`)";

pub(crate) fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(get_currencies, set_currencies))
        .routes(routes!(list_rates, create_rate))
        .routes(routes!(get_rate, update_rate, delete_rate))
        .routes(routes!(update_provider))
        .routes(routes!(sync_rates))
}

#[utoipa::path(
    put,
    path = "/money/currencies/provider",
    tag = TAG,
    operation_id = "currencies_provider_update",
    security(("session_cookie" = [])),
    request_body = UpdateRatesProviderRequest,
    responses(
        (status = 200, description = "Exchange rate provider configured (`currency.provider_updated`). `manual` turns the automatic fetching off; `frankfurter` fetches the daily rates of the European Central Bank from the public Frankfurter API (no key; `url` points to a self-hosted instance, empty uses https://api.frankfurter.app). The `money.sync_rates` job fetches every day at 06:15 UTC", body = Data<CurrencySettingsResponse>),
        (status = 401, description = UNAUTHORIZED, body = ErrorEnvelope),
        (status = 403, description = FORBIDDEN_MANAGE, body = ErrorEnvelope),
        (status = 422, description = "Unknown provider or invalid address (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn update_provider(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    JsonBody(request): JsonBody<UpdateRatesProviderRequest>,
) -> Result<Json<Data<CurrencySettingsResponse>>, ApiError> {
    let settings = service::update_provider(
        &Ctx::from_state(&state, &tenant.scope).await?,
        UpdateRatesProviderCommand {
            provider: request.provider,
            url: request.url,
            ip: client.0.ip,
        },
    )
    .await?;
    Ok(Json(Data::new(settings.into())))
}

#[utoipa::path(
    post,
    path = "/money/currencies/rates/sync",
    tag = TAG,
    operation_id = "currencies_rates_sync",
    security(("session_cookie" = [])),
    responses(
        (status = 200, description = "Today's rates fetched now from the configured provider for every enabled currency. A new rate publishes `exchange_rate.created` and a changed one `exchange_rate.updated` (source `integration`); a manual rate of the same date is kept. Currencies the provider does not know are listed in `failed`. Active monetary records are converted again in the background", body = Data<ExchangeRatesSyncResponse>),
        (status = 401, description = UNAUTHORIZED, body = ErrorEnvelope),
        (status = 403, description = FORBIDDEN_MANAGE, body = ErrorEnvelope),
        (status = 422, description = "No provider configured (`exchange_rate_provider_not_configured`)", body = ErrorEnvelope),
    )
)]
async fn sync_rates(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
) -> Result<Json<Data<ExchangeRatesSyncResponse>>, ApiError> {
    let outcome =
        service::sync_now(&Ctx::from_state(&state, &tenant.scope).await?, client.0.ip).await?;
    Ok(Json(Data::new(outcome.into())))
}

#[utoipa::path(
    get,
    path = "/money/currencies/rates/{rate_id}",
    tag = TAG,
    operation_id = "currencies_rates_get",
    security(("session_cookie" = [])),
    params(("rate_id" = String, Path, description = "Exchange rate id")),
    responses(
        (status = 200, description = "Exchange rate (the `data` of the `exchange_rate.*` events)", body = Data<ExchangeRateResponse>),
        (status = 401, description = UNAUTHORIZED, body = ErrorEnvelope),
        (status = 403, description = FORBIDDEN_READ, body = ErrorEnvelope),
        (status = 404, description = "Rate not found (`exchange_rate_not_found`)", body = ErrorEnvelope),
    )
)]
async fn get_rate(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(rate_id): Path<String>,
) -> Result<Json<Data<ExchangeRateResponse>>, ApiError> {
    let rate = service::get_rate(&Ctx::from_state(&state, &tenant.scope).await?, &rate_id).await?;
    Ok(Json(Data::new(rate.into())))
}

#[utoipa::path(
    get,
    path = "/money/currencies",
    tag = TAG,
    operation_id = "currencies_get",
    security(("session_cookie" = [])),
    responses(
        (status = 200, description = "Currencies of the organization: the base currency (the organization currency, always first and enabled, rate 1) and the other enabled ones with the rate in effect today (`today`, in the organization time zone): the latest rate dated on or before today. Monetary records can only use enabled currencies; their `amount_base` is `amount × rate`, rounded to cents", body = Data<CurrencySettingsResponse>),
        (status = 401, description = UNAUTHORIZED, body = ErrorEnvelope),
        (status = 403, description = FORBIDDEN_READ, body = ErrorEnvelope),
    )
)]
async fn get_currencies(
    State(state): State<AppState>,
    tenant: AuthContext,
) -> Result<Json<Data<CurrencySettingsResponse>>, ApiError> {
    let settings = service::settings(&Ctx::from_state(&state, &tenant.scope).await?).await?;
    Ok(Json(Data::new(settings.into())))
}

#[utoipa::path(
    put,
    path = "/money/currencies",
    tag = TAG,
    operation_id = "currencies_set",
    security(("session_cookie" = [])),
    request_body = SetCurrenciesRequest,
    responses(
        (status = 200, description = "Enabled currencies replaced by the list (ISO 4217 codes, at most 30; the base currency is always enabled and ignored in the list). Publishes `currency.enabled` or `currency.disabled` per change. Disabling keeps the monetary records already in the currency, but new monetary records and currency changes can no longer use it", body = Data<CurrencySettingsResponse>),
        (status = 401, description = UNAUTHORIZED, body = ErrorEnvelope),
        (status = 403, description = FORBIDDEN_MANAGE, body = ErrorEnvelope),
        (status = 422, description = "Invalid currency code or too many currencies (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn set_currencies(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    JsonBody(request): JsonBody<SetCurrenciesRequest>,
) -> Result<Json<Data<CurrencySettingsResponse>>, ApiError> {
    let settings = service::set_currencies(
        &Ctx::from_state(&state, &tenant.scope).await?,
        request.into_command(client.0.ip),
    )
    .await?;
    Ok(Json(Data::new(settings.into())))
}

#[utoipa::path(
    get,
    path = "/money/currencies/rates",
    tag = TAG,
    operation_id = "currencies_rates_list",
    security(("session_cookie" = [])),
    params(ExchangeRateListParams, PaginationQuery),
    responses(
        (status = 200, description = "Exchange rates of the organization, newest effective date first (`currency` keeps one currency). A rate is the value of one unit of the currency in the base currency", body = Page<ExchangeRateResponse>),
        (status = 401, description = UNAUTHORIZED, body = ErrorEnvelope),
        (status = 403, description = FORBIDDEN_READ, body = ErrorEnvelope),
        (status = 422, description = "Invalid currency filter (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn list_rates(
    State(state): State<AppState>,
    tenant: AuthContext,
    pagination: Pagination,
    Query(params): Query<ExchangeRateListParams>,
) -> Result<Json<Page<ExchangeRateResponse>>, ApiError> {
    let listing = service::list_rates(
        &Ctx::from_state(&state, &tenant.scope).await?,
        params.into_filter(),
        window(pagination),
    )
    .await?;
    Ok(Json(Page::new(
        listing.items.into_iter().map(Into::into).collect(),
        listing.total,
        pagination,
    )))
}

#[utoipa::path(
    post,
    path = "/money/currencies/rates",
    tag = TAG,
    operation_id = "currencies_rates_create",
    security(("session_cookie" = [])),
    request_body = CreateExchangeRateRequest,
    responses(
        (status = 201, description = "Manual exchange rate created (`exchange_rate.created`); `effective_date` defaults to today in the organization time zone and `rate` is a positive number with up to 10 decimal places. Active monetary records in the currency and their items are converted again in the background (`money.recalculate`; completed records retain their closing conversion). A future date is applied on that day", body = Data<ExchangeRateResponse>),
        (status = 401, description = UNAUTHORIZED, body = ErrorEnvelope),
        (status = 403, description = FORBIDDEN_MANAGE, body = ErrorEnvelope),
        (status = 409, description = "The currency already has a rate on this date (`exchange_rate_exists`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid fields or the base currency (`validation_error`), or the currency is not enabled (`currency_not_enabled`)", body = ErrorEnvelope),
    )
)]
async fn create_rate(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    JsonBody(request): JsonBody<CreateExchangeRateRequest>,
) -> Result<(StatusCode, Json<Data<ExchangeRateResponse>>), ApiError> {
    let rate = service::create_rate(
        &Ctx::from_state(&state, &tenant.scope).await?,
        request.into_command(client.0.ip),
    )
    .await?;
    Ok((StatusCode::CREATED, Json(Data::new(rate.into()))))
}

#[utoipa::path(
    patch,
    path = "/money/currencies/rates/{rate_id}",
    tag = TAG,
    operation_id = "currencies_rates_update",
    security(("session_cookie" = [])),
    params(
        ("rate_id" = String, Path, description = "Exchange rate id"),
        ("If-Match" = Option<String>, Header, description = "`updated_at` of the rate as last read (ISO-8601); when it changed since, the update is refused with 409 `version_conflict`"),
    ),
    request_body = UpdateExchangeRateRequest,
    responses(
        (status = 200, description = "Exchange rate updated (`exchange_rate.updated` with the changes); an edited rate becomes `manual`. Active monetary records in the currency are converted again in the background", body = Data<ExchangeRateResponse>),
        (status = 401, description = UNAUTHORIZED, body = ErrorEnvelope),
        (status = 403, description = FORBIDDEN_MANAGE, body = ErrorEnvelope),
        (status = 404, description = "Rate not found (`exchange_rate_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "Another rate of the currency has the date (`exchange_rate_exists`) or the rate changed after the `If-Match` version (`version_conflict`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid fields (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn update_rate(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    IfMatch(expected): IfMatch,
    Path(rate_id): Path<String>,
    JsonBody(request): JsonBody<UpdateExchangeRateRequest>,
) -> Result<Json<Data<ExchangeRateResponse>>, ApiError> {
    let rate = service::update_rate(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &rate_id,
        request.into_command(client.0.ip, expected),
    )
    .await?;
    Ok(Json(Data::new(rate.into())))
}

#[utoipa::path(
    delete,
    path = "/money/currencies/rates/{rate_id}",
    tag = TAG,
    operation_id = "currencies_rates_delete",
    security(("session_cookie" = [])),
    params(("rate_id" = String, Path, description = "Exchange rate id")),
    responses(
        (status = 204, description = "Exchange rate removed (`exchange_rate.deleted`); active monetary records in the currency fall back to the previous rate, or to no conversion (`amount_base` empty) when none is left"),
        (status = 401, description = UNAUTHORIZED, body = ErrorEnvelope),
        (status = 403, description = FORBIDDEN_MANAGE, body = ErrorEnvelope),
        (status = 404, description = "Rate not found (`exchange_rate_not_found`)", body = ErrorEnvelope),
    )
)]
async fn delete_rate(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(rate_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    service::delete_rate(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &rate_id,
        client.0.ip,
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[schema(as = ExchangeRateSource)]
pub(crate) enum ExchangeRateSourceSchema {
    Manual,
    Integration,
}

impl From<RateSource> for ExchangeRateSourceSchema {
    fn from(value: RateSource) -> Self {
        match value {
            RateSource::Manual => Self::Manual,
            RateSource::Integration => Self::Integration,
        }
    }
}

fn rate_text(rate: rust_decimal::Decimal) -> String {
    rate.normalize().to_string()
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct ExchangeRateResponse {
    pub(crate) id: String,
    #[schema(example = "USD")]
    pub(crate) currency: String,
    #[schema(example = "2026-10-22")]
    pub(crate) effective_date: String,
    #[schema(example = "5.4321")]
    pub(crate) rate: String,
    pub(crate) source: ExchangeRateSourceSchema,
    pub(crate) provider: Option<String>,
    pub(crate) created_by: Option<String>,
    #[schema(example = "2026-10-22T13:45:00Z")]
    pub(crate) created_at: String,
    #[schema(example = "2026-10-22T13:45:00Z")]
    pub(crate) updated_at: String,
}

impl From<ExchangeRate> for ExchangeRateResponse {
    fn from(rate: ExchangeRate) -> Self {
        Self {
            id: rate.id.to_string(),
            currency: rate.currency,
            effective_date: rate.effective_date.format("%Y-%m-%d").to_string(),
            rate: rate_text(rate.rate),
            source: rate.source.into(),
            provider: rate.provider,
            created_by: rate.created_by.map(|id| id.into_inner()),
            created_at: instant(rate.created_at),
            updated_at: instant(rate.updated_at),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct CurrencyResponse {
    #[schema(example = "USD")]
    pub(crate) code: String,
    pub(crate) is_base: bool,
    pub(crate) enabled_at: Option<String>,
    pub(crate) current_rate: Option<ExchangeRateResponse>,
}

impl From<CurrencyView> for CurrencyResponse {
    fn from(currency: CurrencyView) -> Self {
        Self {
            code: currency.code,
            is_base: currency.is_base,
            enabled_at: currency.enabled_at.map(instant),
            current_rate: currency.current_rate.map(Into::into),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct CurrencySettingsResponse {
    #[schema(example = "BRL")]
    pub(crate) base: String,
    #[schema(example = "2026-10-22")]
    pub(crate) today: String,
    pub(crate) currencies: Vec<CurrencyResponse>,
    pub(crate) provider: RatesProviderResponse,
}

impl From<CurrencySettings> for CurrencySettingsResponse {
    fn from(settings: CurrencySettings) -> Self {
        Self {
            base: settings.base,
            today: settings.today.format("%Y-%m-%d").to_string(),
            currencies: settings.currencies.into_iter().map(Into::into).collect(),
            provider: settings.provider.into(),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct RatesProviderResponse {
    #[schema(example = "frankfurter")]
    pub(crate) provider: String,
    pub(crate) url: Option<String>,
    pub(crate) last_synced_at: Option<String>,
}

impl From<RatesProviderSettings> for RatesProviderResponse {
    fn from(settings: RatesProviderSettings) -> Self {
        Self {
            provider: settings.provider,
            url: settings.url,
            last_synced_at: settings.last_synced_at.map(instant),
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct UpdateRatesProviderRequest {
    #[schema(example = "frankfurter")]
    pub(crate) provider: String,
    pub(crate) url: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct ExchangeRatesSyncResponse {
    pub(crate) provider: String,
    #[schema(example = "2026-10-22")]
    pub(crate) effective_date: String,
    pub(crate) created: Vec<String>,
    pub(crate) updated: Vec<String>,
    pub(crate) unchanged: Vec<String>,
    pub(crate) kept_manual: Vec<String>,
    pub(crate) failed: Vec<String>,
}

impl From<SyncOutcome> for ExchangeRatesSyncResponse {
    fn from(outcome: SyncOutcome) -> Self {
        Self {
            provider: outcome.provider,
            effective_date: outcome.effective_date.format("%Y-%m-%d").to_string(),
            created: outcome.created,
            updated: outcome.updated,
            unchanged: outcome.unchanged,
            kept_manual: outcome.kept_manual,
            failed: outcome.failed,
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct SetCurrenciesRequest {
    #[schema(example = json!(["USD", "EUR"]))]
    pub(crate) currencies: Vec<String>,
}

impl SetCurrenciesRequest {
    pub(crate) fn into_command(self, ip: Option<String>) -> SetCurrenciesCommand {
        SetCurrenciesCommand {
            currencies: self.currencies,
            ip,
        }
    }
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct ExchangeRateListParams {
    pub(crate) currency: Option<String>,
}

impl ExchangeRateListParams {
    pub(crate) fn into_filter(self) -> ExchangeRateFilter {
        ExchangeRateFilter {
            currency: self.currency,
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct CreateExchangeRateRequest {
    #[schema(example = "USD")]
    pub(crate) currency: String,
    #[schema(example = "2026-10-22")]
    pub(crate) effective_date: Option<String>,
    #[schema(example = "5.4321")]
    pub(crate) rate: String,
}

impl CreateExchangeRateRequest {
    pub(crate) fn into_command(self, ip: Option<String>) -> CreateExchangeRateCommand {
        CreateExchangeRateCommand {
            currency: self.currency,
            effective_date: self.effective_date,
            rate: self.rate,
            ip,
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct UpdateExchangeRateRequest {
    #[schema(example = "2026-10-22")]
    pub(crate) effective_date: Option<String>,
    #[schema(example = "5.4321")]
    pub(crate) rate: Option<String>,
}

impl UpdateExchangeRateRequest {
    pub(crate) fn into_command(
        self,
        ip: Option<String>,
        expected_updated_at: Option<DateTime<Utc>>,
    ) -> UpdateExchangeRateCommand {
        UpdateExchangeRateCommand {
            effective_date: self.effective_date,
            rate: self.rate,
            expected_updated_at,
            ip,
        }
    }
}
