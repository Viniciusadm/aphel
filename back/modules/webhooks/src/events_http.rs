use axum::Json;
use axum::extract::State;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::events_dto::PublicEventTypeResponse;
use aphel_sdk::platform::api::dto::common::{Data, ErrorEnvelope};
use aphel_sdk::platform::api::error::ApiError;
use aphel_sdk::platform::api::state::AppState;
use aphel_sdk::platform::application::public_events::{
    CUSTOM_EVENT_PATTERN_PREFIX, custom_public_events, public_event_catalog,
};

pub(crate) const TAG: &str = "events";

pub(crate) fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new().routes(routes!(event_catalog))
}

#[utoipa::path(
    get,
    path = "/events/catalog",
    tag = TAG,
    operation_id = "events_catalog",
    security(("session_cookie" = [])),
    description = "Stable list of public event types delivered by outgoing webhooks. Each delivery body is `{ id, type, actor: { type, id }, occurred_at, data, previous? }`: `data` is the resource as returned by its `GET /{resource}/{id}` (schema in `data_schema`; null when the event carries its own payload instead of a record), or only `{ id }` when `data_is_id_only`; `previous` carries the old values of the changed fields when `has_previous`. Any authenticated member can read it. The list also has the events of each active custom object (`custom.<key>.created|updated|deleted|archived|unarchived`, data schema `CustomRecordResponse`).",
    responses(
        (status = 200, description = "Public event types", body = Data<Vec<PublicEventTypeResponse>>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
    )
)]
async fn event_catalog(
    State(state): State<AppState>,
) -> Result<Json<Data<Vec<PublicEventTypeResponse>>>, ApiError> {
    aphel_sdk::platform::application::public_events::refresh_public_events(&state.db()?).await?;
    let mut events: Vec<PublicEventTypeResponse> = public_event_catalog()
        .map(|definition| (&definition).into())
        .collect();
    {
        let registry = state.registry().await?;
        for object in registry
            .custom_schema()
            .objects
            .iter()
            .filter(|object| object.data.is_active)
        {
            let record_type = object.record_type();
            for definition in custom_public_events() {
                let mut response = PublicEventTypeResponse::from(definition);
                response.event_type =
                    definition
                        .event_type
                        .replacen(CUSTOM_EVENT_PATTERN_PREFIX, &record_type, 1);
                response.resource = record_type.clone();
                response.description =
                    format!("{} ({})", definition.description, object.data.label_plural);
                events.push(response);
            }
        }
    }
    Ok(Json(Data::new(events)))
}
