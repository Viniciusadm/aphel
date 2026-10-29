use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use serde_json::Value;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::api::dto::common::QuickSearchParams;
use crate::api::dto::common::{Data, ErrorEnvelope, Page};
use crate::api::dto::custom_objects::{
    CreateCustomObjectRequest, CreateRelationshipRequest, CustomObjectAdminResponse,
    CustomObjectResponse, CustomRecordRequest, CustomRecordResponse, LinkRecordRequest,
    LinkRecordResponse, ObjectRelationResponse, ObjectSchemaResponse, RelatedRecordResponse,
    RelationInfoResponse, RelationshipListParams, RelationshipResponse,
    ReorderCustomObjectsRequest, UpdateCustomObjectRequest, UpdateRelationshipRequest,
};
use crate::api::dto::query::{RecordListQuery, RecordQueryParams};
use crate::api::error::ApiError;
use crate::api::extractors::{
    AuthContext, ClientInfo, IfMatch, JsonBody, Pagination, PaginationQuery,
};
use crate::api::routes::v1::field_permissions::{redacted, redacted_items};
use crate::api::routes::v1::fields::field_use_cases;
use crate::api::routes::v1::members::window;
use crate::api::state::AppState;
use crate::application::custom_objects::{
    CustomObjectAdminUseCases, CustomRecordUseCases, RelationUseCases, resolve_relation,
};
use crate::application::shared::RecordType;
use crate::domain::access::AccessScope;
use crate::errors::AppError;
use crate::infrastructure::custom_objects::{
    MySqlCustomObjectAdminPorts, MySqlCustomRecordPorts, MySqlRelationPorts,
};

pub const ADMIN_TAG: &str = "custom-objects";
pub const RECORDS_TAG: &str = "objects";
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
        .routes(routes!(list_objects))
        .routes(routes!(object_schema))
        .routes(routes!(list_object_records, create_object_record))
        .routes(routes!(
            get_object_record,
            update_object_record,
            delete_object_record
        ))
        .routes(routes!(archive_object_record))
        .routes(routes!(unarchive_object_record))
        .routes(routes!(list_record_relations))
        .routes(routes!(list_related_records, link_record))
        .routes(routes!(unlink_record))
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

pub async fn custom_record_use_cases(
    state: &AppState,
) -> Result<CustomRecordUseCases<MySqlCustomRecordPorts>, AppError> {
    Ok(CustomRecordUseCases::new(MySqlCustomRecordPorts::new(
        state.db()?,
        state.registry().await?,
        state.events.clone(),
        state.audit.clone(),
    )))
}

pub async fn relation_use_cases(
    state: &AppState,
) -> Result<RelationUseCases<MySqlRelationPorts>, AppError> {
    Ok(RelationUseCases::new(MySqlRelationPorts::new(
        state.db()?,
        state.registry().await?,
        state.events.clone(),
        state.audit.clone(),
    )))
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
    description = "Creates a custom object (permission `settings.manage`) and publishes `custom_object.created`. The `key` (lowercase letters and underscores, immutable) becomes the record type `co_<key>`, the API path `/objects/<key>/records` and the permissions `co_<key>.read|create|update|delete|export|archive`; managers get them with team scope and members with own scope (no export), administrators get everything. The object starts with a required text field `name` used as the record title (`title_field_key`). At most 50 custom objects can exist.",
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
        .scope_for(&crate::domain::access::Permission::SETTINGS_MANAGE);
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
    description = "Creates a relationship between two record types (`company`, `contact`, `deal`, `product` or an active custom object `co_<key>`), permission `settings.manage`. `cardinality`: `one_to_one`, `one_to_many` (one `from` record, many `to` records) or `many_to_many`. `key` names the relation on the `from` type and `inverse_key` on the `to` type (both used in filters such as `company.industry` and in `related` filter nodes), `from_label` is shown on `from` records and `to_label` on `to` records. `required` makes every new `to` record of a custom object need a link. `on_delete` decides what happens when a `from` record is deleted: `restrict` (default) blocks the deletion while live linked records exist (409 `record_has_relations`), `unlink` lets it go and the links are removed when the record is purged. Publishes `relationship.created`.",
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

#[utoipa::path(
    get,
    path = "/objects",
    tag = RECORDS_TAG,
    operation_id = "objects_list",
    security(("session_cookie" = [])),
    description = "Active custom objects the caller can read, in navigation order.",
    responses(
        (status = 200, description = "Readable custom objects", body = Data<Vec<CustomObjectResponse>>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
    )
)]
async fn list_objects(
    State(state): State<AppState>,
    tenant: AuthContext,
) -> Result<Json<Data<Vec<CustomObjectResponse>>>, ApiError> {
    let objects = admin_use_cases(&state)
        .await?
        .visible(&tenant.scope)
        .await?;
    Ok(Json(Data::new(
        objects.iter().map(CustomObjectResponse::from).collect(),
    )))
}

#[utoipa::path(
    get,
    path = "/objects/{key}/schema",
    tag = RECORDS_TAG,
    operation_id = "objects_schema",
    security(("session_cookie" = [])),
    description = "Documents a custom object at runtime: its active fields as the caller sees them (types, options, filter operators, hidden fields left out), its relations (usable in `relation.field` filters, `related` filter nodes, aggregations and in `relations` when creating a record), the records path and the events it publishes. The static OpenAPI only documents the generic pattern of `/objects/{key}/records`.",
    params(("key" = String, Path, description = "Object key, such as `contract`")),
    responses(
        (status = 200, description = "Object schema", body = Data<ObjectSchemaResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `co_<key>.read` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Unknown or inactive object (`custom_object_not_found`)", body = ErrorEnvelope),
    )
)]
async fn object_schema(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(key): Path<String>,
) -> Result<Json<Data<ObjectSchemaResponse>>, ApiError> {
    let use_cases = custom_record_use_cases(&state).await?;
    let object = use_cases.object(&key)?.clone();
    let record_type = object.record_type();
    let fields = field_use_cases(&state)
        .await?
        .schema(&tenant.scope, &record_type)
        .await?;
    let registry = state.registry().await?;
    let schema = registry.custom_schema();
    let relations = registry
        .require(&record_type)?
        .relations
        .iter()
        .map(|relation| {
            let mut response = ObjectRelationResponse::from(relation);
            response.relationship_key = resolve_relation(&schema.relationships, relation)
                .map(|resolved| resolved.relationship.key);
            response
        })
        .collect();
    Ok(Json(Data::new(ObjectSchemaResponse {
        records_path: format!("/api/v1/objects/{}/records", object.key),
        events: ["created", "updated", "deleted", "archived", "unarchived"]
            .iter()
            .map(|action| format!("{record_type}.{action}"))
            .collect(),
        object: CustomObjectResponse::from(&object),
        fields: fields.into(),
        relations,
    })))
}

async fn redacted_record(
    state: &AppState,
    scope: &AccessScope,
    response: CustomRecordResponse,
) -> Result<Value, AppError> {
    let record_type = RecordType::parse(response.record_type.as_str())?;
    redacted(state, scope, &record_type, response).await
}

#[utoipa::path(
    get,
    path = "/objects/{key}/records",
    tag = RECORDS_TAG,
    operation_id = "objects_records_list",
    security(("session_cookie" = [])),
    description = "Lists records of a custom object (permission `co_<key>.read`, scope own/team/organization by `owner_id`). Same query language as every listing: the JSON AST in `filter` or `filter[field][operator]=value` over `title`, `owner_id`, `created_at`, `updated_at` and custom fields, conditions on related records (`company.industry`) and `related` nodes (`{\"related\": {\"relationship\": \"company\", \"filter\": {...}}}`), `sort=field,-other`, `search`/`q` over the title and searchable fields and `archived=exclude|include|only`. Field permissions apply.",
    params(
        ("key" = String, Path, description = "Object key, such as `contract`"),
        RecordQueryParams,
        QuickSearchParams,
        PaginationQuery
    ),
    responses(
        (status = 200, description = "Page of records with their custom fields", body = Page<CustomRecordResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `co_<key>.read` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Unknown or inactive object (`custom_object_not_found`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid filter, sort, search or pagination (`validation_error`, `field_not_accessible`)", body = ErrorEnvelope),
    )
)]
async fn list_object_records(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(key): Path<String>,
    RecordListQuery(mut query): RecordListQuery,
    Query(quick): Query<QuickSearchParams>,
    pagination: Pagination,
) -> Result<Json<Page<Value>>, ApiError> {
    quick.merge_into(&mut query.search);
    let use_cases = custom_record_use_cases(&state).await?;
    let record_type = RecordType::parse(use_cases.object(&key)?.record_type())?;
    let listing = use_cases.list(&tenant.scope, &key, query).await?;
    let items: Vec<CustomRecordResponse> = listing.items.into_iter().map(Into::into).collect();
    Ok(Json(Page::new(
        redacted_items(&state, &tenant.scope, &record_type, items).await?,
        listing.total,
        pagination,
    )))
}

#[utoipa::path(
    post,
    path = "/objects/{key}/records",
    tag = RECORDS_TAG,
    operation_id = "objects_records_create",
    security(("session_cookie" = [])),
    description = "Creates a record of a custom object (permission `co_<key>.create`) and publishes `co_<key>.created`. Values go in `custom_fields` (see `GET /objects/{key}/schema`); the `title` comes from the title field. `owner_id` defaults to the caller when the object has owners. `relations` links the new record in the same transaction: relation key → record id (or list of ids), and required relations must be present.",
    params(("key" = String, Path, description = "Object key")),
    request_body = CustomRecordRequest,
    responses(
        (status = 201, description = "Record created", body = Data<CustomRecordResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `co_<key>.create` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Unknown or inactive object (`custom_object_not_found`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid fields or relations (`validation_error` with `details.fields` such as `custom_fields.<key>` or `relations.<key>`), or fields the role cannot write (`field_not_accessible`, `field_read_only`)", body = ErrorEnvelope),
    )
)]
async fn create_object_record(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(key): Path<String>,
    JsonBody(request): JsonBody<CustomRecordRequest>,
) -> Result<(StatusCode, Json<Data<Value>>), ApiError> {
    let record = custom_record_use_cases(&state)
        .await?
        .create(&tenant.scope, &key, request.into_command(client.0.ip, None))
        .await?;
    Ok((
        StatusCode::CREATED,
        Json(Data::new(
            redacted_record(&state, &tenant.scope, record.into()).await?,
        )),
    ))
}

#[utoipa::path(
    get,
    path = "/objects/{key}/records/{record_id}",
    tag = RECORDS_TAG,
    operation_id = "objects_records_get",
    security(("session_cookie" = [])),
    params(
        ("key" = String, Path, description = "Object key"),
        ("record_id" = String, Path, description = "Record id"),
    ),
    responses(
        (status = 200, description = "The record (archived records included)", body = Data<CustomRecordResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `co_<key>.read` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Unknown object (`custom_object_not_found`) or record not found, deleted or outside the read scope (`custom_record_not_found`)", body = ErrorEnvelope),
    )
)]
async fn get_object_record(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path((key, record_id)): Path<(String, String)>,
) -> Result<Json<Data<Value>>, ApiError> {
    let record = custom_record_use_cases(&state)
        .await?
        .get(&tenant.scope, &key, &record_id)
        .await?;
    Ok(Json(Data::new(
        redacted_record(&state, &tenant.scope, record.into()).await?,
    )))
}

#[utoipa::path(
    patch,
    path = "/objects/{key}/records/{record_id}",
    tag = RECORDS_TAG,
    operation_id = "objects_records_update",
    security(("session_cookie" = [])),
    description = "Partially updates a record (permission `co_<key>.update`) and publishes `co_<key>.updated` with the changes. Omitted custom fields stay unchanged and `null` clears one. Links are changed through the relation endpoints. Send `If-Match` with the `updated_at` you read.",
    params(
        ("key" = String, Path, description = "Object key"),
        ("record_id" = String, Path, description = "Record id"),
        ("If-Match" = Option<String>, Header, description = "`updated_at` of the record as read; 409 when it changed since"),
    ),
    request_body = CustomRecordRequest,
    responses(
        (status = 200, description = "Record updated", body = Data<CustomRecordResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `co_<key>.update` or outside its scope (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Record not found (`custom_record_not_found`, `custom_object_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "Changed since the `If-Match` version (`version_conflict`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid fields (`validation_error`) or fields the role cannot write (`field_not_accessible`, `field_read_only`)", body = ErrorEnvelope),
    )
)]
async fn update_object_record(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    IfMatch(expected): IfMatch,
    Path((key, record_id)): Path<(String, String)>,
    JsonBody(request): JsonBody<CustomRecordRequest>,
) -> Result<Json<Data<Value>>, ApiError> {
    let record = custom_record_use_cases(&state)
        .await?
        .update(
            &tenant.scope,
            &key,
            &record_id,
            request.into_command(client.0.ip, expected),
        )
        .await?;
    Ok(Json(Data::new(
        redacted_record(&state, &tenant.scope, record.into()).await?,
    )))
}

#[utoipa::path(
    delete,
    path = "/objects/{key}/records/{record_id}",
    tag = RECORDS_TAG,
    operation_id = "objects_records_delete",
    security(("session_cookie" = [])),
    description = "Deletes a record (soft delete, permission `co_<key>.delete`) and publishes `co_<key>.deleted`. Relationships with `on_delete: restrict` where the record is on the `from` side block the deletion while live linked records exist.",
    params(
        ("key" = String, Path, description = "Object key"),
        ("record_id" = String, Path, description = "Record id"),
    ),
    responses(
        (status = 204, description = "Record deleted"),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `co_<key>.delete` or outside its scope (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Record not found (`custom_record_not_found`, `custom_object_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "Linked records block the deletion (`record_has_relations`)", body = ErrorEnvelope),
    )
)]
async fn delete_object_record(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path((key, record_id)): Path<(String, String)>,
) -> Result<StatusCode, ApiError> {
    custom_record_use_cases(&state)
        .await?
        .delete(&tenant.scope, &key, &record_id, client.0.ip)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    post,
    path = "/objects/{key}/records/{record_id}/archive",
    tag = RECORDS_TAG,
    operation_id = "objects_records_archive",
    security(("session_cookie" = [])),
    description = "Archives a record (permission `co_<key>.archive`) and publishes `co_<key>.archived`. Archived records leave the default listings (`archived=include|only` brings them back).",
    params(
        ("key" = String, Path, description = "Object key"),
        ("record_id" = String, Path, description = "Record id"),
    ),
    responses(
        (status = 200, description = "Record archived", body = Data<CustomRecordResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `co_<key>.archive` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Record not found (`custom_record_not_found`, `custom_object_not_found`)", body = ErrorEnvelope),
    )
)]
async fn archive_object_record(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path((key, record_id)): Path<(String, String)>,
) -> Result<Json<Data<Value>>, ApiError> {
    let record = custom_record_use_cases(&state)
        .await?
        .set_archived(&tenant.scope, &key, &record_id, true, client.0.ip)
        .await?;
    Ok(Json(Data::new(
        redacted_record(&state, &tenant.scope, record.into()).await?,
    )))
}

#[utoipa::path(
    post,
    path = "/objects/{key}/records/{record_id}/unarchive",
    tag = RECORDS_TAG,
    operation_id = "objects_records_unarchive",
    security(("session_cookie" = [])),
    description = "Restores an archived record (permission `co_<key>.archive`) and publishes `co_<key>.unarchived`.",
    params(
        ("key" = String, Path, description = "Object key"),
        ("record_id" = String, Path, description = "Record id"),
    ),
    responses(
        (status = 200, description = "Record restored", body = Data<CustomRecordResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `co_<key>.archive` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Record not found (`custom_record_not_found`, `custom_object_not_found`)", body = ErrorEnvelope),
    )
)]
async fn unarchive_object_record(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path((key, record_id)): Path<(String, String)>,
) -> Result<Json<Data<Value>>, ApiError> {
    let record = custom_record_use_cases(&state)
        .await?
        .set_archived(&tenant.scope, &key, &record_id, false, client.0.ip)
        .await?;
    Ok(Json(Data::new(
        redacted_record(&state, &tenant.scope, record.into()).await?,
    )))
}

#[utoipa::path(
    get,
    path = "/records/{record_type}/{record_id}/relations",
    tag = RELATIONSHIPS_TAG,
    operation_id = "record_relations_list",
    security(("session_cookie" = [])),
    description = "Relations of any record (core or custom) defined by relationships, with the number of linked records the caller can see (`total` is null and `readable` false when the caller cannot read the other type).",
    params(
        ("record_type" = String, Path, description = "Record type, such as `company` or `co_contract`"),
        ("record_id" = String, Path, description = "Record id"),
    ),
    responses(
        (status = 200, description = "Relations of the record", body = Data<Vec<RelationInfoResponse>>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "The record does not exist or cannot be read (`record_not_accessible`, `permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Unknown record type (`record_type_not_found`)", body = ErrorEnvelope),
    )
)]
async fn list_record_relations(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path((record_type, record_id)): Path<(String, String)>,
) -> Result<Json<Data<Vec<RelationInfoResponse>>>, ApiError> {
    let relations = relation_use_cases(&state)
        .await?
        .relations(&tenant.scope, &record_type, &record_id)
        .await?;
    Ok(Json(Data::new(
        relations.into_iter().map(Into::into).collect(),
    )))
}

#[utoipa::path(
    get,
    path = "/records/{record_type}/{record_id}/relations/{relation}",
    tag = RELATIONSHIPS_TAG,
    operation_id = "record_relations_records",
    security(("session_cookie" = [])),
    description = "Records linked to a record through a relation, newest link first, limited to those the caller can read.",
    params(
        ("record_type" = String, Path, description = "Record type"),
        ("record_id" = String, Path, description = "Record id"),
        ("relation" = String, Path, description = "Relation key on the record type, such as `contracts`"),
        PaginationQuery,
    ),
    responses(
        (status = 200, description = "Linked records", body = Page<RelatedRecordResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "The record or the related type cannot be read (`record_not_accessible`, `permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Unknown record type or relation (`record_type_not_found`, `relation_not_found`)", body = ErrorEnvelope),
    )
)]
async fn list_related_records(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path((record_type, record_id, relation)): Path<(String, String, String)>,
    pagination: Pagination,
) -> Result<Json<Page<RelatedRecordResponse>>, ApiError> {
    let (_, listing) = relation_use_cases(&state)
        .await?
        .related(
            &tenant.scope,
            &record_type,
            &record_id,
            &relation,
            window(pagination),
        )
        .await?;
    Ok(Json(Page::new(
        listing
            .items
            .into_iter()
            .map(|summary| RelatedRecordResponse {
                id: summary.record.record_id.to_string(),
                record_type: summary.record.record_type.as_str().to_owned(),
                title: summary.title,
            })
            .collect(),
        listing.total,
        pagination,
    )))
}

#[utoipa::path(
    post,
    path = "/records/{record_type}/{record_id}/relations/{relation}",
    tag = RELATIONSHIPS_TAG,
    operation_id = "record_relations_link",
    security(("session_cookie" = [])),
    description = "Links a record to another through a relation. Needs the update permission on the record and read access to the other one. Cardinality is enforced (`one_to_one`: one link on each side; `one_to_many`: a `to` record has a single `from`). Publishes `record.linked` on the record. Linking twice is a no-op (`linked: false`).",
    params(
        ("record_type" = String, Path, description = "Record type"),
        ("record_id" = String, Path, description = "Record id"),
        ("relation" = String, Path, description = "Relation key on the record type"),
    ),
    request_body = LinkRecordRequest,
    responses(
        (status = 200, description = "Link result", body = Data<LinkRecordResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Cannot change the record (`permission_denied`, `record_not_accessible`)", body = ErrorEnvelope),
        (status = 404, description = "Unknown relation or linked record (`relation_not_found`, `linked_record_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "Cardinality reached (`relation_limit_reached`)", body = ErrorEnvelope),
    )
)]
async fn link_record(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path((record_type, record_id, relation)): Path<(String, String, String)>,
    JsonBody(request): JsonBody<LinkRecordRequest>,
) -> Result<Json<Data<LinkRecordResponse>>, ApiError> {
    let linked = relation_use_cases(&state)
        .await?
        .link(
            &tenant.scope,
            &record_type,
            &record_id,
            &relation,
            &request.record_id,
            client.0.ip,
        )
        .await?;
    Ok(Json(Data::new(LinkRecordResponse { linked })))
}

#[utoipa::path(
    delete,
    path = "/records/{record_type}/{record_id}/relations/{relation}/{target_id}",
    tag = RELATIONSHIPS_TAG,
    operation_id = "record_relations_unlink",
    security(("session_cookie" = [])),
    description = "Removes a link. Publishes `record.unlinked`. The last link of a required relationship of a custom object cannot be removed.",
    params(
        ("record_type" = String, Path, description = "Record type"),
        ("record_id" = String, Path, description = "Record id"),
        ("relation" = String, Path, description = "Relation key on the record type"),
        ("target_id" = String, Path, description = "Id of the linked record"),
    ),
    responses(
        (status = 204, description = "Link removed (or it did not exist)"),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Cannot change the record (`permission_denied`, `record_not_accessible`)", body = ErrorEnvelope),
        (status = 404, description = "Unknown relation or linked record (`relation_not_found`, `linked_record_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "Last link of a required relationship (`relation_required`)", body = ErrorEnvelope),
    )
)]
async fn unlink_record(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path((record_type, record_id, relation, target_id)): Path<(String, String, String, String)>,
) -> Result<StatusCode, ApiError> {
    relation_use_cases(&state)
        .await?
        .unlink(
            &tenant.scope,
            &record_type,
            &record_id,
            &relation,
            &target_id,
            client.0.ip,
        )
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
