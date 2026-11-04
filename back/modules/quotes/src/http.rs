use std::collections::BTreeMap;
use std::sync::LazyLock;
use std::time::Duration;

use async_trait::async_trait;
use axum::Json;
use axum::body::Body;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::Response;
use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use utoipa::{IntoParams, ToSchema};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;
use uuid::Uuid;

use crate::QUOTE;
use crate::domain::{
    AcceptQuoteCommand, CreateQuoteCommand, ManualAcceptQuoteCommand, PublicQuote,
    PublicQuoteState, Quote, QuoteDetails, QuoteInput, QuoteItem, QuoteItemInput, QuoteSettings,
    QuoteStatus, QuoteSummary, QuoteVersionSummary, RejectQuoteCommand, SendQuoteCommand,
    SubmitQuoteCommand, UpdateQuoteCommand, UpdateQuoteSettingsCommand,
};
use crate::{document, public, service, workflow};
use aphel_kernel::Ctx;
use aphel_kernel::api::dto::common::{
    Data, ErrorEnvelope, Page, QuickSearchParams, decimal_text, double_option, timestamp,
};
use aphel_kernel::api::dto::query::{RecordListQuery, RecordQueryParams};
use aphel_kernel::api::error::ApiError;
use aphel_kernel::api::extractors::{
    AuthContext, ClientInfo, IfMatch, JsonBody, Pagination, PaginationQuery,
};
use aphel_kernel::api::rate_limit::SharedRateLimiter;
use aphel_kernel::api::routes::v1::field_permissions::{redacted, redacted_items};
use aphel_kernel::api::routes::v1::files::content_disposition;
use aphel_kernel::api::state::AppState;
use aphel_kernel::domain::UserId;
use aphel_kernel::domain::access::AccessScope;
use aphel_kernel::errors::AppError;
use aphel_kernel::records::{RecordCtx, RecordTypeHandler, optional_record, values_request};
use aphel_sdk::email::EmailAddressDto;
use aphel_sdk::line_items::{DiscountKind, money};
use aphel_sdk::money::DecimalInputSchema;

pub(crate) const TAG: &str = "quotes";

pub(crate) fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list_quotes, create_quote))
        .routes(routes!(get_quote, update_quote, delete_quote))
        .routes(routes!(list_quote_versions, create_quote_version))
        .routes(routes!(submit_quote))
        .routes(routes!(send_quote))
        .routes(routes!(accept_quote))
        .routes(routes!(quote_pdf))
        .routes(routes!(quote_link))
        .routes(routes!(deal_quotes))
        .routes(routes!(get_quote_settings, update_quote_settings))
}

pub async fn quote_detail(
    state: &AppState,
    scope: &AccessScope,
    details: QuoteDetails,
) -> Result<Value, AppError> {
    redacted(state, scope, &QUOTE, QuoteDetailResponse::from(details)).await
}

#[utoipa::path(
    get,
    path = "/quotes",
    tag = TAG,
    operation_id = "quotes_list",
    security(("session_cookie" = [])),
    description = "Lists quotes (permission `quotes.read`, scoped by owner). Only the current version of each quote is listed unless `include_versions=true`. Filters accept the JSON AST in `filter` or `filter[field][operator]=value` (`filter[status][eq]=sent`, `filter[deal_id][eq]=<id>`, `filter[total][gte]=1000`, `filter[exceeds_discount_limit][eq]=true`). `sort=field,-other`; `search` (or `q`) matches the title.",
    params(RecordQueryParams, QuickSearchParams, QuoteListParams, PaginationQuery),
    responses(
        (status = 200, description = "Page of quotes", body = Page<QuoteResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `quotes.read` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid filter, sort, search or pagination (`validation_error`, `field_not_accessible`)", body = ErrorEnvelope),
    )
)]
async fn list_quotes(
    State(state): State<AppState>,
    tenant: AuthContext,
    RecordListQuery(mut query): RecordListQuery,
    Query(quick): Query<QuickSearchParams>,
    Query(params): Query<QuoteListParams>,
    pagination: Pagination,
) -> Result<Json<Page<Value>>, ApiError> {
    quick.merge_into(&mut query.search);
    let listing = service::list(
        &Ctx::from_state(&state, &tenant.scope).await?,
        query,
        params.include_versions.unwrap_or(false),
    )
    .await?;
    let items: Vec<QuoteResponse> = listing.items.into_iter().map(Into::into).collect();
    Ok(Json(Page::new(
        redacted_items(&state, &tenant.scope, &QUOTE, items).await?,
        listing.total,
        pagination,
    )))
}

#[utoipa::path(
    post,
    path = "/quotes",
    tag = TAG,
    operation_id = "quotes_create",
    security(("session_cookie" = [])),
    description = "Creates a quote for a deal (permissions `quotes.create` and read access to the deal) with the next number of the organization, version 1 and status `draft`, and publishes `quote.created`. The currency, contact and company come from the deal; the owner defaults to the caller; `valid_until` and `terms` default to the quote settings; the price list defaults to the best active list for the currency and company. `from_line_items: true` copies the deal items; otherwise send `items`. Items with `product_id` and no `unit_price` take the price of the chosen price list (or the product price); bundles enter as one line with their components. Totals are computed on the server: line = quantity × unit price − line discount; the total discount applies to the sum of the lines after their discounts and is prorated among them before the tax; tax = line tax % over the prorated line.",
    request_body = QuoteRequest,
    responses(
        (status = 201, description = "Quote created", body = Data<QuoteDetailResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `quotes.create` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Deal not found or not visible (`deal_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "A pending approval locks the write (`approval_pending`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid fields or items (`validation_error` with `details.fields` such as `items.0.unit_price`), unknown price list (`quote_price_list_not_found`), too many items (`quote_items_limit_reached`), fields the role cannot write (`field_not_accessible`, `field_read_only`)", body = ErrorEnvelope),
    )
)]
async fn create_quote(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    JsonBody(request): JsonBody<QuoteRequest>,
) -> Result<(StatusCode, Json<Data<Value>>), ApiError> {
    let details = service::create(
        &Ctx::from_state(&state, &tenant.scope).await?,
        request.into_create(client.0.ip),
    )
    .await?;
    Ok((
        StatusCode::CREATED,
        Json(Data::new(
            quote_detail(&state, &tenant.scope, details).await?,
        )),
    ))
}

#[utoipa::path(
    get,
    path = "/quotes/{quote_id}",
    tag = TAG,
    operation_id = "quotes_get",
    security(("session_cookie" = [])),
    description = "Returns a quote version with its items, totals, discount limit of the owner, versions, public link (after sending) and custom fields (permission `quotes.read`).",
    params(("quote_id" = String, Path, description = "Quote id (one version)")),
    responses(
        (status = 200, description = "The quote", body = Data<QuoteDetailResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `quotes.read` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Quote not found (`quote_not_found`)", body = ErrorEnvelope),
    )
)]
async fn get_quote(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(quote_id): Path<String>,
) -> Result<Json<Data<Value>>, ApiError> {
    let details = service::get(&Ctx::from_state(&state, &tenant.scope).await?, &quote_id).await?;
    Ok(Json(Data::new(
        quote_detail(&state, &tenant.scope, details).await?,
    )))
}

#[utoipa::path(
    patch,
    path = "/quotes/{quote_id}",
    tag = TAG,
    operation_id = "quotes_update",
    security(("session_cookie" = [])),
    description = "Partially updates the current version of a quote in `draft` or `approved` (permission `quotes.update`) and publishes `quote.updated` with the changes. `items`, when sent, replaces every line. `reprice: true` (or changing `price_list_id`) prices the product lines again. Changing an `approved` quote takes it back to `draft` and cancels its open approval requests. Send `If-Match` with the `updated_at` you read.",
    params(
        ("quote_id" = String, Path, description = "Quote id"),
        ("If-Match" = Option<String>, Header, description = "`updated_at` of the quote as read; 409 when it changed since"),
    ),
    request_body = QuoteRequest,
    responses(
        (status = 200, description = "Quote updated", body = Data<QuoteDetailResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `quotes.update` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Quote not found (`quote_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "Old version (`quote_not_current`), sent or closed (`quote_not_editable`), waiting for approval (`approval_pending`) or changed since `If-Match` (`version_conflict`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid fields or items (`validation_error`, `quote_price_list_not_found`, `quote_items_limit_reached`, `field_not_accessible`, `field_read_only`)", body = ErrorEnvelope),
    )
)]
async fn update_quote(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    IfMatch(expected): IfMatch,
    Path(quote_id): Path<String>,
    JsonBody(request): JsonBody<QuoteRequest>,
) -> Result<Json<Data<Value>>, ApiError> {
    let details = service::update(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &quote_id,
        request.into_update(client.0.ip, expected),
    )
    .await?;
    Ok(Json(Data::new(
        quote_detail(&state, &tenant.scope, details).await?,
    )))
}

#[utoipa::path(
    delete,
    path = "/quotes/{quote_id}",
    tag = TAG,
    operation_id = "quotes_delete",
    security(("session_cookie" = [])),
    params(("quote_id" = String, Path, description = "Quote id (any version)")),
    responses(
        (status = 204, description = "Quote deleted with all its versions (soft delete); publishes `quote.deleted` for each version and invalidates the public link"),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `quotes.delete` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Quote not found (`quote_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "A version was accepted (`quote_already_accepted`) or a link blocks the deletion (`record_in_use`)", body = ErrorEnvelope),
    )
)]
async fn delete_quote(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(quote_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    service::delete(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &quote_id,
        client.0.ip,
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    get,
    path = "/quotes/{quote_id}/versions",
    tag = TAG,
    operation_id = "quotes_versions",
    security(("session_cookie" = [])),
    description = "Every version of the quote number, newest first (permission `quotes.read`).",
    params(("quote_id" = String, Path, description = "Quote id (any version)")),
    responses(
        (status = 200, description = "Versions", body = Data<Vec<QuoteVersionResponse>>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `quotes.read` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Quote not found (`quote_not_found`)", body = ErrorEnvelope),
    )
)]
async fn list_quote_versions(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(quote_id): Path<String>,
) -> Result<Json<Data<Vec<QuoteVersionResponse>>>, ApiError> {
    let versions =
        service::versions(&Ctx::from_state(&state, &tenant.scope).await?, &quote_id).await?;
    Ok(Json(Data::new(
        versions.into_iter().map(Into::into).collect(),
    )))
}

#[utoipa::path(
    post,
    path = "/quotes/{quote_id}/versions",
    tag = TAG,
    operation_id = "quotes_create_version",
    security(("session_cookie" = [])),
    description = "Creates a new version from the current one (permissions `quotes.create` and `quotes.update`): same number, `version` + 1, status `draft`, copy of the header, items and custom fields. The previous version becomes read-only, its public link shows it was replaced and its open approval requests are cancelled. Publishes `quote.version_created`.",
    params(("quote_id" = String, Path, description = "Id of the current version")),
    responses(
        (status = 201, description = "New version", body = Data<QuoteDetailResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing permission (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Quote not found (`quote_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "Not the current version (`quote_not_current`) or already accepted (`quote_already_accepted`)", body = ErrorEnvelope),
    )
)]
async fn create_quote_version(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(quote_id): Path<String>,
) -> Result<(StatusCode, Json<Data<Value>>), ApiError> {
    let details = service::create_version(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &quote_id,
        client.0.ip,
    )
    .await?;
    Ok((
        StatusCode::CREATED,
        Json(Data::new(
            quote_detail(&state, &tenant.scope, details).await?,
        )),
    ))
}

#[utoipa::path(
    post,
    path = "/quotes/{quote_id}/submit",
    tag = TAG,
    operation_id = "quotes_submit",
    security(("session_cookie" = [])),
    description = "Sends a `draft` quote for approval (permission `quotes.update`) through the approval policies of the moment `quote_send` (such as the default discount limit policy, based on `exceeds_discount_limit`). When no policy applies, or an approval is already granted, the quote becomes `approved`; otherwise `pending_approval` until the approvers decide (`approval.approved` → `approved`, `approval.rejected`/`approval.cancelled` → `draft`). Publishes `quote.submitted` (and `quote.approved` when approved at once).",
    params(("quote_id" = String, Path, description = "Quote id")),
    request_body = SubmitQuoteRequest,
    responses(
        (status = 200, description = "Quote after the submission", body = Data<QuoteDetailResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `quotes.update` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Quote not found (`quote_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "Not a current draft (`quote_invalid_transition`, `quote_not_current`)", body = ErrorEnvelope),
        (status = 422, description = "No items (`quote_has_no_items`)", body = ErrorEnvelope),
    )
)]
async fn submit_quote(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(quote_id): Path<String>,
    JsonBody(request): JsonBody<SubmitQuoteRequest>,
) -> Result<Json<Data<Value>>, ApiError> {
    let details = workflow::submit(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &quote_id,
        request.into_command(client.0.ip),
    )
    .await?;
    Ok(Json(Data::new(
        quote_detail(&state, &tenant.scope, details).await?,
    )))
}

#[utoipa::path(
    post,
    path = "/quotes/{quote_id}/send",
    tag = TAG,
    operation_id = "quotes_send",
    security(("session_cookie" = [])),
    description = "Queues the quote e-mail from the caller's mailbox (permissions `quotes.send` and `emails.send`), linked to the deal, with the signed public link in the body (`{{quote_link}}` marks where; otherwise it is appended). Without `to`, the deal contact receives it. A `draft` or `approved` quote passes the approval checkpoint of `quote_send` first: a quote that needs approval cannot be sent before it (`approval_required`), and the granted approval is consumed. A `sent` quote can be sent again. Queue acceptance changes the status to `sent` and publishes `quote.sent` (also `deal.quote_sent` on the deal timeline); the delivery job sends the e-mail afterwards.",
    params(("quote_id" = String, Path, description = "Quote id")),
    request_body = SendQuoteRequest,
    responses(
        (status = 200, description = "Quote e-mail queued", body = Data<QuoteDetailResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `quotes.send` or `emails.send` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Quote not found (`quote_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "Needs approval (`approval_required`), waiting for approval (`approval_pending`), expired (`quote_expired`), old version (`quote_not_current`) or closed (`quote_invalid_transition`)", body = ErrorEnvelope),
        (status = 422, description = "No recipient or invalid e-mail (`validation_error`), no items (`quote_has_no_items`) or no mailbox to send from", body = ErrorEnvelope),
    )
)]
async fn send_quote(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(quote_id): Path<String>,
    JsonBody(request): JsonBody<SendQuoteRequest>,
) -> Result<Json<Data<Value>>, ApiError> {
    let details = workflow::send(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &quote_id,
        request.into_command(client.0.ip),
    )
    .await?;
    Ok(Json(Data::new(
        quote_detail(&state, &tenant.scope, details).await?,
    )))
}

#[utoipa::path(
    post,
    path = "/quotes/{quote_id}/accept",
    tag = TAG,
    operation_id = "quotes_accept",
    security(("session_cookie" = [])),
    description = "Records the acceptance of the current version of a quote by the seller on behalf of the customer (permission `quotes.update`), for an acceptance received outside the public link (signed paper, phone, e-mail). Works on `sent`, `approved` and `draft` quotes; a `draft` or `approved` quote passes the approval checkpoint of `quote_send` first (`approval_required`), like sending. `name` defaults to the quote contact and `email` to the contact e-mail. The status becomes `accepted`, the hook `QuoteAccepting` runs, `quote.accepted` is published with `manual: true` and `recorded_by` (also `deal.quote_accepted`) and the deal is synchronized exactly as in the public acceptance (job `quote.deal_sync`).",
    params(("quote_id" = String, Path, description = "Quote id")),
    request_body = AcceptQuoteManuallyRequest,
    responses(
        (status = 200, description = "Quote after the acceptance", body = Data<QuoteDetailResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `quotes.update` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Quote not found (`quote_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "Needs approval (`approval_required`), waiting for approval (`approval_pending`), expired (`quote_expired`), old version (`quote_not_current`) or already answered (`quote_invalid_transition`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid name, e-mail or note (`validation_error`) or no items (`quote_has_no_items`)", body = ErrorEnvelope),
    )
)]
async fn accept_quote(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(quote_id): Path<String>,
    JsonBody(request): JsonBody<AcceptQuoteManuallyRequest>,
) -> Result<Json<Data<Value>>, ApiError> {
    let details = workflow::accept(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &quote_id,
        request.into_command(client.0.ip),
    )
    .await?;
    Ok(Json(Data::new(
        quote_detail(&state, &tenant.scope, details).await?,
    )))
}

#[utoipa::path(
    get,
    path = "/quotes/{quote_id}/pdf",
    tag = TAG,
    operation_id = "quotes_pdf",
    security(("session_cookie" = [])),
    description = "PDF document of a quote version (permission `quotes.read`), generated on the fly: organization, number, version, status, contact, company, owner, validity, items, totals, terms and notes. Totals hidden from the caller by field permissions are left out.",
    params(("quote_id" = String, Path, description = "Quote id")),
    responses(
        (status = 200, description = "PDF served as an attachment", content_type = "application/pdf"),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `quotes.read` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Quote not found (`quote_not_found`)", body = ErrorEnvelope),
    )
)]
async fn quote_pdf(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(quote_id): Path<String>,
) -> Result<Response, ApiError> {
    let pdf = document::render(&Ctx::from_state(&state, &tenant.scope).await?, &quote_id).await?;
    let disposition = HeaderValue::from_str(&content_disposition(&pdf.file_name))
        .map_err(|_| AppError::Internal("invalid content disposition".to_owned()))?;
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/pdf")
        .header(header::CONTENT_LENGTH, pdf.content.len())
        .header(header::CONTENT_DISPOSITION, disposition)
        .header(header::X_CONTENT_TYPE_OPTIONS, "nosniff")
        .header(header::CACHE_CONTROL, "private, no-store")
        .body(Body::from(pdf.content))
        .map_err(|error| {
            AppError::Internal(format!("cannot build quote pdf response: {error}")).into()
        })
}

#[utoipa::path(
    get,
    path = "/quotes/{quote_id}/link",
    tag = TAG,
    operation_id = "quotes_link",
    security(("session_cookie" = [])),
    description = "Signed public link of a sent quote version (permission `quotes.read`). The link carries no internal id and only shows that version.",
    params(("quote_id" = String, Path, description = "Quote id")),
    responses(
        (status = 200, description = "Public link", body = Data<QuoteLinkResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `quotes.read` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Quote not found (`quote_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "The quote was not sent yet (`quote_not_sent`)", body = ErrorEnvelope),
    )
)]
async fn quote_link(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(quote_id): Path<String>,
) -> Result<Json<Data<QuoteLinkResponse>>, ApiError> {
    let url =
        workflow::public_link(&Ctx::from_state(&state, &tenant.scope).await?, &quote_id).await?;
    Ok(Json(Data::new(QuoteLinkResponse { url })))
}

#[utoipa::path(
    get,
    path = "/deals/{deal_id}/quotes",
    tag = TAG,
    operation_id = "quotes_for_deal",
    security(("session_cookie" = [])),
    description = "Current version of every quote of a deal, newest number first (permissions `quotes.read` and read access to the deal).",
    params(("deal_id" = String, Path, description = "Deal id")),
    responses(
        (status = 200, description = "Quotes of the deal", body = Data<Vec<QuoteResponse>>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `quotes.read` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Deal not found (`deal_not_found`)", body = ErrorEnvelope),
    )
)]
async fn deal_quotes(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(deal_id): Path<String>,
) -> Result<Json<Data<Vec<Value>>>, ApiError> {
    let quotes =
        service::for_deal(&Ctx::from_state(&state, &tenant.scope).await?, &deal_id).await?;
    let items: Vec<QuoteResponse> = quotes.into_iter().map(Into::into).collect();
    Ok(Json(Data::new(
        redacted_items(&state, &tenant.scope, &QUOTE, items).await?,
    )))
}

#[utoipa::path(
    get,
    path = "/quote-settings",
    tag = TAG,
    operation_id = "quote_settings_get",
    security(("session_cookie" = [])),
    description = "Quote settings of the organization: default validity in days, default terms and, per pipeline, the stage the deal moves to when a quote is accepted (`settings.manage` or `quotes.read`).",
    responses(
        (status = 200, description = "Settings", body = Data<QuoteSettingsResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing permission (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
    )
)]
async fn get_quote_settings(
    State(state): State<AppState>,
    tenant: AuthContext,
) -> Result<Json<Data<QuoteSettingsResponse>>, ApiError> {
    let settings = service::settings(&Ctx::from_state(&state, &tenant.scope).await?).await?;
    Ok(Json(Data::new(settings.into())))
}

#[utoipa::path(
    patch,
    path = "/quote-settings",
    tag = TAG,
    operation_id = "quote_settings_update",
    security(("session_cookie" = [])),
    description = "Updates the quote settings (`settings.manage`) and publishes `quote_settings.updated`. `accept_stages` replaces the whole map `{ \"<pipeline_id>\": \"<stage_id>\" }` (a `null` stage removes the pipeline).",
    params(("If-Match" = Option<String>, Header, description = "`updated_at` of the settings as read")),
    request_body = QuoteSettingsRequest,
    responses(
        (status = 200, description = "Settings updated", body = Data<QuoteSettingsResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 409, description = "Changed since `If-Match` (`version_conflict`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid settings (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn update_quote_settings(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    IfMatch(expected): IfMatch,
    JsonBody(request): JsonBody<QuoteSettingsRequest>,
) -> Result<Json<Data<QuoteSettingsResponse>>, ApiError> {
    let settings = service::update_settings(
        &Ctx::from_state(&state, &tenant.scope).await?,
        request.into_command(client.0.ip, expected),
    )
    .await?;
    Ok(Json(Data::new(settings.into())))
}

const MINUTE: Duration = Duration::from_secs(60);
const HOUR: Duration = Duration::from_secs(3600);

struct QuoteLimits {
    views: SharedRateLimiter,
    answers_per_address: SharedRateLimiter,
    answers_per_link: SharedRateLimiter,
}

static LIMITS: LazyLock<QuoteLimits> = LazyLock::new(|| QuoteLimits {
    views: SharedRateLimiter::new("quotes.public_view", 120, MINUTE),
    answers_per_address: SharedRateLimiter::new("quotes.public_answer_address", 20, HOUR),
    answers_per_link: SharedRateLimiter::new("quotes.public_answer_link", 10, HOUR),
});

pub(crate) fn public_router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(public_quote))
        .routes(routes!(public_accept))
        .routes(routes!(public_reject))
}

async fn limit(limiter: &SharedRateLimiter, key: &str) -> Result<(), AppError> {
    limiter
        .check(key, "too many attempts; wait a little and try again")
        .await
}

#[utoipa::path(
    get,
    path = "/public/quotes/{token}",
    tag = TAG,
    operation_id = "quotes_public_get",
    description = "Quote behind a signed public link (no session): organization, number, version, items, totals, validity, terms and notes. `state` is `open` (can be accepted or rejected), `accepted`, `rejected`, `expired` or `superseded` (a newer version replaced it). The first view of an open quote publishes `quote.viewed`.",
    params(("token" = String, Path, description = "Signed public token")),
    responses(
        (status = 200, description = "Quote", body = Data<PublicQuoteResponse>),
        (status = 404, description = "Invalid token (`quote_not_found`)", body = ErrorEnvelope),
        (status = 429, description = "Too many requests from this address (`rate_limited`)", body = ErrorEnvelope),
    )
)]
async fn public_quote(
    State(state): State<AppState>,
    client: ClientInfo,
    Path(token): Path<String>,
) -> Result<Json<Data<PublicQuoteResponse>>, ApiError> {
    limit(&LIMITS.views, client.ip_or_unknown()).await?;
    let quote = public::public_view(
        &Ctx::from_state(&state, &AccessScope::system()).await?,
        &token,
    )
    .await?;
    Ok(Json(Data::new(quote.into())))
}

#[utoipa::path(
    post,
    path = "/public/quotes/{token}/accept",
    tag = TAG,
    operation_id = "quotes_public_accept",
    description = "Accepts an open quote (no session, no `X-Requested-With` needed) with the name and e-mail of who accepts; the IP address, user agent and time are recorded. Publishes `quote.accepted` (and `deal.quote_accepted`) and queues the synchronization of the deal: its items are replaced by the quote lines and its amount becomes the quote total (or the sum of the items when the amount follows the items); when configured, the deal moves to the acceptance stage of its pipeline (a failure there is recorded and published as `quote.deal_sync_failed`, the acceptance is kept).",
    params(("token" = String, Path, description = "Signed public token")),
    request_body = PublicAcceptQuoteRequest,
    responses(
        (status = 200, description = "Accepted", body = Data<PublicQuoteResponse>),
        (status = 404, description = "Invalid token (`quote_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "Already answered (`quote_closed`), expired (`quote_expired`) or replaced by a newer version (`quote_superseded`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid name or e-mail (`validation_error`)", body = ErrorEnvelope),
        (status = 429, description = "Too many attempts (`rate_limited`)", body = ErrorEnvelope),
    )
)]
async fn public_accept(
    State(state): State<AppState>,
    client: ClientInfo,
    Path(token): Path<String>,
    JsonBody(request): JsonBody<PublicAcceptQuoteRequest>,
) -> Result<Json<Data<PublicQuoteResponse>>, ApiError> {
    limit(&LIMITS.answers_per_address, client.ip_or_unknown()).await?;
    limit(&LIMITS.answers_per_link, token.trim()).await?;
    let quote = public::public_accept(
        &Ctx::from_state(&state, &AccessScope::system()).await?,
        &token,
        request.into_command(client.0.ip.clone(), client.0.user_agent.clone()),
    )
    .await?;
    Ok(Json(Data::new(quote.into())))
}

#[utoipa::path(
    post,
    path = "/public/quotes/{token}/reject",
    tag = TAG,
    operation_id = "quotes_public_reject",
    description = "Rejects an open quote with an optional reason (no session). Publishes `quote.rejected` (and `deal.quote_rejected`).",
    params(("token" = String, Path, description = "Signed public token")),
    request_body = PublicRejectQuoteRequest,
    responses(
        (status = 200, description = "Rejected", body = Data<PublicQuoteResponse>),
        (status = 404, description = "Invalid token (`quote_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "Already answered (`quote_closed`), expired (`quote_expired`) or replaced (`quote_superseded`)", body = ErrorEnvelope),
        (status = 422, description = "Reason too long (`validation_error`)", body = ErrorEnvelope),
        (status = 429, description = "Too many attempts (`rate_limited`)", body = ErrorEnvelope),
    )
)]
async fn public_reject(
    State(state): State<AppState>,
    client: ClientInfo,
    Path(token): Path<String>,
    JsonBody(request): JsonBody<PublicRejectQuoteRequest>,
) -> Result<Json<Data<PublicQuoteResponse>>, ApiError> {
    limit(&LIMITS.answers_per_address, client.ip_or_unknown()).await?;
    limit(&LIMITS.answers_per_link, token.trim()).await?;
    let quote = public::public_reject(
        &Ctx::from_state(&state, &AccessScope::system()).await?,
        &token,
        request.into_command(client.0.ip.clone()),
    )
    .await?;
    Ok(Json(Data::new(quote.into())))
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[schema(as = QuoteStatus)]
pub(crate) enum QuoteStatusSchema {
    Draft,
    PendingApproval,
    Approved,
    Sent,
    Accepted,
    Rejected,
    Expired,
}

impl From<QuoteStatus> for QuoteStatusSchema {
    fn from(value: QuoteStatus) -> Self {
        match value {
            QuoteStatus::Draft => Self::Draft,
            QuoteStatus::PendingApproval => Self::PendingApproval,
            QuoteStatus::Approved => Self::Approved,
            QuoteStatus::Sent => Self::Sent,
            QuoteStatus::Accepted => Self::Accepted,
            QuoteStatus::Rejected => Self::Rejected,
            QuoteStatus::Expired => Self::Expired,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[schema(as = QuoteDiscountType)]
pub(crate) enum QuoteDiscountTypeSchema {
    None,
    Percent,
    Amount,
}

impl From<DiscountKind> for QuoteDiscountTypeSchema {
    fn from(value: DiscountKind) -> Self {
        match value {
            DiscountKind::None => Self::None,
            DiscountKind::Percent => Self::Percent,
            DiscountKind::Amount => Self::Amount,
        }
    }
}

impl From<QuoteDiscountTypeSchema> for DiscountKind {
    fn from(value: QuoteDiscountTypeSchema) -> Self {
        match value {
            QuoteDiscountTypeSchema::None => Self::None,
            QuoteDiscountTypeSchema::Percent => Self::Percent,
            QuoteDiscountTypeSchema::Amount => Self::Amount,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[schema(as = PublicQuoteState)]
pub(crate) enum PublicQuoteStateSchema {
    Open,
    Accepted,
    Rejected,
    Expired,
    Superseded,
}

impl From<PublicQuoteState> for PublicQuoteStateSchema {
    fn from(value: PublicQuoteState) -> Self {
        match value {
            PublicQuoteState::Open => Self::Open,
            PublicQuoteState::Accepted => Self::Accepted,
            PublicQuoteState::Rejected => Self::Rejected,
            PublicQuoteState::Expired => Self::Expired,
            PublicQuoteState::Superseded => Self::Superseded,
        }
    }
}

fn discount_kind_text(kind: QuoteDiscountTypeSchema) -> String {
    DiscountKind::from(kind).as_str().to_owned()
}

fn date(value: Option<NaiveDate>) -> Option<String> {
    value.map(|date| date.format("%Y-%m-%d").to_string())
}

fn instant(value: Option<DateTime<Utc>>) -> Option<String> {
    value.map(timestamp)
}

fn percent(value: Decimal) -> String {
    value.normalize().to_string()
}

fn text(value: Option<Value>) -> Option<String> {
    value.and_then(decimal_text)
}

#[derive(Debug, Default, Deserialize, ToSchema)]
pub(crate) struct QuoteItemRequest {
    #[serde(default)]
    pub(crate) product_id: Option<String>,
    #[serde(default)]
    #[schema(example = "Onboarding")]
    pub(crate) name: Option<String>,
    #[serde(default)]
    pub(crate) description: Option<String>,
    #[serde(default)]
    pub(crate) unit: Option<String>,
    #[serde(default)]
    #[schema(value_type = Option<DecimalInputSchema>, example = "2")]
    pub(crate) quantity: Option<Value>,
    #[serde(default)]
    #[schema(value_type = Option<DecimalInputSchema>, example = "1500.00")]
    pub(crate) unit_price: Option<Value>,
    #[serde(default)]
    pub(crate) discount_type: Option<QuoteDiscountTypeSchema>,
    #[serde(default)]
    #[schema(value_type = Option<DecimalInputSchema>, example = "10")]
    pub(crate) discount_value: Option<Value>,
    #[serde(default)]
    #[schema(value_type = Option<DecimalInputSchema>, example = "5")]
    pub(crate) tax_percent: Option<Value>,
    #[serde(default)]
    pub(crate) recurrence: Option<String>,
    #[serde(default)]
    #[schema(value_type = Option<i64>, example = 12)]
    pub(crate) term_months: Option<Value>,
}

impl From<QuoteItemRequest> for QuoteItemInput {
    fn from(request: QuoteItemRequest) -> Self {
        Self {
            product_id: request.product_id,
            name: request.name,
            description: request.description,
            unit: request.unit,
            quantity: text(request.quantity),
            unit_price: text(request.unit_price),
            discount_type: request.discount_type.map(discount_kind_text),
            discount_value: text(request.discount_value),
            tax_percent: text(request.tax_percent),
            recurrence: request.recurrence,
            term_months: text(request.term_months),
        }
    }
}

#[derive(Debug, Default, Deserialize, ToSchema)]
pub(crate) struct QuoteRequest {
    #[serde(default)]
    pub(crate) deal_id: Option<String>,
    #[serde(default)]
    pub(crate) from_line_items: Option<bool>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>, example = "Onboarding proposal")]
    pub(crate) title: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>)]
    pub(crate) owner_id: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>)]
    pub(crate) price_list_id: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>, example = "2026-11-30")]
    pub(crate) valid_until: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>)]
    pub(crate) terms: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>)]
    pub(crate) notes: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<QuoteDiscountTypeSchema>)]
    pub(crate) discount_type: Option<Option<QuoteDiscountTypeSchema>>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<DecimalInputSchema>, example = "5")]
    pub(crate) discount_value: Option<Option<Value>>,
    #[serde(default)]
    pub(crate) items: Option<Vec<QuoteItemRequest>>,
    #[serde(default)]
    pub(crate) reprice: Option<bool>,
    #[serde(default)]
    #[schema(value_type = Option<Object>)]
    pub(crate) custom_fields: Option<Map<String, Value>>,
}

impl QuoteRequest {
    fn input(&mut self) -> QuoteInput {
        QuoteInput {
            title: self.title.take(),
            owner_id: self.owner_id.take(),
            price_list_id: self.price_list_id.take(),
            valid_until: self.valid_until.take(),
            terms: self.terms.take(),
            notes: self.notes.take(),
            discount_type: self
                .discount_type
                .take()
                .map(|value| value.map(discount_kind_text)),
            discount_value: self
                .discount_value
                .take()
                .map(|value| value.and_then(decimal_text)),
        }
    }

    fn items(&mut self) -> Option<Vec<QuoteItemInput>> {
        self.items
            .take()
            .map(|items| items.into_iter().map(Into::into).collect())
    }

    pub(crate) fn into_create(mut self, ip: Option<String>) -> CreateQuoteCommand {
        CreateQuoteCommand {
            deal_id: self.deal_id.take().unwrap_or_default(),
            from_line_items: self.from_line_items.unwrap_or(false),
            input: self.input(),
            items: self.items(),
            custom_fields: self.custom_fields.take().unwrap_or_default(),
            ip,
        }
    }

    pub(crate) fn into_update(
        mut self,
        ip: Option<String>,
        expected_updated_at: Option<DateTime<Utc>>,
    ) -> UpdateQuoteCommand {
        UpdateQuoteCommand {
            input: self.input(),
            items: self.items(),
            reprice: self.reprice.unwrap_or(false),
            custom_fields: self.custom_fields.take().unwrap_or_default(),
            ip,
            expected_updated_at,
        }
    }
}

#[derive(Debug, Default, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct QuoteListParams {
    pub(crate) include_versions: Option<bool>,
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct QuoteItemComponentResponse {
    pub(crate) product_id: Option<String>,
    pub(crate) name: String,
    pub(crate) code: Option<String>,
    #[schema(example = "1")]
    pub(crate) quantity: String,
    #[schema(example = "100.00")]
    pub(crate) unit_price: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct QuoteItemResponse {
    pub(crate) id: String,
    pub(crate) position: i32,
    pub(crate) product_id: Option<String>,
    pub(crate) name: String,
    pub(crate) description: Option<String>,
    pub(crate) unit: Option<String>,
    #[schema(example = "2")]
    pub(crate) quantity: String,
    #[schema(example = "1500.00")]
    pub(crate) unit_price: String,
    pub(crate) discount_type: QuoteDiscountTypeSchema,
    #[schema(example = "10.00")]
    pub(crate) discount_value: String,
    #[schema(example = "5")]
    pub(crate) tax_percent: String,
    pub(crate) recurrence: String,
    pub(crate) term_months: Option<u16>,
    pub(crate) components: Vec<QuoteItemComponentResponse>,
    #[schema(example = "3000.00")]
    pub(crate) gross: String,
    #[schema(example = "300.00")]
    pub(crate) discount_amount: String,
    #[schema(example = "0.00")]
    pub(crate) total_discount_share: String,
    #[schema(example = "135.00")]
    pub(crate) tax_amount: String,
    #[schema(example = "2835.00")]
    pub(crate) total: String,
}

impl From<QuoteItem> for QuoteItemResponse {
    fn from(item: QuoteItem) -> Self {
        let data = item.data;
        Self {
            id: item.id.to_string(),
            position: item.position,
            product_id: data.product_id.map(|id| id.to_string()),
            name: data.name,
            description: data.description,
            unit: data.unit,
            quantity: data.quantity.normalize().to_string(),
            unit_price: money(data.unit_price),
            discount_type: data.discount_kind.into(),
            discount_value: money(data.discount_value),
            tax_percent: percent(data.tax_percent),
            recurrence: data.recurrence.as_str().to_owned(),
            term_months: data.term_months,
            components: data
                .components
                .into_iter()
                .map(|component| QuoteItemComponentResponse {
                    product_id: component.product_id.map(|id| id.to_string()),
                    name: component.name,
                    code: component.code,
                    quantity: component.quantity.normalize().to_string(),
                    unit_price: money(component.unit_price),
                })
                .collect(),
            gross: money(item.amounts.gross),
            discount_amount: money(item.amounts.item_discount),
            total_discount_share: money(item.amounts.total_discount_share),
            tax_amount: money(item.amounts.tax),
            total: money(item.amounts.total),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct QuoteResponse {
    pub(crate) id: String,
    #[schema(example = 42)]
    pub(crate) number: u32,
    #[schema(example = 1)]
    pub(crate) version: u32,
    pub(crate) is_current: bool,
    pub(crate) title: String,
    pub(crate) status: QuoteStatusSchema,
    pub(crate) deal_id: String,
    pub(crate) owner_id: Option<String>,
    pub(crate) contact_id: Option<String>,
    pub(crate) company_id: Option<String>,
    #[schema(example = "BRL")]
    pub(crate) currency: String,
    pub(crate) price_list_id: Option<String>,
    #[schema(example = "2026-11-30")]
    pub(crate) valid_until: Option<String>,
    #[schema(example = "3000.00")]
    pub(crate) subtotal: String,
    #[schema(example = "300.00")]
    pub(crate) discount_total: String,
    #[schema(example = "135.00")]
    pub(crate) tax_total: String,
    #[schema(example = "2835.00")]
    pub(crate) total: String,
    pub(crate) exceeds_discount_limit: bool,
    pub(crate) sent_at: Option<String>,
    pub(crate) viewed_at: Option<String>,
    pub(crate) accepted_at: Option<String>,
    pub(crate) rejected_at: Option<String>,
    pub(crate) expired_at: Option<String>,
    pub(crate) created_by: Option<String>,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
    #[schema(value_type = Object)]
    pub(crate) custom_fields: Map<String, Value>,
}

impl QuoteResponse {
    fn build(quote: &Quote, custom_fields: Map<String, Value>) -> Self {
        Self {
            id: quote.id.to_string(),
            number: quote.number,
            version: quote.version,
            is_current: quote.is_current,
            title: quote.title.clone(),
            status: quote.status.into(),
            deal_id: quote.deal_id.to_string(),
            owner_id: quote.owner_id.as_ref().map(|id| id.as_str().to_owned()),
            contact_id: quote.contact_id.map(|id| id.to_string()),
            company_id: quote.company_id.map(|id| id.to_string()),
            currency: quote.currency.clone(),
            price_list_id: quote.price_list_id.map(|id| id.to_string()),
            valid_until: date(quote.valid_until),
            subtotal: money(quote.subtotal),
            discount_total: money(quote.discount_total),
            tax_total: money(quote.tax_total),
            total: money(quote.total),
            exceeds_discount_limit: quote.exceeds_discount_limit,
            sent_at: instant(quote.sent_at),
            viewed_at: instant(quote.viewed_at),
            accepted_at: instant(quote.accepted_at),
            rejected_at: instant(quote.rejected_at),
            expired_at: instant(quote.expired_at),
            created_by: quote.created_by.as_ref().map(|id| id.as_str().to_owned()),
            created_at: timestamp(quote.created_at),
            updated_at: timestamp(quote.updated_at),
            custom_fields,
        }
    }
}

impl From<QuoteSummary> for QuoteResponse {
    fn from(summary: QuoteSummary) -> Self {
        Self::build(&summary.quote, summary.custom_fields)
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct QuoteVersionResponse {
    pub(crate) id: String,
    pub(crate) version: u32,
    pub(crate) status: QuoteStatusSchema,
    pub(crate) total: String,
    pub(crate) is_current: bool,
    pub(crate) created_at: String,
}

impl From<QuoteVersionSummary> for QuoteVersionResponse {
    fn from(version: QuoteVersionSummary) -> Self {
        Self {
            id: version.id.to_string(),
            version: version.version,
            status: version.status.into(),
            total: money(version.total),
            is_current: version.is_current,
            created_at: timestamp(version.created_at),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct QuoteAcceptanceResponse {
    pub(crate) name: String,
    pub(crate) email: String,
    pub(crate) ip: Option<String>,
    pub(crate) user_agent: Option<String>,
    pub(crate) accepted_at: Option<String>,
    pub(crate) recorded_by: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct QuoteDetailResponse {
    pub(crate) id: String,
    pub(crate) number: u32,
    pub(crate) version: u32,
    pub(crate) is_current: bool,
    pub(crate) title: String,
    pub(crate) status: QuoteStatusSchema,
    pub(crate) deal_id: String,
    pub(crate) owner_id: Option<String>,
    pub(crate) contact_id: Option<String>,
    pub(crate) company_id: Option<String>,
    pub(crate) currency: String,
    pub(crate) price_list_id: Option<String>,
    pub(crate) valid_until: Option<String>,
    pub(crate) terms: Option<String>,
    pub(crate) notes: Option<String>,
    pub(crate) discount_type: QuoteDiscountTypeSchema,
    pub(crate) discount_value: String,
    pub(crate) items: Vec<QuoteItemResponse>,
    pub(crate) subtotal: String,
    pub(crate) items_discount: String,
    pub(crate) total_discount: String,
    pub(crate) discount_total: String,
    pub(crate) tax_total: String,
    pub(crate) total: String,
    #[schema(example = "12.5")]
    pub(crate) max_discount_percent: String,
    pub(crate) discount_limit: Option<String>,
    pub(crate) exceeds_discount_limit: bool,
    pub(crate) public_url: Option<String>,
    pub(crate) sent_at: Option<String>,
    pub(crate) sent_by: Option<String>,
    pub(crate) viewed_at: Option<String>,
    pub(crate) accepted_at: Option<String>,
    pub(crate) acceptance: Option<QuoteAcceptanceResponse>,
    pub(crate) rejected_at: Option<String>,
    pub(crate) rejection_reason: Option<String>,
    pub(crate) expired_at: Option<String>,
    pub(crate) deal_synced_at: Option<String>,
    pub(crate) deal_sync_error: Option<String>,
    pub(crate) versions: Vec<QuoteVersionResponse>,
    pub(crate) created_by: Option<String>,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
    #[schema(value_type = Object)]
    pub(crate) custom_fields: Map<String, Value>,
}

impl From<QuoteDetails> for QuoteDetailResponse {
    fn from(details: QuoteDetails) -> Self {
        let quote = details.quote;
        Self {
            id: quote.id.to_string(),
            number: quote.number,
            version: quote.version,
            is_current: quote.is_current,
            title: quote.title,
            status: quote.status.into(),
            deal_id: quote.deal_id.to_string(),
            owner_id: quote.owner_id.as_ref().map(|id| id.as_str().to_owned()),
            contact_id: quote.contact_id.map(|id| id.to_string()),
            company_id: quote.company_id.map(|id| id.to_string()),
            currency: quote.currency,
            price_list_id: quote.price_list_id.map(|id| id.to_string()),
            valid_until: date(quote.valid_until),
            terms: quote.terms,
            notes: quote.notes,
            discount_type: quote.discount.kind.into(),
            discount_value: money(quote.discount.value),
            items: details.items.into_iter().map(Into::into).collect(),
            subtotal: money(quote.subtotal),
            items_discount: money(details.items_discount),
            total_discount: money(details.total_discount),
            discount_total: money(quote.discount_total),
            tax_total: money(quote.tax_total),
            total: money(quote.total),
            max_discount_percent: percent(details.max_discount_percent),
            discount_limit: details.discount_limit.map(percent),
            exceeds_discount_limit: quote.exceeds_discount_limit,
            public_url: details.public_url,
            sent_at: instant(quote.sent_at),
            sent_by: quote.sent_by.map(UserId::into_inner),
            viewed_at: instant(quote.viewed_at),
            accepted_at: instant(quote.accepted_at),
            acceptance: quote.acceptance.map(|acceptance| QuoteAcceptanceResponse {
                name: acceptance.name,
                email: acceptance.email,
                ip: acceptance.ip,
                user_agent: acceptance.user_agent,
                accepted_at: instant(quote.accepted_at),
                recorded_by: acceptance.recorded_by.map(UserId::into_inner),
            }),
            rejected_at: instant(quote.rejected_at),
            rejection_reason: quote.rejection_reason,
            expired_at: instant(quote.expired_at),
            deal_synced_at: instant(quote.deal_synced_at),
            deal_sync_error: quote.deal_sync_error,
            versions: details.versions.into_iter().map(Into::into).collect(),
            created_by: quote.created_by.map(UserId::into_inner),
            created_at: timestamp(quote.created_at),
            updated_at: timestamp(quote.updated_at),
            custom_fields: details.custom_fields,
        }
    }
}

#[derive(Debug, Default, Deserialize, ToSchema)]
pub(crate) struct SubmitQuoteRequest {
    #[serde(default)]
    pub(crate) comment: Option<String>,
}

impl SubmitQuoteRequest {
    pub(crate) fn into_command(self, ip: Option<String>) -> SubmitQuoteCommand {
        SubmitQuoteCommand {
            comment: self.comment,
            ip,
        }
    }
}

#[derive(Debug, Default, Deserialize, ToSchema)]
pub(crate) struct AcceptQuoteManuallyRequest {
    #[serde(default)]
    pub(crate) name: Option<String>,
    #[serde(default)]
    pub(crate) email: Option<String>,
    #[serde(default)]
    pub(crate) note: Option<String>,
}

impl AcceptQuoteManuallyRequest {
    pub(crate) fn into_command(self, ip: Option<String>) -> ManualAcceptQuoteCommand {
        ManualAcceptQuoteCommand {
            name: self.name,
            email: self.email,
            note: self.note,
            ip,
        }
    }
}

#[derive(Debug, Default, Deserialize, ToSchema)]
pub(crate) struct SendQuoteRequest {
    #[serde(default)]
    pub(crate) to: Vec<EmailAddressDto>,
    #[serde(default)]
    pub(crate) cc: Vec<EmailAddressDto>,
    #[serde(default)]
    pub(crate) subject: Option<String>,
    #[serde(default)]
    pub(crate) body_html: Option<String>,
    #[serde(default)]
    pub(crate) mailbox_id: Option<String>,
}

impl SendQuoteRequest {
    pub(crate) fn into_command(self, ip: Option<String>) -> SendQuoteCommand {
        SendQuoteCommand {
            to: self.to.into_iter().map(Into::into).collect(),
            cc: self.cc.into_iter().map(Into::into).collect(),
            subject: self.subject,
            body_html: self.body_html,
            mailbox_id: self.mailbox_id,
            ip,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct QuoteLinkResponse {
    pub(crate) url: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct QuoteSettingsResponse {
    pub(crate) default_valid_days: Option<u16>,
    pub(crate) default_terms: Option<String>,
    #[schema(value_type = Object)]
    pub(crate) accept_stages: BTreeMap<String, String>,
    pub(crate) updated_at: Option<String>,
}

impl From<QuoteSettings> for QuoteSettingsResponse {
    fn from(settings: QuoteSettings) -> Self {
        Self {
            default_valid_days: settings.default_valid_days,
            default_terms: settings.default_terms,
            accept_stages: settings
                .accept_stages
                .into_iter()
                .map(|(pipeline, stage)| (pipeline.to_string(), stage.to_string()))
                .collect(),
            updated_at: instant(settings.updated_at),
        }
    }
}

#[derive(Debug, Default, Deserialize, ToSchema)]
pub(crate) struct QuoteSettingsRequest {
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<u16>, example = 15)]
    pub(crate) default_valid_days: Option<Option<u16>>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>)]
    pub(crate) default_terms: Option<Option<String>>,
    #[serde(default)]
    #[schema(value_type = Option<Object>)]
    pub(crate) accept_stages: Option<BTreeMap<String, Option<String>>>,
}

impl QuoteSettingsRequest {
    pub(crate) fn into_command(
        self,
        ip: Option<String>,
        expected_updated_at: Option<DateTime<Utc>>,
    ) -> UpdateQuoteSettingsCommand {
        UpdateQuoteSettingsCommand {
            default_valid_days: self.default_valid_days,
            default_terms: self.default_terms,
            accept_stages: self.accept_stages,
            ip,
            expected_updated_at,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct PublicQuoteOrganization {
    pub(crate) name: String,
    pub(crate) primary_color: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct PublicQuoteItemResponse {
    pub(crate) name: String,
    pub(crate) description: Option<String>,
    pub(crate) unit: Option<String>,
    pub(crate) quantity: String,
    pub(crate) unit_price: String,
    pub(crate) discount_type: QuoteDiscountTypeSchema,
    pub(crate) discount_value: String,
    pub(crate) discount_amount: String,
    pub(crate) tax_percent: String,
    pub(crate) tax_amount: String,
    pub(crate) total: String,
    pub(crate) components: Vec<QuoteItemComponentResponse>,
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct PublicQuoteResponse {
    pub(crate) organization: PublicQuoteOrganization,
    pub(crate) state: PublicQuoteStateSchema,
    pub(crate) number: u32,
    pub(crate) version: u32,
    pub(crate) title: String,
    pub(crate) currency: String,
    pub(crate) valid_until: Option<String>,
    pub(crate) terms: Option<String>,
    pub(crate) notes: Option<String>,
    pub(crate) items: Vec<PublicQuoteItemResponse>,
    pub(crate) subtotal: String,
    pub(crate) items_discount: String,
    pub(crate) discount_type: QuoteDiscountTypeSchema,
    pub(crate) discount_value: String,
    pub(crate) total_discount: String,
    pub(crate) discount_total: String,
    pub(crate) tax_total: String,
    pub(crate) total: String,
    pub(crate) owner_name: Option<String>,
    pub(crate) owner_email: Option<String>,
    pub(crate) contact_name: Option<String>,
    pub(crate) company_name: Option<String>,
    pub(crate) sent_at: Option<String>,
    pub(crate) accepted_at: Option<String>,
    pub(crate) accepted_name: Option<String>,
    pub(crate) rejected_at: Option<String>,
    pub(crate) rejection_reason: Option<String>,
}

impl From<PublicQuote> for PublicQuoteResponse {
    fn from(public: PublicQuote) -> Self {
        let quote = public.quote;
        Self {
            organization: PublicQuoteOrganization {
                name: public.organization.name,
                primary_color: public.organization.primary_color,
            },
            state: public.state.into(),
            number: quote.number,
            version: quote.version,
            title: quote.title,
            currency: quote.currency,
            valid_until: date(quote.valid_until),
            terms: quote.terms,
            notes: quote.notes,
            items: public
                .items
                .into_iter()
                .map(|item| {
                    let response = QuoteItemResponse::from(item);
                    PublicQuoteItemResponse {
                        name: response.name,
                        description: response.description,
                        unit: response.unit,
                        quantity: response.quantity,
                        unit_price: response.unit_price,
                        discount_type: response.discount_type,
                        discount_value: response.discount_value,
                        discount_amount: response.discount_amount,
                        tax_percent: response.tax_percent,
                        tax_amount: response.tax_amount,
                        total: response.total,
                        components: response.components,
                    }
                })
                .collect(),
            subtotal: money(quote.subtotal),
            items_discount: money(public.items_discount),
            discount_type: quote.discount.kind.into(),
            discount_value: money(quote.discount.value),
            total_discount: money(public.total_discount),
            discount_total: money(quote.discount_total),
            tax_total: money(quote.tax_total),
            total: money(quote.total),
            owner_name: public.owner_name,
            owner_email: public.owner_email,
            contact_name: public.contact_name,
            company_name: public.company_name,
            sent_at: instant(quote.sent_at),
            accepted_at: instant(quote.accepted_at),
            accepted_name: quote.acceptance.map(|acceptance| acceptance.name),
            rejected_at: instant(quote.rejected_at),
            rejection_reason: quote.rejection_reason,
        }
    }
}

#[derive(Debug, Default, Deserialize, ToSchema)]
pub(crate) struct PublicAcceptQuoteRequest {
    #[schema(example = "Maria Souza")]
    pub(crate) name: String,
    #[schema(example = "maria@customer.com")]
    pub(crate) email: String,
}

impl PublicAcceptQuoteRequest {
    pub(crate) fn into_command(
        self,
        ip: Option<String>,
        user_agent: Option<String>,
    ) -> AcceptQuoteCommand {
        AcceptQuoteCommand {
            name: self.name,
            email: self.email,
            ip,
            user_agent,
        }
    }
}

#[derive(Debug, Default, Deserialize, ToSchema)]
pub(crate) struct PublicRejectQuoteRequest {
    #[serde(default)]
    pub(crate) reason: Option<String>,
}

impl PublicRejectQuoteRequest {
    pub(crate) fn into_command(self, ip: Option<String>) -> RejectQuoteCommand {
        RejectQuoteCommand {
            reason: self.reason,
            ip,
        }
    }
}

pub(crate) struct QuoteHandler;

#[async_trait]
impl RecordTypeHandler for QuoteHandler {
    fn durable_mutations(&self) -> bool {
        true
    }

    fn record_type(&self) -> &str {
        "quotes.quote"
    }

    async fn create(&self, ctx: &RecordCtx<'_>, values: Value) -> Result<Value, AppError> {
        let command = values_request::<QuoteRequest>(values)?.into_create(None);
        let details =
            service::create(&Ctx::from_state(ctx.state, ctx.scope).await?, command).await?;
        quote_detail(ctx.state, ctx.scope, details).await
    }

    async fn update(
        &self,
        ctx: &RecordCtx<'_>,
        id: Uuid,
        values: Value,
    ) -> Result<Value, AppError> {
        let command = values_request::<QuoteRequest>(values)?.into_update(None, None);
        let details = service::update(
            &Ctx::from_state(ctx.state, ctx.scope).await?,
            &id.to_string(),
            command,
        )
        .await?;
        quote_detail(ctx.state, ctx.scope, details).await
    }

    async fn delete(&self, ctx: &RecordCtx<'_>, id: Uuid) -> Result<(), AppError> {
        service::delete(
            &Ctx::from_state(ctx.state, ctx.scope).await?,
            &id.to_string(),
            None,
        )
        .await
    }

    async fn load(&self, ctx: &RecordCtx<'_>, id: Uuid) -> Result<Option<Value>, AppError> {
        let module_ctx = Ctx::from_state(ctx.state, ctx.scope).await?;
        let Some(details) = optional_record(service::get(&module_ctx, &id.to_string()).await)?
        else {
            return Ok(None);
        };
        Ok(Some(quote_detail(ctx.state, ctx.scope, details).await?))
    }
}
