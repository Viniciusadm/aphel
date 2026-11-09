use std::collections::BTreeMap;

use axum::Json;
use axum::extract::State;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::json;
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::api::dto::common::{Data, ErrorEnvelope};
use crate::api::error::ApiError;
use crate::api::extractors::{AuthContext, ClientInfo, JsonBody};
use crate::api::state::AppState;
use crate::application::access::Authorizer;
use crate::application::keybindings::{KEYBINDINGS_UPDATED, normalize_keybindings};
use crate::application::shared::{
    Actor, AuditEntry, AuditLog, DomainEvent, EventPublisher, RecordRef, TransactionManager,
};
use crate::domain::access::Permission;
use crate::errors::AppError;
use crate::infrastructure::organization_profile::save_organization_keybindings;
use crate::infrastructure::transactions::MySqlTransactionManager;

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new().routes(routes!(set_organization_keybindings))
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SetKeybindingsRequest {
    #[schema(example = json!({ "pos.pay": ["f10"], "core.create": [] }))]
    pub keybindings: BTreeMap<String, Vec<String>>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct KeybindingsResponse {
    pub keybindings: BTreeMap<String, Vec<String>>,
}

#[utoipa::path(
    put,
    path = "/organization/keybindings",
    tag = "organization",
    operation_id = "organization_keybindings_set",
    description = "Replaces the organization keyboard shortcuts that apply to everyone who has not overridden an action. Requires settings.manage. The body maps action ids to key combinations (`mod+k`, `alt+d`, `f9`, `g h`); `[]` disables the action and actions not listed use their own default. Everyone reads the result as `keybindings_default` in GET /preferences. Writes organization.keybindings_updated and audit in the same transaction.",
    security(("session_cookie" = [])),
    request_body = SetKeybindingsRequest,
    responses(
        (status = 200, description = "Saved organization shortcuts", body = Data<KeybindingsResponse>),
        (status = 401, description = "Missing or invalid credentials", body = ErrorEnvelope),
        (status = 403, description = "Missing settings.manage", body = ErrorEnvelope),
        (status = 422, description = "Invalid action id or key combination (validation_error with field paths)", body = ErrorEnvelope),
    )
)]
async fn set_organization_keybindings(
    State(state): State<AppState>,
    tenant: AuthContext,
    ClientInfo(client): ClientInfo,
    JsonBody(request): JsonBody<SetKeybindingsRequest>,
) -> Result<Json<Data<KeybindingsResponse>>, ApiError> {
    tenant.scope.require(&Permission::SETTINGS_MANAGE)?;
    let mut violations = Vec::new();
    let keybindings = normalize_keybindings("keybindings", request.keybindings, &mut violations);
    if !violations.is_empty() {
        return Err(AppError::invalid_fields("invalid keybindings", violations).into());
    }
    let transactions = MySqlTransactionManager::new(state.db()?);
    let mut tx = transactions.begin().await?;
    let previous = save_organization_keybindings(&mut tx, &keybindings).await?;
    if previous != keybindings {
        let now = Utc::now();
        let actor = Actor::from_scope(&tenant.scope);
        let payload = json!({ "keybindings": keybindings, "previous": previous });
        state
            .events
            .publish(
                &mut tx,
                DomainEvent::new(actor.clone(), KEYBINDINGS_UPDATED, now)
                    .with_record(RecordRef::organization())
                    .with_payload(payload),
            )
            .await?;
        state
            .audit
            .record(
                &mut tx,
                AuditEntry::new(actor, KEYBINDINGS_UPDATED, now)
                    .with_record(RecordRef::organization())
                    .with_ip(client.ip),
            )
            .await?;
    }
    transactions.commit(tx).await?;
    Ok(Json(Data::new(KeybindingsResponse { keybindings })))
}
