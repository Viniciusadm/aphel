use axum::Json;
use axum::extract::{Path, State};
use serde::Serialize;
use serde_json::Value;
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use aphel_sdk::platform::api::dto::common::{Data, ErrorEnvelope};
use aphel_sdk::platform::api::error::ApiError;
use aphel_sdk::platform::api::extractors::AuthContext;
use aphel_sdk::platform::api::state::AppState;
use aphel_sdk::platform::application::access::Authorizer;
use aphel_sdk::platform::domain::access::Permission;
use aphel_sdk::records::RecordCtx;

pub(crate) const TAG: &str = "media";

pub(crate) fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new().routes(routes!(asset_usage))
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct MediaUsageResponse {
    #[schema(example = "content.entry")]
    pub record_type: String,
    pub record_id: String,
    #[schema(example = "Entradas")]
    pub type_label: String,
    pub label: Option<String>,
}

#[utoipa::path(
    get,
    path = "/media/assets/{id}/usage",
    tag = TAG,
    operation_id = "media_asset_usage",
    description = "Records that use the file of a library asset (it is attached to them), with the record title when the caller can read it. An asset in use cannot be deleted (`media_asset_in_use`). Requires `media.assets.read`.",
    params(("id" = String, Path, description = "Asset id")),
    security(("session_cookie" = [])),
    responses(
        (status = 200, description = "Records using the asset", body = Data<Vec<MediaUsageResponse>>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
        (status = 403, description = "Missing `media.assets.read` (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Unknown asset", body = ErrorEnvelope),
    )
)]
async fn asset_usage(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(id): Path<String>,
) -> Result<Json<Data<Vec<MediaUsageResponse>>>, ApiError> {
    Authorizer::require(&tenant.scope, &Permission::from_static(crate::READ))?;
    let registry = state.registry().await?;
    let asset_id = uuid::Uuid::parse_str(&id)
        .map_err(|_| aphel_sdk::AppError::NotFound("asset not found".to_owned()))?;
    let context = RecordCtx::new(&tenant.scope, &state);
    let asset = registry
        .handler(crate::ASSET)?
        .load(&context, asset_id)
        .await?
        .ok_or_else(|| aphel_sdk::AppError::NotFound("asset not found".to_owned()))?;
    let Some(file) = asset.get("file").and_then(Value::as_str) else {
        return Ok(Json(Data::new(Vec::new())));
    };
    let mut used = Vec::new();
    for record in crate::usage(
        &aphel_sdk::platform::domain::access::AccessScope::system(),
        file,
    )
    .await?
    {
        let record_type = record.record_type.as_str().to_owned();
        let definition = registry.require(&record_type).ok();
        let type_label = definition
            .and_then(|definition| {
                definition
                    .custom
                    .as_ref()
                    .map(|labels| labels.plural.clone())
            })
            .unwrap_or_else(|| record_type.clone());
        let label = match (
            definition.and_then(|definition| definition.title_field.clone()),
            registry.handler(&record_type),
        ) {
            (Some(title), Ok(handler)) => handler
                .load(&context, record.record_id)
                .await
                .ok()
                .flatten()
                .and_then(|loaded| {
                    loaded
                        .get(&title)
                        .and_then(Value::as_str)
                        .map(str::to_owned)
                }),
            _ => None,
        };
        used.push(MediaUsageResponse {
            record_type,
            record_id: record.record_id.to_string(),
            type_label,
            label,
        });
    }
    Ok(Json(Data::new(used)))
}
