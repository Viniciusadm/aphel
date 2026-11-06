use axum::Json;
use axum::extract::State;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::json;
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::api::dto::common::{Data, ErrorEnvelope, double_option};
use crate::api::error::ApiError;
use crate::api::extractors::{AuthContext, ClientInfo, JsonBody};
use crate::api::state::AppState;
use crate::application::access::Authorizer;
use crate::application::organization_profile::{
    UI_PLUGIN_UPDATED, is_ui_plugin_key, ui_plugin_available,
};
use crate::application::shared::{
    Actor, AuditEntry, AuditLog, DomainEvent, EventPublisher, RecordRef, TransactionManager,
};
use crate::domain::access::Permission;
use crate::errors::{AppError, FieldViolation};
use crate::infrastructure::organization_profile::save_ui_plugin_default;
use crate::infrastructure::transactions::MySqlTransactionManager;

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new().routes(routes!(set_default_ui_plugin))
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SetUiPluginRequest {
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>, nullable = true, example = "midnight")]
    pub ui_plugin: Option<Option<String>>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct UiPluginDefaultResponse {
    pub ui_plugin: Option<String>,
}

#[utoipa::path(
    put,
    path = "/organization/ui-plugin",
    tag = "organization",
    operation_id = "organization_ui_plugin_set",
    description = "Sets the interface plugin used by everyone who has not chosen one, and by pages shown before sign-in. Requires settings.manage. The value is the key of an enabled plugin that declares an interface, or null for the core interface. Writes organization.ui_plugin_updated and audit in the same transaction. It does not install or enable code.",
    security(("session_cookie" = [])),
    request_body = SetUiPluginRequest,
    responses(
        (status = 200, description = "Saved default", body = Data<UiPluginDefaultResponse>),
        (status = 401, description = "Missing or invalid credentials", body = ErrorEnvelope),
        (status = 403, description = "Missing settings.manage", body = ErrorEnvelope),
        (status = 422, description = "The plugin is not an enabled interface plugin", body = ErrorEnvelope),
    )
)]
async fn set_default_ui_plugin(
    State(state): State<AppState>,
    tenant: AuthContext,
    ClientInfo(client): ClientInfo,
    JsonBody(request): JsonBody<SetUiPluginRequest>,
) -> Result<Json<Data<UiPluginDefaultResponse>>, ApiError> {
    tenant.scope.require(&Permission::SETTINGS_MANAGE)?;
    let key = request.ui_plugin.flatten();
    if let Some(key) = &key
        && (!is_ui_plugin_key(key) || !ui_plugin_available(key).await?)
    {
        return Err(AppError::invalid_fields(
            "invalid interface plugin",
            vec![FieldViolation::new(
                "ui_plugin",
                "must be an enabled plugin that provides an interface",
            )],
        )
        .into());
    }
    let transactions = MySqlTransactionManager::new(state.db()?);
    let mut tx = transactions.begin().await?;
    let previous = save_ui_plugin_default(&mut tx, key.as_deref()).await?;
    if previous != key {
        let now = Utc::now();
        let actor = Actor::from_scope(&tenant.scope);
        let payload = json!({ "ui_plugin": key, "previous": previous });
        state
            .events
            .publish(
                &mut tx,
                DomainEvent::new(actor.clone(), UI_PLUGIN_UPDATED, now)
                    .with_record(RecordRef::organization())
                    .with_payload(payload),
            )
            .await?;
        state
            .audit
            .record(
                &mut tx,
                AuditEntry::new(actor, UI_PLUGIN_UPDATED, now)
                    .with_record(RecordRef::organization())
                    .with_ip(client.ip),
            )
            .await?;
    }
    transactions.commit(tx).await?;
    Ok(Json(Data::new(UiPluginDefaultResponse { ui_plugin: key })))
}
