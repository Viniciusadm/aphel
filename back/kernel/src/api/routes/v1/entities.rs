use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use serde_json::{Value, json};
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;

use crate::api::dto::common::{Data, ErrorEnvelope, Page};
use crate::api::dto::query::RecordListQuery;
use crate::api::error::ApiError;
use crate::api::extractors::{AuthContext, IfMatch, JsonBody, Pagination};
use crate::application::access::Authorizer;
use crate::application::query::RecordQueryEngine;
use crate::records::RecordCtx;
use crate::state::AppState;

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(entities))
        .routes(routes!(list, create))
        .routes(routes!(get, update, delete))
}

#[utoipa::path(get, path = "/entities", tag = "fields", operation_id = "entities_list", security(("session_cookie" = [])), responses((status = 200, body = Object), (status = 403, body = ErrorEnvelope)))]
async fn entities(
    State(state): State<AppState>,
    tenant: AuthContext,
) -> Result<Json<Data<Vec<Value>>>, ApiError> {
    let registry = state.registry().await?;
    let app = crate::modules::app();
    let items = registry.definitions().filter(|entity| {
        let enabled = app.record_contributions().iter().find(|entry|entry.registers.as_ref() == Some(&entity.record_type)).is_none_or(|entry|app.is_enabled(entry.module));
        enabled && (tenant.scope.require(&entity.read_permission).is_ok() || entity.action_permission("create").is_some_and(|permission|tenant.scope.require(&permission).is_ok()))
    }).map(|entity| {
        let create = entity.action_permission("create");
        let update = entity.action_permission("update");
        json!({
            "key":entity.key,"record_type":entity.record_type.as_str(),"storage":entity.storage,"title_field":entity.title_field,
            "labels":entity.capabilities.plural_labels_value().or_else(||entity.custom.as_ref().map(|labels|labels.plural.clone().into())).unwrap_or_else(||entity.record_type.as_str().to_owned().into()),
            "traits":entity.capabilities.traits,"indexes":entity.indexes,"references":entity.references,
            "permissions":{"read":entity.read_permission.as_str(),"create":create.as_ref().map(|permission|permission.as_str()),"update":update.as_ref().map(|permission|permission.as_str())},
            "access":{"read":tenant.scope.require(&entity.read_permission).is_ok(),"create":create.is_some_and(|permission|tenant.scope.require(&permission).is_ok()),"update":update.is_some_and(|permission|tenant.scope.require(&permission).is_ok())},
        })
    }).collect();
    Ok(Json(Data::new(items)))
}

#[utoipa::path(get, path = "/entities/{entity}/records", tag = "fields", operation_id = "entity_records_list", security(("session_cookie" = [])), params(("entity" = String, Path)), responses((status = 200, body = Page<Value>), (status = 403, body = ErrorEnvelope)))]
async fn list(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(entity): Path<String>,
    RecordListQuery(query): RecordListQuery,
    pagination: Pagination,
) -> Result<Json<Page<Value>>, ApiError> {
    let registry = state.registry().await?;
    let definition = registry.require(&entity)?;
    let ports = crate::records::query_ports(&state).await?;
    let listing = RecordQueryEngine::new(&ports)
        .query(&tenant.scope, definition.record_type.as_str(), query)
        .await?;
    let items = registry
        .handler(&entity)?
        .load_listed(&RecordCtx::new(&tenant.scope, &state), &listing.items)
        .await?;
    Ok(Json(Page::new(items, listing.total, pagination)))
}

#[utoipa::path(post, path = "/entities/{entity}/records", tag = "fields", operation_id = "entity_records_create", security(("session_cookie" = [])), params(("entity" = String, Path)), request_body = Value, responses((status = 201, body = Data<Value>), (status = 422, body = ErrorEnvelope)))]
async fn create(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(entity): Path<String>,
    JsonBody(body): JsonBody<Value>,
) -> Result<(StatusCode, Json<Data<Value>>), ApiError> {
    let value = state
        .registry()
        .await?
        .handler(&entity)?
        .create(&RecordCtx::new(&tenant.scope, &state), body)
        .await?;
    Ok((StatusCode::CREATED, Json(Data::new(value))))
}

#[utoipa::path(get, path = "/entities/{entity}/records/{id}", tag = "fields", operation_id = "entity_records_get", security(("session_cookie" = [])), params(("entity" = String, Path), ("id" = String, Path)), responses((status = 200, body = Data<Value>), (status = 404, body = ErrorEnvelope)))]
async fn get(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path((entity, id)): Path<(String, Uuid)>,
) -> Result<Json<Data<Value>>, ApiError> {
    let value = state
        .registry()
        .await?
        .handler(&entity)?
        .load(&RecordCtx::new(&tenant.scope, &state), id)
        .await?
        .ok_or_else(|| {
            crate::errors::AppError::not_found_code("record_not_found", "record was not found")
        })?;
    Ok(Json(Data::new(value)))
}

#[utoipa::path(patch, path = "/entities/{entity}/records/{id}", tag = "fields", operation_id = "entity_records_update", security(("session_cookie" = [])), params(("entity" = String, Path), ("id" = String, Path)), request_body = Value, responses((status = 200, body = Data<Value>), (status = 409, body = ErrorEnvelope)))]
async fn update(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path((entity, id)): Path<(String, Uuid)>,
    IfMatch(expected): IfMatch,
    JsonBody(body): JsonBody<Value>,
) -> Result<Json<Data<Value>>, ApiError> {
    let registry = state.registry().await?;
    let definition = registry.require(&entity)?;
    let handler = registry.handler(&entity)?;
    let record_ctx = RecordCtx::new(&tenant.scope, &state);
    let value = if expected.is_some() {
        if !handler.durable_mutations() {
            return Err(
                crate::records::operation_not_supported(&entity, "versioned updates").into(),
            );
        }
        tenant.scope.require(
            definition
                .update_permission
                .as_ref()
                .ok_or_else(|| crate::records::operation_not_supported(&entity, "updates"))?,
        )?;
        if handler.load(&record_ctx, id).await?.is_none() {
            return Err(crate::errors::AppError::not_found_code(
                "record_not_found",
                "record was not found",
            )
            .into());
        }
        let mut tx = state
            .db()?
            .begin()
            .await
            .map_err(crate::errors::AppError::from)?
            .into();
        crate::records::lock_version(&mut tx, &definition.table, id, expected).await?;
        let pending = handler.update_in_tx(&record_ctx, &mut tx, id, body).await?;
        tx.commit().await.map_err(crate::errors::AppError::from)?;
        if let Some(pending) = pending {
            pending
                .committed(&crate::Ctx::from_state(&state, &tenant.scope).await?)
                .await;
        }
        handler.load(&record_ctx, id).await?.unwrap_or_else(|| json!({"id": id, "record_type": definition.record_type.as_str(), "entity": definition.key}))
    } else {
        handler.update(&record_ctx, id, body).await?
    };
    Ok(Json(Data::new(value)))
}

#[utoipa::path(delete, path = "/entities/{entity}/records/{id}", tag = "fields", operation_id = "entity_records_delete", security(("session_cookie" = [])), params(("entity" = String, Path), ("id" = String, Path)), responses((status = 204), (status = 409, body = ErrorEnvelope)))]
async fn delete(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path((entity, id)): Path<(String, Uuid)>,
) -> Result<StatusCode, ApiError> {
    state
        .registry()
        .await?
        .handler(&entity)?
        .delete(&RecordCtx::new(&tenant.scope, &state), id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
