use std::collections::{HashMap, HashSet};

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use serde_json::{Value, json};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;
use uuid::Uuid;

use aphel_kernel::Ctx;
use aphel_kernel::api::dto::common::{Data, ErrorEnvelope, Page, PageMeta};
use aphel_kernel::api::error::ApiError;
use aphel_kernel::api::extractors::{AuthContext, JsonBody};
use aphel_kernel::application::access::Authorizer;
use aphel_kernel::application::shared::FieldChange;
use aphel_kernel::domain::access::AccessScope;
use aphel_kernel::errors::{AppError, FieldViolation};
use aphel_kernel::state::AppState;

use crate::actor::scope_for_user;
use crate::errors::{
    INVALID_ASSIGNEE, INVALID_QUERY, RECORD_NOT_ACCESSIBLE, conversation_not_found,
};
use crate::inbound::{audit_conversation, publish_on_records};
use crate::model::{
    AssignRequest, ConversationChannel, ConversationDetail, ConversationStatus,
    ConversationSummary, LinkRequest, LinkedRecord, MessageItem, ReplyRequest, StatusRequest,
};
use crate::sending::{self, whatsapp_window_end};
use crate::store::{
    self, AssigneeFilter, ConversationFilter, StoredChannel, StoredConversation, StoredMessage,
};
use crate::text::{self, timestamp};
use crate::{
    CONVERSATION_ASSIGNED, CONVERSATION_LINKED, CONVERSATION_STATUS_CHANGED, REPLY, TAG, VIEW, can,
    load_record, user_uuid,
};

const DEFAULT_PER_PAGE: u32 = 25;
const MAX_PER_PAGE: u32 = 100;
const MESSAGE_LIMIT: u64 = 300;

pub(crate) fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list_conversations))
        .routes(routes!(get_conversation))
        .routes(routes!(reply))
        .routes(routes!(assign))
        .routes(routes!(change_status))
        .routes(routes!(link))
        .routes(routes!(mark_read))
}

fn parse_id(value: &str) -> Result<Uuid, AppError> {
    Uuid::parse_str(value).map_err(|_| conversation_not_found())
}

fn record_title(record: &Value) -> Option<String> {
    let data = record.get("data").unwrap_or(record);
    let text_of = |key: &str| {
        data.get(key)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
    };
    text_of("title")
        .or_else(|| text_of("full_name"))
        .or_else(|| text_of("name"))
        .or_else(|| {
            let parts: Vec<String> = ["first_name", "last_name"]
                .iter()
                .filter_map(|key| text_of(key))
                .collect();
            (!parts.is_empty()).then(|| parts.join(" "))
        })
}

#[derive(Default)]
struct RecordCache(HashMap<(String, Uuid), Option<String>>);

impl RecordCache {
    async fn resolve(
        &mut self,
        ctx: &Ctx<'_>,
        scope: &AccessScope,
        record_type: &str,
        id: Option<Uuid>,
    ) -> Option<LinkedRecord> {
        let id = id?;
        let key = (record_type.to_owned(), id);
        if !self.0.contains_key(&key) {
            let title = match load_record(ctx, scope, record_type, id).await {
                Ok(Some(record)) => Some(record_title(&record).unwrap_or_default()),
                _ => None,
            };
            self.0.insert(key.clone(), title);
        }
        let title = self.0.get(&key).cloned().flatten();
        Some(LinkedRecord {
            id,
            accessible: title.is_some(),
            title: title.filter(|title| !title.is_empty()),
        })
    }
}

async fn summary(
    ctx: &Ctx<'_>,
    scope: &AccessScope,
    cache: &mut RecordCache,
    channel: &StoredChannel,
    conversation: StoredConversation,
) -> ConversationSummary {
    let contact = cache
        .resolve(ctx, scope, "people.person", conversation.contact_id)
        .await;
    let deal = cache
        .resolve(ctx, scope, "deals.deal", conversation.deal_id)
        .await;
    ConversationSummary {
        id: conversation.id,
        channel: ConversationChannel {
            id: channel.id,
            name: channel.name.clone(),
            kind: channel.kind,
            address: channel.address(),
            is_active: channel.is_active,
        },
        reply_window_ends_at: whatsapp_window_end(channel, &conversation).map(timestamp),
        status: ConversationStatus::parse(&conversation.status).unwrap_or(ConversationStatus::Open),
        peer: conversation.peer,
        peer_name: conversation.peer_name,
        subject: conversation.subject,
        assignee_user_id: conversation.assignee_user_id,
        unread_count: conversation.unread_count,
        last_message_at: timestamp(conversation.last_message_at),
        last_message_preview: conversation.last_message_preview,
        last_message_direction: conversation.last_message_direction,
        contact,
        deal,
        created_at: timestamp(conversation.created_at),
    }
}

pub(crate) fn message_item(message: StoredMessage) -> MessageItem {
    MessageItem {
        id: message.id,
        direction: message.direction,
        sender: message.sender,
        sender_name: message.sender_name,
        subject: message.subject,
        body: message.body,
        message_type: message.message_type,
        status: message.status,
        error_code: message.error_code,
        error_message: message.error_message,
        sent_by_user_id: message.sent_by_user_id,
        provider_timestamp: message.provider_timestamp.map(timestamp),
        sent_at: message.sent_at.map(timestamp),
        delivered_at: message.delivered_at.map(timestamp),
        read_at: message.read_at.map(timestamp),
        created_at: timestamp(message.created_at),
    }
}

async fn channel_of(
    ctx: &Ctx<'_>,
    conversation: &StoredConversation,
) -> Result<StoredChannel, AppError> {
    store::find_channel(ctx.pool(), conversation.channel_id)
        .await?
        .ok_or_else(conversation_not_found)
}

async fn single_summary(
    ctx: &Ctx<'_>,
    scope: &AccessScope,
    conversation: StoredConversation,
) -> Result<ConversationSummary, AppError> {
    let channel = channel_of(ctx, &conversation).await?;
    Ok(summary(
        ctx,
        scope,
        &mut RecordCache::default(),
        &channel,
        conversation,
    )
    .await)
}

fn uuid_param(
    params: &HashMap<String, String>,
    key: &str,
    violations: &mut Vec<FieldViolation>,
) -> Option<Uuid> {
    let value = params
        .get(key)
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())?;
    match Uuid::parse_str(value) {
        Ok(id) => Some(id),
        Err(_) => {
            violations.push(FieldViolation::new(key, "identificador inválido"));
            None
        }
    }
}

fn parse_filter(
    scope: &AccessScope,
    params: &HashMap<String, String>,
) -> Result<(ConversationFilter, u32, u32), AppError> {
    let mut violations = Vec::new();
    let mut filter = ConversationFilter {
        channel_id: uuid_param(params, "channel_id", &mut violations),
        contact_id: uuid_param(params, "contact_id", &mut violations),
        deal_id: uuid_param(params, "deal_id", &mut violations),
        ..ConversationFilter::default()
    };
    if let Some(status) = params.get("status").filter(|value| !value.is_empty()) {
        match ConversationStatus::parse(status) {
            Some(status) => filter.status = Some(status.as_str().to_owned()),
            None => violations.push(FieldViolation::new(
                "status",
                "use open, pending ou resolved",
            )),
        }
    }
    filter.assignee = match params.get("assignee").map(String::as_str) {
        None | Some("") | Some("any") => None,
        Some("me") => Some(AssigneeFilter::User(user_uuid(scope)?)),
        Some("none") => Some(AssigneeFilter::Unassigned),
        Some(value) => match Uuid::parse_str(value) {
            Ok(id) => Some(AssigneeFilter::User(id)),
            Err(_) => {
                violations.push(FieldViolation::new(
                    "assignee",
                    "use me, none, any ou um id de usuário",
                ));
                None
            }
        },
    };
    filter.unread = matches!(params.get("unread").map(String::as_str), Some("true" | "1"));
    filter.search = params
        .get("search")
        .map(|value| text::single_line(value, 100))
        .filter(|value| !value.is_empty());
    let number = |key: &str, default: u32| -> Option<u32> {
        match params.get(key).filter(|value| !value.is_empty()) {
            None => Some(default),
            Some(value) => value.parse::<u32>().ok().filter(|value| *value >= 1),
        }
    };
    let page = number("page", 1).unwrap_or_else(|| {
        violations.push(FieldViolation::new("page", "página inválida"));
        1
    });
    let per_page = number("per_page", DEFAULT_PER_PAGE)
        .filter(|value| *value <= MAX_PER_PAGE)
        .unwrap_or_else(|| {
            violations.push(FieldViolation::new("per_page", "use de 1 a 100"));
            DEFAULT_PER_PAGE
        });
    if !violations.is_empty() {
        return Err(AppError::invalid_fields_code(
            INVALID_QUERY,
            "invalid query",
            violations,
        ));
    }
    Ok((filter, page, per_page))
}

#[utoipa::path(
    get,
    path = "/messaging/conversations",
    tag = TAG,
    operation_id = "messaging_list_conversations",
    security(("session_cookie" = [])),
    params(
        ("channel_id" = Option<String>, Query, description = "Only this channel"),
        ("status" = Option<String>, Query, description = "`open`, `pending` or `resolved`"),
        ("assignee" = Option<String>, Query, description = "`me`, `none` (unassigned), `any` or a user id"),
        ("contact_id" = Option<String>, Query, description = "Conversations linked to this contact"),
        ("deal_id" = Option<String>, Query, description = "Conversations linked to this deal"),
        ("unread" = Option<bool>, Query, description = "Only conversations with unread messages"),
        ("search" = Option<String>, Query, description = "Text in the phone/e-mail, name, subject or last message"),
        ("page" = Option<u32>, Query, description = "Page, from 1"),
        ("per_page" = Option<u32>, Query, description = "Items per page, up to 100 (default 25)"),
    ),
    responses(
        (status = 200, description = "Conversations, most recent activity first. Linked contact/deal come with `accessible = false` and no title when the user cannot read them", body = Page<ConversationSummary>),
        (status = 403, description = "Missing `messaging.view` (`permission_denied`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid filter (`messaging_invalid_query` with `details.fields`)", body = ErrorEnvelope),
    )
)]
async fn list_conversations(
    State(state): State<AppState>,
    tenant: AuthContext,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<Page<ConversationSummary>>, ApiError> {
    let scope = &tenant.scope;
    let ctx = Ctx::from_state(&state, scope).await?;
    scope.require(&VIEW)?;
    let (filter, page, per_page) = parse_filter(scope, &params)?;
    let pool = ctx.pool().clone();
    let (conversations, total) = store::list_conversations(
        &pool,
        &filter,
        u64::from(page - 1) * u64::from(per_page),
        u64::from(per_page),
    )
    .await?;
    let channels = store::list_channels(&pool).await?;
    let mut cache = RecordCache::default();
    let mut data = Vec::with_capacity(conversations.len());
    for conversation in conversations {
        let Some(channel) = channels
            .iter()
            .find(|channel| channel.id == conversation.channel_id)
        else {
            continue;
        };
        data.push(summary(&ctx, scope, &mut cache, channel, conversation).await);
    }
    Ok(Json(Page {
        data,
        meta: PageMeta {
            total,
            page,
            per_page,
        },
    }))
}

#[utoipa::path(
    get,
    path = "/messaging/conversations/{id}",
    tag = TAG,
    operation_id = "messaging_get_conversation",
    security(("session_cookie" = [])),
    params(("id" = String, Path, description = "Conversation id")),
    responses(
        (status = 200, description = "The conversation with its latest 300 messages, oldest first", body = Data<ConversationDetail>),
        (status = 403, description = "Missing `messaging.view` (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Unknown conversation (`messaging_conversation_not_found`)", body = ErrorEnvelope),
    )
)]
async fn get_conversation(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(id): Path<String>,
) -> Result<Json<Data<ConversationDetail>>, ApiError> {
    let scope = &tenant.scope;
    let ctx = Ctx::from_state(&state, scope).await?;
    scope.require(&VIEW)?;
    let pool = ctx.pool().clone();
    let conversation = store::find_conversation(&pool, parse_id(&id)?)
        .await?
        .ok_or_else(conversation_not_found)?;
    let (messages, total_messages) =
        store::list_messages(&pool, conversation.id, MESSAGE_LIMIT).await?;
    Ok(Json(Data {
        data: ConversationDetail {
            conversation: single_summary(&ctx, scope, conversation).await?,
            messages: messages.into_iter().map(message_item).collect(),
            total_messages,
        },
    }))
}

#[utoipa::path(
    post,
    path = "/messaging/conversations/{id}/messages",
    tag = TAG,
    operation_id = "messaging_reply",
    security(("session_cookie" = [])),
    params(("id" = String, Path, description = "Conversation id")),
    request_body = ReplyRequest,
    responses(
        (status = 202, description = "Reply queued (`status = queued`) and sent in the background by the job `messaging.send` with retries. Publishes `message.sent`", body = Data<MessageItem>),
        (status = 403, description = "Missing `messaging.reply` (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Unknown conversation (`messaging_conversation_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "The channel is inactive (`messaging_channel_inactive`)", body = ErrorEnvelope),
        (status = 422, description = "Empty or too long message (`messaging_invalid_reply`), or WhatsApp conversation outside the 24-hour window (`whatsapp_window_closed`)", body = ErrorEnvelope),
    )
)]
async fn reply(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(id): Path<String>,
    JsonBody(request): JsonBody<ReplyRequest>,
) -> Result<(StatusCode, Json<Data<MessageItem>>), ApiError> {
    let scope = &tenant.scope;
    let ctx = Ctx::from_state(&state, scope).await?;
    scope.require(&VIEW)?;
    scope.require(&REPLY)?;
    let (_, _, message) = sending::queue_reply(&ctx, scope, parse_id(&id)?, &request.body).await?;
    Ok((
        StatusCode::ACCEPTED,
        Json(Data {
            data: message_item(message),
        }),
    ))
}

async fn mutate<F>(
    ctx: &Ctx<'_>,
    scope: &AccessScope,
    id: Uuid,
    event_type: &'static str,
    apply: F,
) -> Result<StoredConversation, AppError>
where
    F: FnOnce(&mut StoredConversation) -> Vec<(&'static str, Value, Value)>,
{
    let mut tx = ctx.begin().await?;
    let mut conversation = store::find_conversation_for_update(&mut tx, id)
        .await?
        .ok_or_else(conversation_not_found)?;
    let changes = apply(&mut conversation);
    if changes.is_empty() {
        tx.rollback().await?;
        return Ok(conversation);
    }
    store::save_conversation(&mut tx, &conversation).await?;
    let mut payload = json!({
        "conversation_id": conversation.id,
        "channel_id": conversation.channel_id,
        "peer": conversation.peer,
        "status": conversation.status,
        "assignee_user_id": conversation.assignee_user_id,
        "contact_id": conversation.contact_id,
        "deal_id": conversation.deal_id,
    });
    if let Value::Object(map) = &mut payload {
        map.insert(
            "changes".to_owned(),
            json!(
                changes
                    .iter()
                    .map(|(field, old, new)| json!({ "field": field, "old": old, "new": new }))
                    .collect::<Vec<_>>()
            ),
        );
    }
    publish_on_records(ctx, &mut tx, scope, event_type, &conversation, payload).await?;
    let changes = changes
        .into_iter()
        .map(|(field, old, new)| FieldChange::new(field, old, new))
        .chain([FieldChange::new(
            "conversation_id",
            Value::Null,
            json!(conversation.id),
        )])
        .collect();
    audit_conversation(ctx, &mut tx, scope, event_type, &conversation, changes).await?;
    tx.commit().await?;
    Ok(conversation)
}

#[utoipa::path(
    put,
    path = "/messaging/conversations/{id}/assignee",
    tag = TAG,
    operation_id = "messaging_assign",
    security(("session_cookie" = [])),
    params(("id" = String, Path, description = "Conversation id")),
    request_body = AssignRequest,
    responses(
        (status = 200, description = "Assignee changed (`null` unassigns). Publishes `conversation.assigned`", body = Data<ConversationSummary>),
        (status = 403, description = "Missing `messaging.reply` (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Unknown conversation (`messaging_conversation_not_found`)", body = ErrorEnvelope),
        (status = 422, description = "The user is not an active member who can see the inbox (`messaging_invalid_assignee`)", body = ErrorEnvelope),
    )
)]
async fn assign(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(id): Path<String>,
    JsonBody(request): JsonBody<AssignRequest>,
) -> Result<Json<Data<ConversationSummary>>, ApiError> {
    let scope = &tenant.scope;
    let ctx = Ctx::from_state(&state, scope).await?;
    scope.require(&REPLY)?;
    let id = parse_id(&id)?;
    if let Some(user_id) = request.user_id {
        let member = scope_for_user(&ctx, user_id)
            .await?
            .is_some_and(|member| can(&member, &VIEW));
        if !member {
            return Err(AppError::invalid_fields_code(
                INVALID_ASSIGNEE,
                "the user must be an active member with access to the inbox",
                vec![FieldViolation::new(
                    "user_id",
                    "usuário sem acesso à caixa de entrada",
                )],
            )
            .into());
        }
    }
    let conversation = mutate(&ctx, scope, id, CONVERSATION_ASSIGNED, |conversation| {
        if conversation.assignee_user_id == request.user_id {
            return Vec::new();
        }
        let old = json!(conversation.assignee_user_id);
        conversation.assignee_user_id = request.user_id;
        vec![("assignee_user_id", old, json!(request.user_id))]
    })
    .await?;
    Ok(Json(Data {
        data: single_summary(&ctx, scope, conversation).await?,
    }))
}

#[utoipa::path(
    put,
    path = "/messaging/conversations/{id}/status",
    tag = TAG,
    operation_id = "messaging_change_status",
    security(("session_cookie" = [])),
    params(("id" = String, Path, description = "Conversation id")),
    request_body = StatusRequest,
    responses(
        (status = 200, description = "Status changed (`open`, `pending`, `resolved`). A new incoming message reopens a resolved conversation. Publishes `conversation.status_changed`", body = Data<ConversationSummary>),
        (status = 403, description = "Missing `messaging.reply` (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Unknown conversation (`messaging_conversation_not_found`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid body (`invalid_json`)", body = ErrorEnvelope),
    )
)]
async fn change_status(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(id): Path<String>,
    JsonBody(request): JsonBody<StatusRequest>,
) -> Result<Json<Data<ConversationSummary>>, ApiError> {
    let scope = &tenant.scope;
    let ctx = Ctx::from_state(&state, scope).await?;
    scope.require(&REPLY)?;
    let status = request.status.as_str();
    let conversation = mutate(
        &ctx,
        scope,
        parse_id(&id)?,
        CONVERSATION_STATUS_CHANGED,
        |conversation| {
            if conversation.status == status {
                return Vec::new();
            }
            let old = json!(conversation.status);
            conversation.status = status.to_owned();
            if status == "resolved" {
                conversation.unread_count = 0;
            }
            vec![("status", old, json!(status))]
        },
    )
    .await?;
    Ok(Json(Data {
        data: single_summary(&ctx, scope, conversation).await?,
    }))
}

async fn require_readable(
    ctx: &Ctx<'_>,
    scope: &AccessScope,
    record_type: &'static str,
    id: Option<Uuid>,
    field: &'static str,
    violations: &mut Vec<FieldViolation>,
) {
    let Some(id) = id else {
        return;
    };
    let readable = matches!(load_record(ctx, scope, record_type, id).await, Ok(Some(_)));
    if !readable {
        violations.push(FieldViolation::new(
            field,
            "registro inexistente ou sem acesso",
        ));
    }
}

#[utoipa::path(
    put,
    path = "/messaging/conversations/{id}/links",
    tag = TAG,
    operation_id = "messaging_link",
    security(("session_cookie" = [])),
    params(("id" = String, Path, description = "Conversation id")),
    request_body = LinkRequest,
    responses(
        (status = 200, description = "Links set to the given contact and deal (`null` removes the link). Both records must be readable by the user, and the user must be able to read the records currently linked to change them. Publishes `conversation.linked`", body = Data<ConversationSummary>),
        (status = 403, description = "Missing `messaging.reply` (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Unknown conversation (`messaging_conversation_not_found`)", body = ErrorEnvelope),
        (status = 422, description = "Record not found or not readable (`messaging_record_not_accessible` with `details.fields`)", body = ErrorEnvelope),
    )
)]
async fn link(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(id): Path<String>,
    JsonBody(request): JsonBody<LinkRequest>,
) -> Result<Json<Data<ConversationSummary>>, ApiError> {
    let scope = &tenant.scope;
    let ctx = Ctx::from_state(&state, scope).await?;
    scope.require(&REPLY)?;
    let id = parse_id(&id)?;
    let current = store::find_conversation(ctx.pool(), id)
        .await?
        .ok_or_else(conversation_not_found)?;
    let mut violations = Vec::new();
    let mut checked = HashSet::new();
    for (record_type, field, old, new) in [
        (
            "people.person",
            "contact_id",
            current.contact_id,
            request.contact_id,
        ),
        ("deals.deal", "deal_id", current.deal_id, request.deal_id),
    ] {
        if old == new {
            continue;
        }
        for value in [old, new].into_iter().flatten() {
            if checked.insert((record_type, value)) {
                require_readable(
                    &ctx,
                    scope,
                    record_type,
                    Some(value),
                    field,
                    &mut violations,
                )
                .await;
            }
        }
    }
    if !violations.is_empty() {
        return Err(AppError::invalid_fields_code(
            RECORD_NOT_ACCESSIBLE,
            "the record does not exist or the user cannot read it",
            violations,
        )
        .into());
    }
    let conversation = mutate(&ctx, scope, id, CONVERSATION_LINKED, |conversation| {
        let mut changes = Vec::new();
        if conversation.contact_id != request.contact_id {
            changes.push((
                "contact_id",
                json!(conversation.contact_id),
                json!(request.contact_id),
            ));
            conversation.contact_id = request.contact_id;
        }
        if conversation.deal_id != request.deal_id {
            changes.push((
                "deal_id",
                json!(conversation.deal_id),
                json!(request.deal_id),
            ));
            conversation.deal_id = request.deal_id;
        }
        changes
    })
    .await?;
    Ok(Json(Data {
        data: single_summary(&ctx, scope, conversation).await?,
    }))
}

#[utoipa::path(
    post,
    path = "/messaging/conversations/{id}/read",
    tag = TAG,
    operation_id = "messaging_mark_read",
    security(("session_cookie" = [])),
    params(("id" = String, Path, description = "Conversation id")),
    responses(
        (status = 204, description = "Unread counter cleared"),
        (status = 403, description = "Missing `messaging.view` (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Unknown conversation (`messaging_conversation_not_found`)", body = ErrorEnvelope),
    )
)]
async fn mark_read(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let scope = &tenant.scope;
    let ctx = Ctx::from_state(&state, scope).await?;
    scope.require(&VIEW)?;
    let pool = ctx.pool().clone();
    let conversation = store::find_conversation(&pool, parse_id(&id)?)
        .await?
        .ok_or_else(conversation_not_found)?;
    store::mark_conversation_read(&pool, conversation.id).await?;
    Ok(StatusCode::NO_CONTENT)
}
