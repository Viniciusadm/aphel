use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use chrono::SecondsFormat;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;
use uuid::Uuid;

use crate::domain::{
    GENERAL_VOCABULARY, Vocabulary, normalize_vocabulary_key, normalize_vocabulary_name,
};
use crate::http::TAG;
use crate::service::vocabulary;
use crate::store::TagStore;
use aphel_kernel::Ctx;
use aphel_kernel::errors::{AppError, FieldViolation};
use aphel_kernel::records::{RecordMutation, publish_mutation};
use aphel_sdk::platform::api::dto::common::{Data, ErrorEnvelope};
use aphel_sdk::platform::api::error::ApiError;
use aphel_sdk::platform::api::extractors::{AuthContext, ClientInfo, JsonBody};
use aphel_sdk::platform::api::state::AppState;
use aphel_sdk::platform::application::access::Authorizer;
use aphel_sdk::platform::application::shared::{FieldChange, RecordRef, RecordType};
use aphel_sdk::platform::database::transaction::Tx;
use aphel_sdk::platform::domain::access::Permission;

pub(crate) const VOCABULARY_KEY_TAKEN: &str = "tag_vocabulary_key_taken";
pub(crate) const VOCABULARY_NOT_EMPTY: &str = "tag_vocabulary_not_empty";
pub(crate) const VOCABULARY_HAS_HIERARCHY: &str = "tag_vocabulary_has_hierarchy";
pub(crate) const VOCABULARY_PROTECTED: &str = "tag_vocabulary_protected";
const VOCABULARY_CREATED: &str = "tag_vocabulary.created";
const VOCABULARY_UPDATED: &str = "tag_vocabulary.updated";
const VOCABULARY_DELETED: &str = "tag_vocabulary.deleted";
const VOCABULARY_RECORD_TYPE: RecordType = RecordType::from_static("tagging.vocabulary");
const MAX_VOCABULARIES: usize = 50;

#[derive(Debug, Clone, Default)]
pub(crate) struct VocabularyInput {
    pub(crate) key: String,
    pub(crate) name: String,
    pub(crate) hierarchical: bool,
    pub(crate) public: bool,
}

#[derive(Debug, Clone, Default, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct CreateVocabularyRequest {
    #[schema(example = "categories")]
    pub(crate) key: String,
    #[schema(example = "Categorias")]
    pub(crate) name: String,
    #[serde(default)]
    pub(crate) hierarchical: bool,
    #[serde(default)]
    pub(crate) public: bool,
}

#[derive(Debug, Clone, Default, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct UpdateVocabularyRequest {
    pub(crate) name: Option<String>,
    pub(crate) hierarchical: Option<bool>,
    pub(crate) public: Option<bool>,
}

#[derive(Debug, Serialize, ToSchema)]
#[schema(as = TagVocabulary)]
pub(crate) struct VocabularyResponse {
    pub(crate) id: String,
    #[schema(example = "categories")]
    pub(crate) key: String,
    pub(crate) name: String,
    pub(crate) hierarchical: bool,
    pub(crate) public: bool,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

impl From<Vocabulary> for VocabularyResponse {
    fn from(vocabulary: Vocabulary) -> Self {
        Self {
            id: vocabulary.id.to_string(),
            key: vocabulary.key,
            name: vocabulary.name,
            hierarchical: vocabulary.hierarchical,
            public: vocabulary.public,
            created_at: vocabulary
                .created_at
                .to_rfc3339_opts(SecondsFormat::Secs, true),
            updated_at: vocabulary
                .updated_at
                .to_rfc3339_opts(SecondsFormat::Secs, true),
        }
    }
}

fn snapshot(vocabulary: &Vocabulary) -> Value {
    json!({
        "key": vocabulary.key,
        "name": vocabulary.name,
        "hierarchical": vocabulary.hierarchical,
        "public": vocabulary.public,
    })
}

fn diff(old: &Value, new: &Value) -> Vec<FieldChange> {
    ["key", "name", "hierarchical", "public"]
        .into_iter()
        .filter_map(|key| {
            let before = old.get(key).cloned().unwrap_or(Value::Null);
            let after = new.get(key).cloned().unwrap_or(Value::Null);
            (before != after).then(|| FieldChange::new(key, before, after))
        })
        .collect()
}

async fn publish(
    ctx: &Ctx<'_>,
    tx: &mut Tx,
    vocabulary: &Vocabulary,
    event_type: &'static str,
    changes: Vec<FieldChange>,
    ip: Option<String>,
) -> Result<(), AppError> {
    publish_mutation(
        ctx.events(),
        ctx.audit(),
        tx,
        ctx.scope,
        ctx.now(),
        RecordMutation {
            record: RecordRef::new(VOCABULARY_RECORD_TYPE, vocabulary.id),
            event_type,
            payload: json!({ "key": vocabulary.key, "name": vocabulary.name }),
            changes,
            ip,
        },
    )
    .await
}

pub(crate) async fn list(ctx: &Ctx<'_>) -> Result<Vec<Vocabulary>, AppError> {
    TagStore::of(ctx).vocabularies().await
}

pub(crate) async fn create(
    ctx: &Ctx<'_>,
    input: VocabularyInput,
    ip: Option<String>,
) -> Result<Vocabulary, AppError> {
    ctx.scope.require(&Permission::SETTINGS_MANAGE)?;
    let mut violations = Vec::new();
    let key = normalize_vocabulary_key(&input.key)
        .map_err(|message| violations.push(FieldViolation::new("key", message)))
        .ok();
    let name = normalize_vocabulary_name(&input.name)
        .map_err(|message| violations.push(FieldViolation::new("name", message)))
        .ok();
    let (Some(key), Some(name)) = (key, name) else {
        return Err(AppError::invalid_fields("invalid vocabulary", violations));
    };
    let store = TagStore::of(ctx);
    if store.vocabulary_by_key(&key).await?.is_some() {
        return Err(AppError::conflict_code(
            VOCABULARY_KEY_TAKEN,
            format!("a vocabulary with the key {key} already exists"),
        ));
    }
    if store.vocabularies().await?.len() >= MAX_VOCABULARIES {
        return Err(AppError::validation_code(
            "too_many_tag_vocabularies",
            format!("an organization can have at most {MAX_VOCABULARIES} vocabularies"),
        ));
    }
    let now = ctx.now();
    let vocabulary = Vocabulary {
        id: Uuid::now_v7(),
        key,
        name,
        hierarchical: input.hierarchical,
        public: input.public,
        created_at: now,
        updated_at: now,
    };
    let mut tx = ctx.begin().await?;
    store.insert_vocabulary(&mut tx, &vocabulary).await?;
    publish(
        ctx,
        &mut tx,
        &vocabulary,
        VOCABULARY_CREATED,
        diff(&Value::Null, &snapshot(&vocabulary)),
        ip,
    )
    .await?;
    tx.commit().await?;
    Ok(vocabulary)
}

pub(crate) async fn update(
    ctx: &Ctx<'_>,
    key: &str,
    request: UpdateVocabularyRequest,
    ip: Option<String>,
) -> Result<Vocabulary, AppError> {
    ctx.scope.require(&Permission::SETTINGS_MANAGE)?;
    let current = vocabulary(ctx, Some(key)).await?;
    let mut updated = current.clone();
    if let Some(name) = &request.name {
        updated.name = normalize_vocabulary_name(name).map_err(|message| {
            AppError::invalid_fields(
                "invalid vocabulary",
                vec![FieldViolation::new("name", message)],
            )
        })?;
    }
    if let Some(hierarchical) = request.hierarchical {
        updated.hierarchical = hierarchical;
    }
    if let Some(public) = request.public {
        updated.public = public;
    }
    let changes = diff(&snapshot(&current), &snapshot(&updated));
    if changes.is_empty() {
        return Ok(current);
    }
    let store = TagStore::of(ctx);
    if current.hierarchical
        && !updated.hierarchical
        && store
            .list()
            .await?
            .iter()
            .any(|tag| tag.vocabulary_id == current.id && tag.parent_id.is_some())
    {
        return Err(AppError::conflict_code(
            VOCABULARY_HAS_HIERARCHY,
            "some terms are below others; move them to the top level first",
        ));
    }
    updated.updated_at = ctx.now();
    let mut tx = ctx.begin().await?;
    store.update_vocabulary(&mut tx, &updated).await?;
    publish(ctx, &mut tx, &updated, VOCABULARY_UPDATED, changes, ip).await?;
    tx.commit().await?;
    Ok(updated)
}

pub(crate) async fn delete(ctx: &Ctx<'_>, key: &str, ip: Option<String>) -> Result<(), AppError> {
    ctx.scope.require(&Permission::SETTINGS_MANAGE)?;
    let current = vocabulary(ctx, Some(key)).await?;
    if current.key == GENERAL_VOCABULARY {
        return Err(AppError::conflict_code(
            VOCABULARY_PROTECTED,
            "the general vocabulary cannot be deleted",
        ));
    }
    let store = TagStore::of(ctx);
    if store
        .list()
        .await?
        .iter()
        .any(|tag| tag.vocabulary_id == current.id)
    {
        return Err(AppError::conflict_code(
            VOCABULARY_NOT_EMPTY,
            "the vocabulary still has terms; delete them first",
        ));
    }
    let mut tx = ctx.begin().await?;
    store.delete_vocabulary(&mut tx, current.id).await?;
    publish(
        ctx,
        &mut tx,
        &current,
        VOCABULARY_DELETED,
        diff(&snapshot(&current), &Value::Null),
        ip,
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

pub(crate) fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list_vocabularies, create_vocabulary))
        .routes(routes!(update_vocabulary, delete_vocabulary))
}

#[utoipa::path(
    get,
    path = "/tagging/vocabularies",
    tag = TAG,
    operation_id = "tag_vocabularies_list",
    security(("session_cookie" = [])),
    responses(
        (status = 200, description = "Vocabularies of the organization. `general` holds the tags of records; others, such as categories, can be hierarchical and public on the site. Any member can list them", body = Data<Vec<VocabularyResponse>>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
    )
)]
async fn list_vocabularies(
    State(state): State<AppState>,
    tenant: AuthContext,
) -> Result<Json<Data<Vec<VocabularyResponse>>>, ApiError> {
    let vocabularies = list(&Ctx::from_state(&state, &tenant.scope).await?).await?;
    Ok(Json(Data::new(
        vocabularies.into_iter().map(Into::into).collect(),
    )))
}

#[utoipa::path(
    post,
    path = "/tagging/vocabularies",
    tag = TAG,
    operation_id = "tag_vocabularies_create",
    security(("session_cookie" = [])),
    request_body = CreateVocabularyRequest,
    responses(
        (status = 201, description = "Vocabulary created (`tag_vocabulary.created`)", body = Data<VocabularyResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 409, description = "The key is taken (`tag_vocabulary_key_taken`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid key or name (`validation_error`) or too many vocabularies (`too_many_tag_vocabularies`)", body = ErrorEnvelope),
    )
)]
async fn create_vocabulary(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    JsonBody(request): JsonBody<CreateVocabularyRequest>,
) -> Result<(StatusCode, Json<Data<VocabularyResponse>>), ApiError> {
    let vocabulary = create(
        &Ctx::from_state(&state, &tenant.scope).await?,
        VocabularyInput {
            key: request.key,
            name: request.name,
            hierarchical: request.hierarchical,
            public: request.public,
        },
        client.0.ip,
    )
    .await?;
    Ok((StatusCode::CREATED, Json(Data::new(vocabulary.into()))))
}

#[utoipa::path(
    patch,
    path = "/tagging/vocabularies/{key}",
    tag = TAG,
    operation_id = "tag_vocabularies_update",
    security(("session_cookie" = [])),
    params(("key" = String, Path, description = "Vocabulary key")),
    request_body = UpdateVocabularyRequest,
    responses(
        (status = 200, description = "Vocabulary updated (`tag_vocabulary.updated`). The key never changes", body = Data<VocabularyResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Vocabulary not found (`tag_vocabulary_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "Turning the hierarchy off while terms are nested (`tag_vocabulary_has_hierarchy`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid name (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn update_vocabulary(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(key): Path<String>,
    JsonBody(request): JsonBody<UpdateVocabularyRequest>,
) -> Result<Json<Data<VocabularyResponse>>, ApiError> {
    let vocabulary = update(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &key,
        request,
        client.0.ip,
    )
    .await?;
    Ok(Json(Data::new(vocabulary.into())))
}

#[utoipa::path(
    delete,
    path = "/tagging/vocabularies/{key}",
    tag = TAG,
    operation_id = "tag_vocabularies_delete",
    security(("session_cookie" = [])),
    params(("key" = String, Path, description = "Vocabulary key")),
    responses(
        (status = 204, description = "Vocabulary deleted (`tag_vocabulary.deleted`)"),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Vocabulary not found (`tag_vocabulary_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "The vocabulary still has terms (`tag_vocabulary_not_empty`) or is `general` (`tag_vocabulary_protected`)", body = ErrorEnvelope),
    )
)]
async fn delete_vocabulary(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(key): Path<String>,
) -> Result<StatusCode, ApiError> {
    delete(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &key,
        client.0.ip,
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}
