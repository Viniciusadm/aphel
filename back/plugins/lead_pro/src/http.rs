use axum::Json;
use axum::extract::{Path, State};
use chrono::{DateTime, SecondsFormat, Utc};
use serde::Serialize;
use serde_json::{Value, json};
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;
use uuid::Uuid;

use aphel_kernel::Ctx;
use aphel_kernel::api::error::ApiError;
use aphel_kernel::api::extractors::AuthContext;
use aphel_kernel::application::access::Authorizer;
use aphel_kernel::application::organization_admin::ListWindow;
use aphel_kernel::application::query::{
    Filter, FilterOperator, RecordQuery, SortDirection, SortSpec,
};
use aphel_kernel::errors::AppError;
use aphel_kernel::state::AppState;

use crate::domain::{LeadProSettings, MANUAL_TOUCH, READ, TEMPERATURE_FIELD};
use crate::store;
use crate::touches::heat;

pub(crate) const TAG: &str = "lead_pro";
const TOUCHES_SHOWN: u32 = 20;
const BOARD_SIZE: u64 = 50;

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct LeadTouchResponse {
    pub(crate) id: String,
    pub(crate) kind: String,
    pub(crate) created_at: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct LeadTemperatureResponse {
    pub(crate) contact_id: String,
    pub(crate) temperature: Option<i64>,
    pub(crate) hot_threshold: i64,
    pub(crate) hot: bool,
    pub(crate) touches: Vec<LeadTouchResponse>,
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct LeadTemperatureEnvelope {
    pub(crate) data: LeadTemperatureResponse,
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct HotLeadResponse {
    pub(crate) contact_id: String,
    pub(crate) name: String,
    pub(crate) email: Option<String>,
    pub(crate) temperature: Option<i64>,
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct HotLeadList {
    pub(crate) data: Vec<HotLeadResponse>,
    pub(crate) hot_threshold: i64,
}

pub(crate) fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(board))
        .routes(routes!(temperature))
        .routes(routes!(register_touch))
}

fn timestamp(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(SecondsFormat::Secs, true)
}

fn contact_id(raw: &str) -> Result<Uuid, AppError> {
    Uuid::parse_str(raw.trim()).map_err(|_| AppError::NotFound("contact not found".to_owned()))
}

async fn temperature_of(ctx: &Ctx<'_>, id: Uuid) -> Result<LeadTemperatureResponse, AppError> {
    readable_contact(ctx, id).await?;
    let settings = LeadProSettings::load(ctx.pool()).await?;
    let mut connection = ctx.pool().acquire().await?;
    let temperature = store::temperature(&mut connection, id).await?;
    let touches = store::touches(ctx.pool(), id, TOUCHES_SHOWN).await?;
    Ok(LeadTemperatureResponse {
        contact_id: id.to_string(),
        temperature,
        hot_threshold: settings.hot_threshold,
        hot: settings.is_hot(temperature),
        touches: touches
            .into_iter()
            .map(|touch| LeadTouchResponse {
                id: touch.id.to_string(),
                kind: touch.kind,
                created_at: timestamp(touch.created_at),
            })
            .collect(),
    })
}

#[utoipa::path(
    get,
    path = "/lead-pro/board",
    tag = TAG,
    operation_id = "lead_pro_board",
    security(("session_cookie" = [])),
    description = "Lists the hottest leads the user can see (people at or above the hot threshold of the `lead_pro` settings), hottest first. Requires `lead_pro.read`.",
    responses(
        (status = 200, description = "Hot leads", body = HotLeadList),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`)"),
        (status = 403, description = "Missing `lead_pro.read` (`permission_denied`)"),
        (status = 404, description = "Module disabled (`module_disabled`)"),
    )
)]
async fn board(
    State(state): State<AppState>,
    tenant: AuthContext,
) -> Result<Json<HotLeadList>, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    ctx.scope.require(&READ)?;
    let settings = LeadProSettings::load(ctx.pool()).await?;
    let listing = aphel_sdk::record_context::list(
        &ctx,
        aphel_sdk::people::RECORD_TYPE,
        RecordQuery {
            filter: Some(Filter::condition(
                TEMPERATURE_FIELD,
                FilterOperator::Gte,
                json!(settings.hot_threshold),
            )),
            sort: vec![SortSpec {
                field: TEMPERATURE_FIELD.to_owned(),
                direction: SortDirection::Desc,
            }],
            window: ListWindow {
                offset: 0,
                limit: BOARD_SIZE,
            },
            ..RecordQuery::default()
        },
    )
    .await?;
    Ok(Json(HotLeadList {
        data: listing
            .items
            .into_iter()
            .filter_map(|record| {
                let id = record.get("id")?.as_str()?;
                Some(HotLeadResponse {
                    contact_id: id.to_owned(),
                    name: record
                        .get("full_name")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_owned(),
                    email: record
                        .get("email")
                        .and_then(Value::as_str)
                        .map(str::to_owned),
                    temperature: record.get(TEMPERATURE_FIELD).and_then(Value::as_i64),
                })
            })
            .collect(),
        hot_threshold: settings.hot_threshold,
    }))
}

#[utoipa::path(
    get,
    path = "/lead-pro/people/{contact_id}",
    tag = TAG,
    operation_id = "lead_pro_contact_temperature",
    security(("session_cookie" = [])),
    params(("contact_id" = String, Path, description = "Contact id")),
    description = "Temperature of a lead, the hot threshold and its latest touches (tasks, notes, e-mails, meetings, manual touches and the conversion). Requires `lead_pro.read` and read access to the contact.",
    responses(
        (status = 200, description = "Temperature of the lead", body = LeadTemperatureEnvelope),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`)"),
        (status = 403, description = "Missing `lead_pro.read` (`permission_denied`)"),
        (status = 404, description = "Contact not found or module disabled (`module_disabled`)"),
    )
)]
async fn temperature(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(contact): Path<String>,
) -> Result<Json<LeadTemperatureEnvelope>, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    ctx.scope.require(&READ)?;
    Ok(Json(LeadTemperatureEnvelope {
        data: temperature_of(&ctx, contact_id(&contact)?).await?,
    }))
}

#[utoipa::path(
    post,
    path = "/lead-pro/people/{contact_id}/touches",
    tag = TAG,
    operation_id = "lead_pro_contact_touch",
    security(("session_cookie" = [])),
    params(("contact_id" = String, Path, description = "Contact id")),
    description = "Registers a manual touch on the lead: the temperature goes up by the `touch_points` setting and `lead_pro.temperature_changed` is published. Requires `lead_pro.read` and read access to the contact.",
    responses(
        (status = 200, description = "Temperature after the touch", body = LeadTemperatureEnvelope),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`)"),
        (status = 403, description = "Missing `lead_pro.read` (`permission_denied`)"),
        (status = 404, description = "Contact not found or module disabled (`module_disabled`)"),
    )
)]
async fn register_touch(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(contact): Path<String>,
) -> Result<Json<LeadTemperatureEnvelope>, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    ctx.scope.require(&READ)?;
    let id = contact_id(&contact)?;
    readable_contact(&ctx, id).await?;
    heat(&ctx, id, MANUAL_TOUCH, None).await?;
    Ok(Json(LeadTemperatureEnvelope {
        data: temperature_of(&ctx, id).await?,
    }))
}

async fn readable_contact(ctx: &Ctx<'_>, id: Uuid) -> Result<(), AppError> {
    ctx.load_record(aphel_sdk::people::RECORD_TYPE, id)
        .await?
        .ok_or_else(|| AppError::NotFound("contact not found".into()))?;
    Ok(())
}
