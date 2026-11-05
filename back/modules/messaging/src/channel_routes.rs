use aphel_sdk::messaging::channel_provider;
use std::collections::BTreeMap;
use std::sync::LazyLock;
use std::time::Duration;

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use serde_json::{Value, json};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;
use uuid::Uuid;

use crate::actor::scope_for_user;
use crate::channels::{self, ChannelContext, ChannelKind};
use crate::errors::{CHANNEL_IN_USE, CHANNEL_INVALID, CHANNEL_LIMIT_REACHED, channel_not_found};
use crate::model::{ChannelRequest, ChannelResponse, ChannelTestResult};
use crate::store::{self, StoredChannel};
use crate::text::{self, timestamp};
use crate::{
    CHANNEL_CREATED, CHANNEL_DELETED, CHANNEL_UPDATED, MANAGE_CHANNELS, PUBLIC_WHATSAPP_PATH, TAG,
    VIEW, can, user_uuid,
};
use aphel_kernel::Ctx;
use aphel_kernel::errors::{AppError, FieldViolation};
use aphel_kernel::state::AppState;
use aphel_sdk::platform::api::dto::common::{Data, ErrorEnvelope, Page, PageMeta};
use aphel_sdk::platform::api::error::ApiError;
use aphel_sdk::platform::api::extractors::{AuthContext, JsonBody};
use aphel_sdk::platform::api::rate_limit::SharedRateLimiter;
use aphel_sdk::platform::application::access::Authorizer;
use aphel_sdk::platform::application::shared::{Actor, AuditEntry, DomainEvent, FieldChange};
use aphel_sdk::platform::database::transaction::MySqlTx;
use aphel_sdk::platform::domain::access::AccessScope;

const MAX_CHANNELS: u64 = 50;
const MAX_SECRET_LENGTH: usize = 4096;
const REDACTED: &str = "[redacted]";

static TEST_LIMITER: LazyLock<SharedRateLimiter> =
    LazyLock::new(|| SharedRateLimiter::new("messaging.test", 20, Duration::from_secs(60)));

pub(crate) fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list_providers))
        .routes(routes!(list_channels, create_channel))
        .routes(routes!(get_channel, update_channel, delete_channel))
        .routes(routes!(test_channel))
}

fn parse_id(value: &str) -> Result<Uuid, AppError> {
    Uuid::parse_str(value).map_err(|_| channel_not_found())
}

pub(crate) fn webhook_url(ctx: &Ctx<'_>, channel: &StoredChannel) -> Option<String> {
    (channel.kind == ChannelKind::parse("whatsapp").expect("channel key")).then(|| {
        format!(
            "{}{PUBLIC_WHATSAPP_PATH}/{}",
            ctx.runtime().public_base_url().get().trim_end_matches('/'),
            channel.id
        )
    })
}

async fn response(
    ctx: &Ctx<'_>,
    scope: &AccessScope,
    channel: StoredChannel,
) -> Result<ChannelResponse, AppError> {
    let manager = can(scope, &MANAGE_CHANNELS);
    let secrets = if manager {
        Some(channels::configured_secrets(ctx, &channel).await?)
    } else {
        None
    };
    Ok(ChannelResponse {
        id: channel.id,
        kind: channel.kind.clone(),
        address: channel.address(),
        name: channel.name.clone(),
        is_active: channel.is_active,
        status: channel.status.clone(),
        last_error: if manager {
            channel.last_error.clone()
        } else {
            None
        },
        last_checked_at: channel.last_checked_at.map(timestamp),
        last_polled_at: channel.last_polled_at.map(timestamp),
        acting_user_id: channel.acting_user_id,
        created_by: channel.created_by,
        config: manager.then(|| channel.config.clone()),
        secrets,
        webhook_url: if manager {
            webhook_url(ctx, &channel)
        } else {
            None
        },
        created_at: timestamp(channel.created_at),
        updated_at: timestamp(channel.updated_at),
    })
}

struct ValidatedChannel {
    kind: ChannelKind,
    name: String,
    is_active: bool,
    acting_user_id: Option<Uuid>,
    config: Value,
    secrets: BTreeMap<String, String>,
    clear: Vec<String>,
}

async fn validate(
    ctx: &Ctx<'_>,
    request: ChannelRequest,
    existing: Option<(&StoredChannel, &BTreeMap<String, bool>)>,
) -> Result<ValidatedChannel, AppError> {
    let mut violations = Vec::new();
    let kind = match (request.kind, existing) {
        (Some(kind), Some((channel, _))) if kind != channel.kind => {
            violations.push(FieldViolation::new(
                "kind",
                "o tipo do canal não pode mudar",
            ));
            channel.kind.clone()
        }
        (_, Some((channel, _))) => channel.kind.clone(),
        (Some(kind), None) => kind,
        (None, None) => {
            return Err(AppError::invalid_fields(
                "invalid channel",
                vec![FieldViolation::new("kind", "escolha WhatsApp ou e-mail")],
            ));
        }
    };
    let handler = channel_provider(kind.as_str())?;
    let name = text::single_line(&request.name, 200);
    if name.is_empty() || name.chars().count() > 120 {
        violations.push(FieldViolation::new(
            "name",
            "informe um nome de até 120 caracteres",
        ));
    }
    let validated = handler.validate_config(&request.config);
    violations.extend(validated.violations);
    let mut secrets = BTreeMap::new();
    for (key, value) in request.secrets {
        if !handler.secret_names().contains(&key.as_str()) {
            violations.push(FieldViolation::new(
                format!("secrets.{key}"),
                "credencial desconhecida",
            ));
            continue;
        }
        let value = value.trim().to_owned();
        if value.len() > MAX_SECRET_LENGTH {
            violations.push(FieldViolation::new(
                format!("secrets.{key}"),
                "valor longo demais",
            ));
            continue;
        }
        if !value.is_empty() {
            secrets.insert(key, value);
        }
    }
    let mut clear = Vec::new();
    for key in request.clear_secrets {
        if handler.required_secrets().contains(&key.as_str())
            || !handler.secret_names().contains(&key.as_str())
        {
            violations.push(FieldViolation::new(
                format!("clear_secrets.{key}"),
                "esta credencial não pode ser removida",
            ));
        } else if !secrets.contains_key(&key) {
            clear.push(key);
        }
    }
    for required in handler.required_secrets() {
        let configured = existing
            .and_then(|(_, configured)| configured.get(*required).copied())
            .unwrap_or(false);
        if !configured && !secrets.contains_key(*required) {
            violations.push(FieldViolation::new(
                format!("secrets.{required}"),
                "credencial obrigatória",
            ));
        }
    }
    if let Some(user_id) = request.acting_user_id
        && scope_for_user(ctx, user_id).await?.is_none()
    {
        violations.push(FieldViolation::new(
            "acting_user_id",
            "o usuário precisa ser membro ativo da organização",
        ));
    }
    if !violations.is_empty() {
        return Err(AppError::invalid_fields_code(
            CHANNEL_INVALID,
            "invalid channel",
            violations,
        ));
    }
    Ok(ValidatedChannel {
        kind,
        name,
        is_active: request
            .is_active
            .unwrap_or_else(|| existing.is_none_or(|(channel, _)| channel.is_active)),
        acting_user_id: request.acting_user_id,
        config: validated.config,
        secrets,
        clear,
    })
}

fn audit_changes(
    before: Option<&StoredChannel>,
    after: Option<&StoredChannel>,
    secrets: &[String],
) -> Vec<FieldChange> {
    let describe = |channel: Option<&StoredChannel>| {
        channel.map_or_else(
            || {
                [
                    Value::Null,
                    Value::Null,
                    Value::Null,
                    Value::Null,
                    Value::Null,
                ]
            },
            |channel| {
                [
                    json!(channel.kind.as_str()),
                    json!(channel.name),
                    json!(channel.is_active),
                    json!(channel.acting_user_id),
                    channel.config.clone(),
                ]
            },
        )
    };
    let old = describe(before);
    let new = describe(after);
    ["kind", "name", "is_active", "acting_user_id", "config"]
        .into_iter()
        .zip(old.into_iter().zip(new))
        .filter(|(_, (old, new))| old != new)
        .map(|(field, (old, new))| FieldChange::new(field, old, new))
        .chain(secrets.iter().map(|name| {
            FieldChange::new(format!("secrets.{name}"), json!(REDACTED), json!(REDACTED))
        }))
        .collect()
}

async fn record_change(
    ctx: &Ctx<'_>,
    tx: &mut MySqlTx,
    scope: &AccessScope,
    event_type: &str,
    payload: Value,
    changes: Vec<FieldChange>,
) -> Result<(), AppError> {
    let actor = Actor::from_scope(scope);
    let now = chrono::Utc::now();
    ctx.events()
        .publish(
            tx,
            DomainEvent::new(actor.clone(), event_type, now).with_payload(payload),
        )
        .await?;
    ctx.audit()
        .record(
            tx,
            AuditEntry::new(actor, event_type, now).with_changes(changes),
        )
        .await
}

async fn persist(
    ctx: &Ctx<'_>,
    scope: &AccessScope,
    event_type: &'static str,
    before: Option<&StoredChannel>,
    channel: &StoredChannel,
    validated: &ValidatedChannel,
) -> Result<(), AppError> {
    let mut tx = ctx.begin().await?;
    if before.is_some() {
        store::update_channel(&mut tx, channel).await?;
    } else {
        store::insert_channel(&mut tx, channel).await?;
    }
    for (name, value) in &validated.secrets {
        channels::store_credential(ctx, &mut tx, channel.id, name, value).await?;
    }
    for name in &validated.clear {
        channels::remove_credential(ctx, &mut tx, channel.id, name).await?;
    }
    let touched: Vec<String> = validated
        .secrets
        .keys()
        .cloned()
        .chain(validated.clear.iter().cloned())
        .collect();
    record_change(
        ctx,
        &mut tx,
        scope,
        event_type,
        json!({
            "channel_id": channel.id,
            "kind": channel.kind.as_str(),
            "name": channel.name,
            "is_active": channel.is_active,
        }),
        audit_changes(before, Some(channel), &touched),
    )
    .await?;
    Ok(tx.commit().await?)
}

#[utoipa::path(
    get,
    path = "/messaging/channels",
    tag = TAG,
    operation_id = "messaging_list_channels",
    security(("session_cookie" = [])),
    responses(
        (status = 200, description = "Channels of the installation. `config`, `secrets` (only whether each credential is configured), `last_error` and `webhook_url` come only with `messaging.manage_channels`", body = Page<ChannelResponse>),
        (status = 403, description = "Missing `messaging.view` and `messaging.manage_channels` (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Module disabled (`module_disabled`)", body = ErrorEnvelope),
    )
)]
async fn list_channels(
    State(state): State<AppState>,
    tenant: AuthContext,
) -> Result<Json<Page<ChannelResponse>>, ApiError> {
    let scope = &tenant.scope;
    let ctx = Ctx::from_state(&state, scope).await?;
    if !can(scope, &MANAGE_CHANNELS) {
        scope.require(&VIEW)?;
    }
    let channels = store::list_channels(ctx.pool()).await?;
    let total = channels.len();
    let mut data = Vec::with_capacity(total);
    for channel in channels {
        data.push(response(&ctx, scope, channel).await?);
    }
    Ok(Json(Page {
        data,
        meta: PageMeta {
            total: total as u64,
            page: 1,
            per_page: u32::try_from(total.max(1)).unwrap_or(u32::MAX),
        },
    }))
}

#[utoipa::path(
    post,
    path = "/messaging/channels",
    tag = TAG,
    operation_id = "messaging_create_channel",
    security(("session_cookie" = [])),
    request_body = ChannelRequest,
    responses(
        (status = 201, description = "Channel created; credentials are encrypted and never returned. Publishes `messaging_channel.created`", body = Data<ChannelResponse>),
        (status = 403, description = "Missing `messaging.manage_channels` (`permission_denied`)", body = ErrorEnvelope),
        (status = 409, description = "The installation reached 50 channels (`messaging_channel_limit_reached`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid channel (`messaging_channel_invalid` with `details.fields`) or body (`invalid_json`)", body = ErrorEnvelope),
    )
)]
async fn create_channel(
    State(state): State<AppState>,
    tenant: AuthContext,
    JsonBody(request): JsonBody<ChannelRequest>,
) -> Result<(StatusCode, Json<Data<ChannelResponse>>), ApiError> {
    let scope = &tenant.scope;
    let ctx = Ctx::from_state(&state, scope).await?;
    scope.require(&MANAGE_CHANNELS)?;
    let pool = ctx.pool().clone();
    if store::count_channels(&pool).await? >= MAX_CHANNELS {
        return Err(AppError::conflict_code(
            CHANNEL_LIMIT_REACHED,
            "the limit of 50 channels was reached",
        )
        .into());
    }
    let validated = validate(&ctx, request, None).await?;
    let now = text::now();
    let channel = StoredChannel {
        id: Uuid::now_v7(),
        kind: validated.kind.clone(),
        name: validated.name.clone(),
        config: validated.config.clone(),
        is_active: validated.is_active,
        status: "pending".to_owned(),
        last_error: None,
        last_checked_at: None,
        last_polled_at: None,
        imap_uid_validity: None,
        imap_last_uid: None,
        acting_user_id: validated.acting_user_id,
        created_by: user_uuid(scope)?,
        created_at: now,
        updated_at: now,
    };
    persist(&ctx, scope, CHANNEL_CREATED, None, &channel, &validated).await?;
    let channel = store::find_channel(&pool, channel.id)
        .await?
        .ok_or_else(channel_not_found)?;
    Ok((
        StatusCode::CREATED,
        Json(Data {
            data: response(&ctx, scope, channel).await?,
        }),
    ))
}

#[utoipa::path(
    get,
    path = "/messaging/channels/{id}",
    tag = TAG,
    operation_id = "messaging_get_channel",
    security(("session_cookie" = [])),
    params(("id" = String, Path, description = "Channel id")),
    responses(
        (status = 200, description = "The channel, with configuration and which credentials are configured", body = Data<ChannelResponse>),
        (status = 403, description = "Missing `messaging.manage_channels` (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Unknown channel (`messaging_channel_not_found`)", body = ErrorEnvelope),
    )
)]
async fn get_channel(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(id): Path<String>,
) -> Result<Json<Data<ChannelResponse>>, ApiError> {
    let scope = &tenant.scope;
    let ctx = Ctx::from_state(&state, scope).await?;
    scope.require(&MANAGE_CHANNELS)?;
    let channel = store::find_channel(ctx.pool(), parse_id(&id)?)
        .await?
        .ok_or_else(channel_not_found)?;
    Ok(Json(Data {
        data: response(&ctx, scope, channel).await?,
    }))
}

#[utoipa::path(
    put,
    path = "/messaging/channels/{id}",
    tag = TAG,
    operation_id = "messaging_update_channel",
    security(("session_cookie" = [])),
    params(("id" = String, Path, description = "Channel id")),
    request_body = ChannelRequest,
    responses(
        (status = 200, description = "Channel saved. Credentials left empty keep their current value; `clear_secrets` removes optional ones. Changing the IMAP account restarts reading from new messages. Publishes `messaging_channel.updated`", body = Data<ChannelResponse>),
        (status = 403, description = "Missing `messaging.manage_channels` (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Unknown channel (`messaging_channel_not_found`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid channel (`messaging_channel_invalid` with `details.fields`)", body = ErrorEnvelope),
    )
)]
async fn update_channel(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(id): Path<String>,
    JsonBody(request): JsonBody<ChannelRequest>,
) -> Result<Json<Data<ChannelResponse>>, ApiError> {
    let scope = &tenant.scope;
    let ctx = Ctx::from_state(&state, scope).await?;
    scope.require(&MANAGE_CHANNELS)?;
    let pool = ctx.pool().clone();
    let before = store::find_channel(&pool, parse_id(&id)?)
        .await?
        .ok_or_else(channel_not_found)?;
    let configured = channels::configured_secrets(&ctx, &before).await?;
    let validated = validate(&ctx, request, Some((&before, &configured))).await?;
    let mailbox_changed = ["imap_host", "imap_port", "imap_username"]
        .iter()
        .any(|key| before.config.get(*key) != validated.config.get(*key));
    let changed = before.config != validated.config
        || !validated.secrets.is_empty()
        || !validated.clear.is_empty();
    let channel = StoredChannel {
        name: validated.name.clone(),
        config: validated.config.clone(),
        is_active: validated.is_active,
        acting_user_id: validated.acting_user_id,
        status: if changed {
            "pending".to_owned()
        } else {
            before.status.clone()
        },
        last_error: if changed {
            None
        } else {
            before.last_error.clone()
        },
        imap_uid_validity: if mailbox_changed {
            None
        } else {
            before.imap_uid_validity
        },
        imap_last_uid: if mailbox_changed {
            None
        } else {
            before.imap_last_uid
        },
        updated_at: text::now(),
        ..before.clone()
    };
    persist(
        &ctx,
        scope,
        CHANNEL_UPDATED,
        Some(&before),
        &channel,
        &validated,
    )
    .await?;
    let channel = store::find_channel(&pool, channel.id)
        .await?
        .ok_or_else(channel_not_found)?;
    Ok(Json(Data {
        data: response(&ctx, scope, channel).await?,
    }))
}

#[utoipa::path(
    delete,
    path = "/messaging/channels/{id}",
    tag = TAG,
    operation_id = "messaging_delete_channel",
    security(("session_cookie" = [])),
    params(("id" = String, Path, description = "Channel id")),
    responses(
        (status = 204, description = "Channel and its credentials removed. Publishes `messaging_channel.deleted`"),
        (status = 403, description = "Missing `messaging.manage_channels` (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Unknown channel (`messaging_channel_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "The channel has conversations; deactivate it instead (`messaging_channel_in_use`)", body = ErrorEnvelope),
    )
)]
async fn delete_channel(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let scope = &tenant.scope;
    let ctx = Ctx::from_state(&state, scope).await?;
    scope.require(&MANAGE_CHANNELS)?;
    let pool = ctx.pool().clone();
    let channel = store::find_channel(&pool, parse_id(&id)?)
        .await?
        .ok_or_else(channel_not_found)?;
    if store::count_channel_conversations(&pool, channel.id).await? > 0 {
        return Err(AppError::conflict_code(
            CHANNEL_IN_USE,
            "the channel has conversations; deactivate it to keep the history",
        )
        .into());
    }
    let mut tx = ctx.begin().await?;
    store::delete_channel(&mut tx, channel.id).await?;
    channels::remove_credentials(&ctx, &mut tx, &channel).await?;
    record_change(
        &ctx,
        &mut tx,
        scope,
        CHANNEL_DELETED,
        json!({
            "channel_id": channel.id,
            "kind": channel.kind.as_str(),
            "name": channel.name,
        }),
        audit_changes(Some(&channel), None, &[]),
    )
    .await?;
    tx.commit().await.map_err(AppError::from)?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    post,
    path = "/messaging/channels/{id}/test",
    tag = TAG,
    operation_id = "messaging_test_channel",
    security(("session_cookie" = [])),
    params(("id" = String, Path, description = "Channel id")),
    responses(
        (status = 200, description = "Result of a connection test with the stored credentials (WhatsApp: reads the phone number at Meta; e-mail: IMAP login and SMTP connection). Updates the channel status", body = Data<ChannelTestResult>),
        (status = 403, description = "Missing `messaging.manage_channels` (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Unknown channel (`messaging_channel_not_found`)", body = ErrorEnvelope),
        (status = 429, description = "Too many tests in a short time (`rate_limited`)", body = ErrorEnvelope),
    )
)]
async fn test_channel(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(id): Path<String>,
) -> Result<Json<Data<ChannelTestResult>>, ApiError> {
    let scope = &tenant.scope;
    let ctx = Ctx::from_state(&state, scope).await?;
    scope.require(&MANAGE_CHANNELS)?;
    TEST_LIMITER
        .check("test", "too many tests, wait a minute")
        .await?;
    let pool = ctx.pool().clone();
    let channel = store::find_channel(&pool, parse_id(&id)?)
        .await?
        .ok_or_else(channel_not_found)?;
    let credentials = channels::load_credentials(&ctx, &channel).await?;
    let context = ChannelContext {
        config: &channel.config,
        credentials: &credentials,
    };
    let result = match channel_provider(channel.kind.as_str())?
        .test(&ctx, &context)
        .await
    {
        Ok(detail) => {
            store::record_channel_check(&pool, &channel, None).await?;
            ChannelTestResult {
                ok: true,
                detail: Some(detail),
                error_code: None,
                message: None,
            }
        }
        Err(error) => {
            store::record_channel_check(&pool, &channel, Some(&error.to_string())).await?;
            ChannelTestResult {
                ok: false,
                detail: None,
                error_code: Some(error.code.to_owned()),
                message: Some(error.message),
            }
        }
    };
    Ok(Json(Data { data: result }))
}

#[utoipa::path(get,path="/messaging/providers",tag=TAG,operation_id="messaging_providers",security(("session_cookie"=[])),responses((status=200,description="Enabled channel provider definitions with localized labels and configuration metadata; credentials are never included",body=Data<Vec<aphel_sdk::messaging::ChannelDefinition>>)))]
async fn list_providers(
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<Data<Vec<aphel_sdk::messaging::ChannelDefinition>>>, ApiError> {
    auth.scope.require(&MANAGE_CHANNELS)?;
    let providers = state
        .app
        .contributions::<dyn aphel_sdk::messaging::MessagingChannel>()
        .into_iter()
        .map(|provider| provider.definition())
        .collect();
    Ok(Json(Data::new(providers)))
}
