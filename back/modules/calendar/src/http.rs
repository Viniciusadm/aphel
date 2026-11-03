use aphel_kernel::api::{
    dto::common::{Data, ErrorEnvelope},
    error::ApiError,
    extractors::{AuthContext, ClientInfo, IfMatch, JsonBody},
    state::AppState,
};
use aphel_kernel::application::access::Authorizer;
use aphel_kernel::application::organization_profile::OrganizationRepository;
use aphel_kernel::domain::access::Permission;
use aphel_sdk::calendar::{CalendarCategory, CalendarQuery, CalendarRecord, CalendarSource};
use aphel_sdk::{AppError, Ctx};
use axum::{
    Json,
    extract::{Path, Query, State},
};
use chrono::{DateTime, Duration};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};
const MAX_ITEMS: usize = 1000;
pub(crate) fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(get_calendar))
        .routes(routes!(reschedule))
}
#[derive(Serialize, ToSchema)]
struct CalendarResponse {
    categories: Vec<CalendarCategory>,
    from: String,
    to: String,
    timezone: String,
    total: u64,
    truncated: bool,
    items: Vec<CalendarRecord>,
}
#[utoipa::path(get, path="/calendar", tag="calendar", operation_id="calendar_get", security(("session_cookie"=[])), params(("from"=String,Query),("to"=String,Query),("owner_id"=Option<String>,Query),("team_id"=Option<String>,Query),("category_id"=Option<String>,Query),("source"=Option<String>,Query),("status"=Option<String>,Query)), responses((status=200,description="Authorized entries from enabled calendar sources, sorted by start time; range at most 62 days, at most 1000 entries",body=Data<CalendarResponse>),(status=422,description="Invalid range or source filter",body=ErrorEnvelope)))]
async fn get_calendar(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(query): Query<CalendarQuery>,
) -> Result<Json<Data<CalendarResponse>>, ApiError> {
    let ctx = Ctx::from_state(&state, &auth.scope).await?;
    ctx.scope
        .require(&Permission::from_static("calendar.read"))?;
    let from = DateTime::parse_from_rfc3339(&query.from)
        .map_err(|_| AppError::Validation("invalid calendar start".into()))?;
    let to = DateTime::parse_from_rfc3339(&query.to)
        .map_err(|_| AppError::Validation("invalid calendar end".into()))?;
    if to <= from || to - from > Duration::days(62) {
        return Err(AppError::Validation(
            "calendar range must be positive and at most 62 days".into(),
        )
        .into());
    }
    let sources = state.app.contributions::<dyn CalendarSource>();
    if query
        .source
        .as_ref()
        .is_some_and(|key| !sources.iter().any(|source| source.key() == key))
    {
        return Err(AppError::Validation("unknown calendar source".into()).into());
    }
    let mut categories = std::collections::BTreeMap::new();
    let mut items = Vec::new();
    let mut total = 0;
    let mut truncated = false;
    for source in sources {
        for mut category in source.categories(&ctx).await? {
            category.source = source.key().to_owned();
            categories.insert((category.source.clone(), category.id.clone()), category);
        }

        if query
            .source
            .as_deref()
            .is_some_and(|key| key != source.key())
        {
            continue;
        }
        let batch = source.entries(&ctx, &query).await?;
        total += batch.total;
        truncated |= batch.truncated;
        for mut item in batch.items {
            let start = DateTime::parse_from_rfc3339(&item.due_at).map_err(|_| {
                AppError::Configuration(format!(
                    "calendar source {} returned an invalid start time",
                    source.key()
                ))
            })?;
            if start < from || start >= to {
                continue;
            }
            item.source = source.key().to_owned();
            item.id = format!("{}:{}", source.key(), item.record_id);
            item.category.source = source.key().to_owned();
            item.due_at = start
                .with_timezone(&chrono::Utc)
                .to_rfc3339_opts(chrono::SecondsFormat::Micros, true);
            items.push(item);
        }
    }
    items.sort_by(|a, b| a.due_at.cmp(&b.due_at).then(a.id.cmp(&b.id)));
    truncated |= items.len() > MAX_ITEMS;
    items.truncate(MAX_ITEMS);
    let timezone = aphel_kernel::infrastructure::repositories::mysql_organization_profile::MySqlOrganizationRepository::new(ctx.pool().clone()).find().await?;
    Ok(Json(Data::new(CalendarResponse {
        categories: categories.into_values().collect(),
        from: query.from,
        to: query.to,
        timezone: timezone
            .map(|profile| profile.timezone)
            .unwrap_or_else(|| "UTC".into()),
        total,
        truncated,
        items,
    })))
}
#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
struct RescheduleRequest {
    due_at: String,
}
#[utoipa::path(patch,path="/calendar/{source}/{id}",tag="calendar",operation_id="calendar_reschedule",security(("session_cookie"=[])),params(("source"=String,Path),("id"=String,Path)),request_body=RescheduleRequest,responses((status=204,description="Entry rescheduled by its owner; owner permissions and version checks apply"),(status=409,description="Version conflict",body=ErrorEnvelope)))]
async fn reschedule(
    State(state): State<AppState>,
    auth: AuthContext,
    Path((source, id)): Path<(String, String)>,
    client: ClientInfo,
    version: IfMatch,
    JsonBody(request): JsonBody<RescheduleRequest>,
) -> Result<axum::http::StatusCode, ApiError> {
    let ctx = Ctx::from_state(&state, &auth.scope).await?;
    ctx.scope
        .require(&Permission::from_static("calendar.read"))?;
    let source = state
        .app
        .contributions::<dyn CalendarSource>()
        .into_iter()
        .find(|entry| entry.key() == source)
        .ok_or_else(|| AppError::NotFound("calendar source not found".into()))?;
    source
        .reschedule(&ctx, &id, &request.due_at, version.0, client.0.ip)
        .await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}
