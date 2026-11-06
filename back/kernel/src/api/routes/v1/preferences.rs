use axum::Json;
use axum::extract::State;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::api::dto::common::{Data, ErrorEnvelope};
use crate::api::dto::preferences::{PreferencesResponse, UpdatePreferencesRequest};
use crate::api::error::ApiError;
use crate::api::extractors::{AuthContext, JsonBody};
use crate::api::state::AppState;
use crate::application::user_preferences::UserPreferencesUseCases;
use crate::errors::AppError;
use crate::infrastructure::user_preferences::MySqlUserPreferencesPorts;

pub const TAG: &str = "preferences";

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new().routes(routes!(get_preferences, update_preferences))
}

fn preferences_use_cases(
    state: &AppState,
) -> Result<UserPreferencesUseCases<MySqlUserPreferencesPorts>, AppError> {
    Ok(UserPreferencesUseCases::new(
        MySqlUserPreferencesPorts::new(state.db()?),
    ))
}

#[utoipa::path(
    get,
    path = "/preferences",
    tag = TAG,
    operation_id = "preferences_get",
    security(("session_cookie" = [])),
    responses(
        (status = 200, description = "Personal preferences of the authenticated current user. Navigation and experience contain policy-permitted values; stored_navigation and stored_experience retain saved choices, including ignored/revoked references, with experience_diagnostics. Defaults when never saved. Effective profile identity is not verification of frontend provider availability.", body = Data<PreferencesResponse>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
    )
)]
async fn get_preferences(
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<Data<PreferencesResponse>>, ApiError> {
    let preferences = preferences_use_cases(&state)?.get(&auth.user.id).await?;
    Ok(Json(Data::new(preferences.into())))
}

#[utoipa::path(
    patch,
    path = "/preferences",
    tag = TAG,
    operation_id = "preferences_update",
    security(("session_cookie" = [])),
    request_body = UpdatePreferencesRequest,
    responses(
        (status = 200, description = "Saved preferences; omitted properties stay unchanged. `keyboard_shortcuts = false` turns off the single-key shortcuts of the web interface (`Ctrl/Cmd+K` keeps working). `locale` (`pt-BR` or `en`, `null` = follow the organization) sets the language of the interface and of the e-mails sent to the user. `navigation` replaces the sidebar order, the hidden items (item keys; items the user cannot see stay hidden and unknown items are appended by the client) and the home screen (an internal path such as `/deals`). experience changes profile selection and record-detail order/hidden source pairs within organization policy. Omitted experience fields stay unchanged; experience.profile = null resets only selection, experience.record_detail = null resets only composition, and experience = null clears both. Resetting navigation uses its empty replacement object. Blocked non-reset writes, unsupported surfaces/sources and omission without owner-declared optionality are rejected on this same endpoint. Saved blocked choices are retained but not applied. Policy and user rows are locked in one transaction; old payloads do not clear new fields. Personal preferences: no domain event or audit entry", body = Data<PreferencesResponse>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 422, description = "Invalid body, unsupported locale, unsafe navigation or organization-policy violations in navigation/experience (validation_error with field paths)", body = ErrorEnvelope),
    )
)]
async fn update_preferences(
    State(state): State<AppState>,
    auth: AuthContext,
    JsonBody(request): JsonBody<UpdatePreferencesRequest>,
) -> Result<Json<Data<PreferencesResponse>>, ApiError> {
    let preferences = preferences_use_cases(&state)?
        .update(&auth.user.id, request.into())
        .await?;
    Ok(Json(Data::new(preferences.into())))
}
