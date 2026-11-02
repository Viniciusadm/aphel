use axum::Json;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::api::dto::common::{Data, ErrorEnvelope};
use crate::api::error::ApiError;
use crate::api::extractors::AuthContext;
use crate::api::state::AppState;
use crate::application::access::Authorizer;
use crate::domain::access::Permission;
use crate::modules::unified_catalog::{self, CatalogDocument};

pub const TAG: &str = "system";

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new().routes(routes!(platform_catalog))
}

#[utoipa::path(
    get,
    path = "/platform/catalog",
    tag = TAG,
    operation_id = "platform_catalog",
    security(("session_cookie" = [])),
    description = "Everything this build offers to plugins in one document (permission `system.manage`): plugins with their layer and runtime, operations, hooks with their listeners, events, permissions and notification kinds. `enabled` is false when the owning plugin is disabled.",
    responses(
        (status = 200, description = "The platform catalog", body = Data<CatalogDocument>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `system.manage` (`permission_denied`)", body = ErrorEnvelope),
    )
)]
async fn platform_catalog(tenant: AuthContext) -> Result<Json<Data<CatalogDocument>>, ApiError> {
    tenant.scope.require(&Permission::SYSTEM_MANAGE)?;
    Ok(Json(Data::new(unified_catalog::build(
        crate::modules::app(),
    ))))
}
