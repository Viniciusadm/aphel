use std::collections::HashMap;

use axum::Json;
use axum::extract::{Path, State};
use serde::Serialize;
use serde_json::Value;
use sqlx::MySqlPool;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::api::dto::common::{Data, ErrorEnvelope};
use crate::api::dto::field_permissions::{
    FieldPermissionMatrixResponse, FieldRedaction, SaveFieldPermissionsRequest,
};
use crate::api::error::ApiError;
use crate::api::extractors::{AuthContext, ClientInfo, IfMatch, JsonBody};
use crate::api::state::AppState;
use crate::application::fields::RelationPath;
use crate::application::fields::permissions::{
    FieldAccessPolicy, FieldAccessRule, FieldPermissionUseCases, is_unrestricted,
};
use crate::application::shared::RecordType;
use crate::domain::UserId;
use crate::domain::access::AccessScope;
use crate::errors::AppError;
use crate::infrastructure::fields::MySqlFieldPermissionPorts;
use crate::infrastructure::fields::permissions::{
    member_role_key, role_access_rules, strictest_access_rules,
};
use crate::records::serialize_records;

pub const TAG: &str = "field-permissions";

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new().routes(routes!(get_field_permissions, save_field_permissions))
}

pub async fn field_permission_use_cases(
    state: &AppState,
) -> Result<FieldPermissionUseCases<MySqlFieldPermissionPorts>, AppError> {
    Ok(FieldPermissionUseCases::new(
        MySqlFieldPermissionPorts::new(
            state.db()?,
            state.registry().await?,
            state.events.clone(),
            state.audit.clone(),
        ),
    ))
}

fn redaction_from(record_type: &RecordType, rules: Vec<FieldAccessRule>) -> FieldRedaction {
    let policy = FieldAccessPolicy::from_rules(record_type, rules);
    FieldRedaction::new(policy.hidden_system_keys(), policy.hidden_custom_keys())
}

pub async fn field_redaction(
    state: &AppState,
    scope: &AccessScope,
    record_type: &RecordType,
) -> Result<FieldRedaction, AppError> {
    if is_unrestricted(scope) {
        return Ok(FieldRedaction::default());
    }
    let rules = role_access_rules(&state.db()?, record_type, scope.role_key().as_str()).await?;
    Ok(redaction_from(record_type, rules))
}

pub async fn user_field_redaction(
    pool: &MySqlPool,
    user_id: Option<&UserId>,
    record_type: &RecordType,
) -> Result<FieldRedaction, AppError> {
    let role_key = match user_id {
        Some(user_id) => member_role_key(pool, user_id).await?,
        None => None,
    };
    let rules = match role_key.as_deref() {
        Some(crate::domain::access::RoleKey::ADMIN) => return Ok(FieldRedaction::default()),
        Some(role_key) => role_access_rules(pool, record_type, role_key).await?,
        None => strictest_access_rules(pool, record_type).await?,
    };
    Ok(redaction_from(record_type, rules))
}

pub async fn field_policies(
    state: &AppState,
    scope: &AccessScope,
    record_types: impl IntoIterator<Item = RecordType>,
) -> Result<HashMap<RecordType, FieldAccessPolicy>, AppError> {
    let mut policies = HashMap::new();
    if is_unrestricted(scope) {
        return Ok(policies);
    }
    let pool = state.db()?;
    let registry = state.registry().await?;
    for record_type in record_types {
        if policies.contains_key(&record_type) || registry.get(&record_type).is_none() {
            continue;
        }
        let rules = role_access_rules(&pool, &record_type, scope.role_key().as_str()).await?;
        let policy = FieldAccessPolicy::from_rules(&record_type, rules);
        policies.insert(record_type, policy);
    }
    Ok(policies)
}

pub struct RecordRedactor {
    record: FieldRedaction,
    nested: Vec<(String, FieldRedaction)>,
}

impl RecordRedactor {
    pub fn item<T: Serialize>(&self, item: T) -> Result<Value, AppError> {
        let mut value = self.record.item(item)?;
        for (key, redaction) in &self.nested {
            redaction.redact_nested(&mut value, key);
        }
        Ok(value)
    }

    pub fn items<T: Serialize>(&self, items: Vec<T>) -> Result<Vec<Value>, AppError> {
        items.into_iter().map(|item| self.item(item)).collect()
    }

    pub fn hides(&self, key: &str) -> bool {
        self.record.hides(key)
    }
}

pub async fn record_redactor(
    state: &AppState,
    scope: &AccessScope,
    record_type: &RecordType,
) -> Result<RecordRedactor, AppError> {
    let record = field_redaction(state, scope, record_type).await?;
    let mut nested = Vec::new();
    if is_unrestricted(scope) {
        return Ok(RecordRedactor { record, nested });
    }
    let registry = state.registry().await?;
    let relations = registry
        .get(record_type)
        .map(|definition| definition.relations.as_slice())
        .unwrap_or_default();
    for relation in relations {
        if matches!(relation.paths.as_slice(), [RelationPath::BelongsTo { .. }]) {
            let redaction = field_redaction(state, scope, &relation.target).await?;
            if !redaction.is_empty() {
                nested.push((relation.key.clone(), redaction));
            }
        }
    }
    Ok(RecordRedactor { record, nested })
}

pub fn nested_redactor(key: &'static str, redaction: FieldRedaction) -> RecordRedactor {
    RecordRedactor {
        record: FieldRedaction::default(),
        nested: vec![(key.to_owned(), redaction)],
    }
}

async fn serialized<T: Serialize>(
    state: &AppState,
    scope: &AccessScope,
    record_type: &RecordType,
    items: Vec<T>,
) -> Result<Vec<Value>, AppError> {
    let items = items
        .into_iter()
        .map(serde_json::to_value)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| AppError::Internal(format!("record could not be serialized: {error}")))?;
    match &state.db {
        Some(pool) => serialize_records(pool, scope, record_type, items).await,
        None => Ok(items),
    }
}

pub async fn redacted<T: Serialize>(
    state: &AppState,
    scope: &AccessScope,
    record_type: &RecordType,
    item: T,
) -> Result<Value, AppError> {
    let item = serialized(state, scope, record_type, vec![item])
        .await?
        .pop()
        .unwrap_or(Value::Null);
    record_redactor(state, scope, record_type).await?.item(item)
}

pub async fn redacted_items<T: Serialize>(
    state: &AppState,
    scope: &AccessScope,
    record_type: &RecordType,
    items: Vec<T>,
) -> Result<Vec<Value>, AppError> {
    let items = serialized(state, scope, record_type, items).await?;
    record_redactor(state, scope, record_type)
        .await?
        .items(items)
}

#[utoipa::path(
    get,
    path = "/field-permissions/{record_type}",
    tag = TAG,
    operation_id = "field_permissions_get",
    security(("session_cookie" = [])),
    description = "Returns the field access matrix of a record type (permission `settings.manage`): every field (system and custom, active or not) with its access per role (`hidden`, `read_only` or `editable`). A cell without a stored restriction is `editable`. The system role `admin` is always `editable` and is not configurable (`is_configurable: false`); fields with `is_locked: true` (required and structural fields, such as the name or title, the owner, the timestamps, the pipeline, stage and status of a deal, and required custom fields without a default value) are always `editable`. `updated_at` is the version to send in `If-Match` when saving (`null` before the first save).",
    params(("record_type" = String, Path, description = "Record type, such as `contact`")),
    responses(
        (status = 200, description = "Field access matrix", body = Data<FieldPermissionMatrixResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Unknown record type (`record_type_not_found`)", body = ErrorEnvelope),
    )
)]
async fn get_field_permissions(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(record_type): Path<String>,
) -> Result<Json<Data<FieldPermissionMatrixResponse>>, ApiError> {
    let matrix = field_permission_use_cases(&state)
        .await?
        .matrix(&tenant.scope, &record_type)
        .await?;
    Ok(Json(Data::new(matrix.into())))
}

#[utoipa::path(
    put,
    path = "/field-permissions/{record_type}",
    tag = TAG,
    operation_id = "field_permissions_save",
    security(("session_cookie" = [])),
    description = "Replaces the field access matrix of a record type (permission `settings.manage`) and publishes `field_permissions.updated`, with one audit change per cell (`<field key>.<role key>`). `entries` lists the restricted cells; every cell left out becomes `editable`, and an `editable` entry is the same as leaving it out. The role `admin` and locked fields only accept `editable`. The access applies on the server to every path of the role: hidden fields leave record responses (the key is removed, also in `custom_fields`, embedded objects and webhook bodies), cannot be filtered, sorted, searched, exported, imported or written (422 `field_not_accessible`) and show as `[hidden]` in timeline and audit diffs; read-only fields are returned but cannot be changed (422 `field_read_only`). Send `If-Match` with the `updated_at` you read to avoid overwriting someone else's change.",
    params(
        ("record_type" = String, Path, description = "Record type, such as `contact`"),
        ("If-Match" = Option<String>, Header, description = "`updated_at` of the matrix as last read; when it changed since, answers 409 `version_conflict`"),
    ),
    request_body = SaveFieldPermissionsRequest,
    responses(
        (status = 200, description = "Matrix saved", body = Data<FieldPermissionMatrixResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Unknown record type (`record_type_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "The matrix changed since `If-Match` (`version_conflict`)", body = ErrorEnvelope),
        (status = 422, description = "Unknown field, role or access, duplicated cell or a restriction on the admin role (`validation_error` with `details.fields`), or a restriction on a locked field (`field_permission_locked`)", body = ErrorEnvelope),
    )
)]
async fn save_field_permissions(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    IfMatch(expected): IfMatch,
    Path(record_type): Path<String>,
    JsonBody(request): JsonBody<SaveFieldPermissionsRequest>,
) -> Result<Json<Data<FieldPermissionMatrixResponse>>, ApiError> {
    let matrix = field_permission_use_cases(&state)
        .await?
        .save(
            &tenant.scope,
            &record_type,
            request.into_command(client.0.ip, expected),
        )
        .await?;
    Ok(Json(Data::new(matrix.into())))
}
