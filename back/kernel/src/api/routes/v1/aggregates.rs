use axum::Json;
use axum::extract::{Path, State};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::api::dto::aggregates::{
    AggregateQueryRequest, AggregateResultResponse, AggregateSchemaResponse,
};
use crate::api::dto::common::{Data, ErrorEnvelope};
use crate::api::error::ApiError;
use crate::api::extractors::{AuthContext, JsonBody};
use crate::api::state::AppState;
use crate::application::query::AggregateEngine;
use crate::errors::AppError;
use crate::infrastructure::query::MySqlRecordQueryPorts;

pub const TAG: &str = "aggregates";

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(run_aggregate))
        .routes(routes!(aggregate_schema))
}

pub async fn aggregate_ports(state: &AppState) -> Result<MySqlRecordQueryPorts, AppError> {
    Ok(MySqlRecordQueryPorts::new(
        state.db()?,
        state.registry().await?,
    ))
}

#[utoipa::path(
    post,
    path = "/aggregates",
    tag = TAG,
    operation_id = "aggregates_run",
    security(("session_cookie" = [])),
    description = "Aggregates records of a type under the read scope of the caller (`own`/`team`/`organization`) and the field permissions of the role, in the time zone of the organization. `group_by` (up to 3) accepts system and custom fields (select, multi-select and user included), `tags`, `owner_team` (team of the owner; an owner in two teams counts in both) and fields of a single related record such as `company.industry` on deals; date fields take a `granularity` (`day`, `week` starting on Monday, `month`, `quarter`, `year`, default `day`) and their bucket key is the first day of the bucket (`YYYY-MM-DD`). `metrics` (up to 10, default `[{\"fn\":\"count\"}]`): `count` (distinct records, or records with a value when `field` is given) and `sum`/`avg`/`min`/`max` over number or currency fields of the type itself; the key of a metric is `fn` or `fn:field`. A duration metric takes two date or date and time fields of the type itself, `field` (start) and `to` (end), and a `unit` (`days`, the default, or `hours`): it applies `fn` (`count`/`sum`/`avg`/`min`/`max`) to the time between both dates of each record (fractional, records missing either date are skipped), such as `{\"fn\":\"avg\",\"field\":\"created_at\",\"to\":\"closed_at\"}` for the average days to close; its key is `fn:field..to:unit`. `filter` is the same AST of the listings and also accepts conditions on related records (`company.industry`, `deals.status` on organization_profile), each evaluated as \"there is a related record visible to the caller that matches\" so totals are never duplicated, and `owner_team` with `in`/`not_in`/`is_empty`/`is_not_empty`. `period` limits a date field to a preset (`today`, `yesterday`, `last_7_days`, `last_30_days`, `week`, `last_week`, `month`, `last_month`, `quarter`, `last_quarter`, `year`, `last_year`) or a `custom` range (`from`/`to`, at most 731 days); with `compare_to_previous` every row and the totals also carry `previous`, computed over the period right before (whole months shift by months, any other range by its length) and matched bucket by bucket when grouping by the period field. Rows grouped only by the period field fill empty buckets with zero. `sort` takes group fields or metric keys (default: groups ascending, select options in the order of their position, empty values last); `limit` defaults to 500 (max 1000) and `truncated` tells when more groups exist. Values are decimal strings; sums ignore the currency of each record.",
    request_body = AggregateQueryRequest,
    responses(
        (status = 200, description = "Aggregated rows, totals and the resolved periods", body = Data<AggregateResultResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing the read permission of the record type (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Unknown record type (`record_type_not_found`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid group, metric, period, sort or filter (`validation_error` with `details.fields`), or a hidden field (`field_not_accessible`)", body = ErrorEnvelope),
    )
)]
async fn run_aggregate(
    State(state): State<AppState>,
    tenant: AuthContext,
    JsonBody(request): JsonBody<AggregateQueryRequest>,
) -> Result<Json<Data<AggregateResultResponse>>, ApiError> {
    let ports = aggregate_ports(&state).await?;
    let result = AggregateEngine::new(&ports)
        .run(&tenant.scope, request.into_query())
        .await?;
    Ok(Json(Data::new(result.into())))
}

#[utoipa::path(
    get,
    path = "/aggregates/{record_type}/schema",
    tag = TAG,
    operation_id = "aggregates_schema",
    security(("session_cookie" = [])),
    description = "Fields of a record type that can be grouped (`groupable`, `temporal` for date fields that take a granularity) or aggregated (`functions`), the virtual fields `tags` and `owner_team`, and the relations usable in filters (`relation.field`) and, when `single`, in groups. Hidden fields of the role are left out; `readable` tells whether the caller can read the related type.",
    params(("record_type" = String, Path, description = "Record type, such as `deal`")),
    responses(
        (status = 200, description = "Aggregation schema of the record type", body = Data<AggregateSchemaResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing the read permission of the record type (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Unknown record type (`record_type_not_found`)", body = ErrorEnvelope),
    )
)]
async fn aggregate_schema(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(record_type): Path<String>,
) -> Result<Json<Data<AggregateSchemaResponse>>, ApiError> {
    let ports = aggregate_ports(&state).await?;
    let schema = AggregateEngine::new(&ports)
        .describe(&tenant.scope, &record_type)
        .await?;
    Ok(Json(Data::new(schema.into())))
}
