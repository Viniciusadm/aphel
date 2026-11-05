use crate::application::CustomObjectAdminUseCases;
use crate::dto::{
    CreateCustomObjectRequest, CreateRelationshipRequest, CustomObjectAdminResponse,
    RelationshipListParams, ReorderCustomObjectsRequest, UpdateCustomObjectRequest,
    UpdateRelationshipRequest,
};
use crate::infrastructure::MySqlCustomObjectAdminPorts;
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::api::dto::common::{Data, ErrorEnvelope};
use aphel_sdk::platform::api::dto::custom_objects::{CustomObjectResponse, RelationshipResponse};
use aphel_sdk::platform::api::error::ApiError;
use aphel_sdk::platform::api::extractors::{AuthContext, ClientInfo, IfMatch, JsonBody};
use aphel_sdk::platform::api::state::AppState;
use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

pub const ADMIN_TAG: &str = "custom-objects";
pub const RELATIONSHIPS_TAG: &str = "relationships";

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list_custom_objects, create_custom_object))
        .routes(routes!(reorder_custom_objects))
        .routes(routes!(
            get_custom_object,
            update_custom_object,
            delete_custom_object
        ))
        .routes(routes!(list_relationships, create_relationship))
        .routes(routes!(update_relationship, delete_relationship))
}

async fn admin_use_cases(
    state: &AppState,
) -> Result<CustomObjectAdminUseCases<MySqlCustomObjectAdminPorts>, AppError> {
    Ok(CustomObjectAdminUseCases::new(
        MySqlCustomObjectAdminPorts::new(
            state.db()?,
            state.registry().await?,
            state.events.clone(),
            state.audit.clone(),
        ),
    ))
}

#[utoipa::path(
    get,
    path = "/custom-objects",
    tag = ADMIN_TAG,
    operation_id = "custom_objects_list",
    security(("session_cookie" = [])),
    description = "Lists the custom objects of the organization, active or not, ordered by position, with the number of live records (permission `settings.manage`).",
    responses(
        (status = 200, description = "Custom objects", body = Data<Vec<CustomObjectAdminResponse>>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
    )
)]
async fn list_custom_objects(
    State(state): State<AppState>,
    tenant: AuthContext,
) -> Result<Json<Data<Vec<CustomObjectAdminResponse>>>, ApiError> {
    let objects = admin_use_cases(&state).await?.list(&tenant.scope).await?;
    Ok(Json(Data::new(
        objects.into_iter().map(Into::into).collect(),
    )))
}

#[utoipa::path(
    post,
    path = "/custom-objects",
    tag = ADMIN_TAG,
    operation_id = "custom_objects_create",
    security(("session_cookie" = [])),
    description = "Creates a custom object (permission `settings.manage`) and publishes `custom_object.created`. The `key` (lowercase letters and underscores, immutable) becomes the record type `custom.<key>`, the API path `/objects/<key>/records` and the permissions `custom.<key>.read|create|update|delete|export|archive`; managers get them with team scope and members with own scope (no export), administrators get everything. The object starts with a required text field `name` used as the record title (`title_field_key`). At most 50 custom objects can exist.",
    request_body = CreateCustomObjectRequest,
    responses(
        (status = 201, description = "Custom object created", body = Data<CustomObjectAdminResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 409, description = "Key already used (`custom_object_key_taken`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid key, labels, icon or color (`validation_error`) or limit reached (`too_many_custom_objects`)", body = ErrorEnvelope),
    )
)]
async fn create_custom_object(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    JsonBody(request): JsonBody<CreateCustomObjectRequest>,
) -> Result<(StatusCode, Json<Data<CustomObjectAdminResponse>>), ApiError> {
    let view = admin_use_cases(&state)
        .await?
        .create(&tenant.scope, request.into_command(client.0.ip))
        .await?;
    state.record_types.invalidate();
    Ok((StatusCode::CREATED, Json(Data::new(view.into()))))
}

#[utoipa::path(
    put,
    path = "/custom-objects/order",
    tag = ADMIN_TAG,
    operation_id = "custom_objects_reorder",
    security(("session_cookie" = [])),
    description = "Sets the order of the custom objects (navigation order). `keys` must list every object exactly once. Publishes `custom_object.reordered`.",
    request_body = ReorderCustomObjectsRequest,
    responses(
        (status = 200, description = "Objects in the new order", body = Data<Vec<CustomObjectResponse>>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 422, description = "Incomplete or repeated keys (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn reorder_custom_objects(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    JsonBody(request): JsonBody<ReorderCustomObjectsRequest>,
) -> Result<Json<Data<Vec<CustomObjectResponse>>>, ApiError> {
    let objects = admin_use_cases(&state)
        .await?
        .reorder(&tenant.scope, &request.keys, client.0.ip)
        .await?;
    state.record_types.invalidate();
    Ok(Json(Data::new(
        objects.iter().map(CustomObjectResponse::from).collect(),
    )))
}

#[utoipa::path(
    get,
    path = "/custom-objects/{key}",
    tag = ADMIN_TAG,
    operation_id = "custom_objects_get",
    security(("session_cookie" = [])),
    params(("key" = String, Path, description = "Object key, such as `contract`")),
    responses(
        (status = 200, description = "The custom object", body = Data<CustomObjectAdminResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Unknown object (`custom_object_not_found`)", body = ErrorEnvelope),
    )
)]
async fn get_custom_object(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(key): Path<String>,
) -> Result<Json<Data<CustomObjectAdminResponse>>, ApiError> {
    let view = admin_use_cases(&state)
        .await?
        .get(&tenant.scope, &key)
        .await?;
    Ok(Json(Data::new(view.into())))
}

#[utoipa::path(
    patch,
    path = "/custom-objects/{key}",
    tag = ADMIN_TAG,
    operation_id = "custom_objects_update",
    security(("session_cookie" = [])),
    description = "Updates labels, icon, color, `owner_enabled`, `is_active` (inactive objects disappear from the navigation, the API and every generic feature, keeping their data) or `title_field_key` (an active custom text, e-mail, phone or URL field; existing titles are recomputed). The key never changes. Publishes `custom_object.updated`.",
    params(
        ("key" = String, Path, description = "Object key"),
        ("If-Match" = Option<String>, Header, description = "`updated_at` of the object as read; 409 when it changed since"),
    ),
    request_body = UpdateCustomObjectRequest,
    responses(
        (status = 200, description = "Custom object updated", body = Data<CustomObjectAdminResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Unknown object (`custom_object_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "Changed since the `If-Match` version (`version_conflict`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid properties (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn update_custom_object(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    IfMatch(expected): IfMatch,
    Path(key): Path<String>,
    JsonBody(request): JsonBody<UpdateCustomObjectRequest>,
) -> Result<Json<Data<CustomObjectAdminResponse>>, ApiError> {
    let view = admin_use_cases(&state)
        .await?
        .update(
            &tenant.scope,
            &key,
            request.into_command(client.0.ip, expected),
        )
        .await?;
    state.record_types.invalidate();
    Ok(Json(Data::new(view.into())))
}

#[utoipa::path(
    delete,
    path = "/custom-objects/{key}",
    tag = ADMIN_TAG,
    operation_id = "custom_objects_delete",
    security(("session_cookie" = [])),
    description = "Deletes a custom object without live records together with its fields, views, layouts, segments, reports, field permissions, role permissions and relationships. Objects with records must be deactivated instead. Publishes `custom_object.deleted`.",
    params(("key" = String, Path, description = "Object key")),
    responses(
        (status = 204, description = "Custom object deleted"),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Unknown object (`custom_object_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "The object still has records (`custom_object_has_records`)", body = ErrorEnvelope),
    )
)]
async fn delete_custom_object(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(key): Path<String>,
) -> Result<StatusCode, ApiError> {
    admin_use_cases(&state)
        .await?
        .delete(&tenant.scope, &key, client.0.ip)
        .await?;
    state.record_types.invalidate();
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    get,
    path = "/relationships",
    tag = RELATIONSHIPS_TAG,
    operation_id = "relationships_list",
    security(("session_cookie" = [])),
    description = "Lists the configurable relationships between record types (core or custom), optionally only those touching `record_type`. Members without `settings.manage` only see relationships between types they can read and get `link_count: null`.",
    params(RelationshipListParams),
    responses(
        (status = 200, description = "Relationships", body = Data<Vec<RelationshipResponse>>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
    )
)]
async fn list_relationships(
    State(state): State<AppState>,
    tenant: AuthContext,
    Query(params): Query<RelationshipListParams>,
) -> Result<Json<Data<Vec<RelationshipResponse>>>, ApiError> {
    let manager = tenant
        .scope
        .permissions()
        .scope_for(&aphel_sdk::platform::domain::access::Permission::SETTINGS_MANAGE);
    let relationships = admin_use_cases(&state)
        .await?
        .list_relationships(&tenant.scope, params.record_type.as_deref())
        .await?;
    Ok(Json(Data::new(
        relationships
            .into_iter()
            .map(|view| {
                let mut response = RelationshipResponse::from(view);
                if manager.is_none() {
                    response.link_count = None;
                }
                response
            })
            .collect(),
    )))
}

#[utoipa::path(
    post,
    path = "/relationships",
    tag = RELATIONSHIPS_TAG,
    operation_id = "relationships_create",
    security(("session_cookie" = [])),
    description = "Creates a relationship between two record types (`company`, `contact`, `deal`, `product` or an active custom object `custom.<key>`), permission `settings.manage`. `cardinality`: `one_to_one`, `one_to_many` (one `from` record, many `to` records) or `many_to_many`. `key` names the relation on the `from` type and `inverse_key` on the `to` type (both used in filters such as `company.industry` and in `related` filter nodes), `from_label` is shown on `from` records and `to_label` on `to` records. `required` makes every new `to` record of a custom object need a link. `on_delete` decides what happens when a `from` record is deleted: `restrict` (default) blocks the deletion while live linked records exist (409 `record_has_relations`), `unlink` lets it go and the links are removed when the record is purged. Publishes `relationship.created`.",
    request_body = CreateRelationshipRequest,
    responses(
        (status = 201, description = "Relationship created", body = Data<RelationshipResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 409, description = "Key already used (`relationship_key_taken`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid properties or a key already used by a field or relation of the type (`validation_error`), or limit reached (`too_many_relationships`)", body = ErrorEnvelope),
    )
)]
async fn create_relationship(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    JsonBody(request): JsonBody<CreateRelationshipRequest>,
) -> Result<(StatusCode, Json<Data<RelationshipResponse>>), ApiError> {
    let view = admin_use_cases(&state)
        .await?
        .create_relationship(&tenant.scope, request.into_command(client.0.ip))
        .await?;
    state.record_types.invalidate();
    Ok((StatusCode::CREATED, Json(Data::new(view.into()))))
}

#[utoipa::path(
    patch,
    path = "/relationships/{key}",
    tag = RELATIONSHIPS_TAG,
    operation_id = "relationships_update",
    security(("session_cookie" = [])),
    description = "Updates labels, `required` and `on_delete` of a relationship. Types, keys and cardinality never change. Publishes `relationship.updated`.",
    params(
        ("key" = String, Path, description = "Relationship key"),
        ("If-Match" = Option<String>, Header, description = "`updated_at` of the relationship as read; 409 when it changed since"),
    ),
    request_body = UpdateRelationshipRequest,
    responses(
        (status = 200, description = "Relationship updated", body = Data<RelationshipResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Unknown relationship (`relationship_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "Changed since the `If-Match` version (`version_conflict`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid properties (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn update_relationship(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    IfMatch(expected): IfMatch,
    Path(key): Path<String>,
    JsonBody(request): JsonBody<UpdateRelationshipRequest>,
) -> Result<Json<Data<RelationshipResponse>>, ApiError> {
    let view = admin_use_cases(&state)
        .await?
        .update_relationship(
            &tenant.scope,
            &key,
            request.into_command(client.0.ip, expected),
        )
        .await?;
    state.record_types.invalidate();
    Ok(Json(Data::new(view.into())))
}

#[utoipa::path(
    delete,
    path = "/relationships/{key}",
    tag = RELATIONSHIPS_TAG,
    operation_id = "relationships_delete",
    security(("session_cookie" = [])),
    description = "Deletes a relationship and all its links. Publishes `relationship.deleted` with the number of removed links.",
    params(("key" = String, Path, description = "Relationship key")),
    responses(
        (status = 204, description = "Relationship deleted"),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Unknown relationship (`relationship_not_found`)", body = ErrorEnvelope),
    )
)]
async fn delete_relationship(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(key): Path<String>,
) -> Result<StatusCode, ApiError> {
    admin_use_cases(&state)
        .await?
        .delete_relationship(&tenant.scope, &key, client.0.ip)
        .await?;
    state.record_types.invalidate();
    Ok(StatusCode::NO_CONTENT)
}
