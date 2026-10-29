pub(crate) mod actor;
pub(crate) mod channel_routes;
pub(crate) mod channels;
pub(crate) mod errors;
pub(crate) mod inbound;
pub(crate) mod inbox_routes;
pub(crate) mod jobs;
pub(crate) mod model;
pub(crate) mod plugin_bridge;
pub(crate) mod public;
pub(crate) mod sending;
pub(crate) mod store;
pub(crate) mod text;
pub(crate) mod triggers;

use async_trait::async_trait;
use serde_json::{Value, json};
use uuid::Uuid;

use aphel_kernel::Ctx;
use aphel_kernel::database::transaction::MySqlTx;
use aphel_kernel::domain::access::{AccessScope, Permission, PermissionDefinition};
use aphel_kernel::errors::{AppError, CodedKind};
use aphel_kernel::modules::builder::APHEL_VERSION;
use aphel_kernel::modules::{
    AppBuilder, Manifest, Migration, Module, ModuleCtx, SettingField, SettingKind,
    module_secret_key, module_settings,
};
use aphel_kernel::records::RecordCtx;

pub(crate) const KEY: &str = "messaging";
pub(crate) const TAG: &str = "messaging";
pub(crate) const VIEW: Permission = Permission::from_static("messaging.view");
pub(crate) const REPLY: Permission = Permission::from_static("messaging.reply");
pub(crate) const MANAGE_CHANNELS: Permission = Permission::from_static("messaging.manage_channels");

pub(crate) const MESSAGE_RECEIVED: &str = "message.received";
pub(crate) const MESSAGE_SENT: &str = "message.sent";
pub(crate) const MESSAGE_FAILED: &str = "message.failed";
pub(crate) const CONVERSATION_ASSIGNED: &str = "conversation.assigned";
pub(crate) const CONVERSATION_STATUS_CHANGED: &str = "conversation.status_changed";
pub(crate) const CONVERSATION_LINKED: &str = "conversation.linked";
pub(crate) const CHANNEL_CREATED: &str = "messaging_channel.created";
pub(crate) const CHANNEL_UPDATED: &str = "messaging_channel.updated";
pub(crate) const CHANNEL_DELETED: &str = "messaging_channel.deleted";

pub(crate) const SETTING_CREATE_CONTACTS: &str = "create_unknown_contacts";
pub(crate) const SETTING_ACTING_USER: &str = "acting_user_email";

pub(crate) const PUBLIC_WHATSAPP_PATH: &str = "/api/v1/public/messaging/whatsapp";

pub(crate) fn job_kind(kind: &str) -> String {
    format!("{KEY}.{kind}")
}

pub(crate) fn job_unique_key(key: &str) -> String {
    format!("{KEY}.{key}")
}

pub(crate) async fn settings(ctx: &Ctx<'_>) -> Result<Value, AppError> {
    module_settings(ctx.pool(), KEY).await
}

pub(crate) async fn secret(ctx: &Ctx<'_>, name: &str) -> Result<Option<String>, AppError> {
    Ok(ctx
        .secrets()?
        .reveal(&AccessScope::system(), &module_secret_key(KEY, name))
        .await?
        .map(|secret| secret.expose().to_owned()))
}

pub(crate) fn user_uuid(scope: &AccessScope) -> Result<Uuid, AppError> {
    Uuid::parse_str(scope.user_id().as_str()).map_err(|error| AppError::Internal(error.to_string()))
}

pub(crate) fn can(scope: &AccessScope, permission: &Permission) -> bool {
    scope.scope_for(permission).is_some()
}

pub(crate) fn error_code(error: &AppError) -> &'static str {
    match error {
        AppError::Coded { code, .. }
        | AppError::CodedFields { code, .. }
        | AppError::CodedDetails { code, .. } => code,
        AppError::Configuration(_) => "configuration_error",
        AppError::Conflict(_) => "conflict",
        AppError::Forbidden(_) => "forbidden",
        AppError::NotFound(_) => "not_found",
        AppError::RateLimited(_) => "rate_limited",
        AppError::Unauthorized(_) => "unauthorized",
        AppError::Validation(_) | AppError::InvalidFields { .. } => "validation_error",
        AppError::Persistence(_) | AppError::Internal(_) => "internal_error",
    }
}

fn is_missing(error: &AppError) -> bool {
    matches!(
        error,
        AppError::NotFound(_)
            | AppError::Forbidden(_)
            | AppError::Coded {
                kind: CodedKind::NotFound | CodedKind::Forbidden,
                ..
            }
    )
}

pub(crate) async fn load_record(
    ctx: &Ctx<'_>,
    scope: &AccessScope,
    record_type: &str,
    record_id: Uuid,
) -> Result<Option<Value>, AppError> {
    let Ok(handler) = ctx.registry().handler(record_type) else {
        return Ok(None);
    };
    let state = ctx.state();
    match handler
        .load(&RecordCtx::new(scope, &state), record_id)
        .await
    {
        Ok(value) => Ok(value),
        Err(error) if is_missing(&error) => Ok(None),
        Err(error) => Err(error),
    }
}

pub struct MessagingModule;

#[async_trait]
impl Module for MessagingModule {
    fn manifest(&self) -> Manifest {
        Manifest::new(KEY, "Messaging channels", APHEL_VERSION)
            .description(
                "Shared inbox for WhatsApp Cloud API and email (IMAP/SMTP), with conversations on the contact timeline",
            )
            .depends_on(contacts::KEY, "*")
            .optional(deals::KEY)
            .optional(activities::KEY)
            .setting(
                SettingField::new(
                    SETTING_CREATE_CONTACTS,
                    "Criar contato para remetente desconhecido",
                    SettingKind::Boolean,
                )
                .help("Quando ligado, mensagens de telefones ou e-mails sem contato criam o contato automaticamente; desligado, a conversa fica sem vínculo até alguém vincular")
                .default_value(json!(true)),
            )
            .setting(
                SettingField::new(
                    SETTING_ACTING_USER,
                    "Usuário de execução",
                    SettingKind::Email,
                )
                .help("E-mail do usuário em nome de quem os webhooks e a leitura de e-mails buscam e criam contatos e registram mensagens, quando o canal não define um. Vazio: quem criou o canal"),
            )
            .migration(Migration::new(
                1,
                "create messaging channels, conversations and messages",
                include_str!("../migrations/0001_messaging.sql"),
            ))
    }

    fn register(&self, app: &mut AppBuilder) -> Result<(), AppError> {
        app.openapi_tag(
            TAG,
            "Shared inbox for WhatsApp Cloud API and email (IMAP/SMTP)",
        )
        .routes(|_| channel_routes::router().merge(inbox_routes::router()))
        .public_routes(|_| public::router())
        .triggers(triggers::messaging_triggers())
        .worker(jobs::messaging_worker);
        app.permission(PermissionDefinition::administrative(VIEW));
        app.permission(PermissionDefinition::administrative(REPLY));
        app.permission(PermissionDefinition::administrative(MANAGE_CHANNELS));
        for operation in plugin_bridge::operations() {
            app.contribute::<aphel_kernel::modules::PluginOperation>(operation);
        }
        Ok(())
    }

    async fn export_data(&self, ctx: &mut ModuleCtx<'_>) -> Result<Value, AppError> {
        let channels = store::list_channels(ctx.pool).await?;
        Ok(json!({
            "credentials": channels
                .iter()
                .map(|channel| json!({
                    "channel_id": channel.id,
                    "secrets": channel.kind.handler().secret_names(),
                    "note": "credentials are encrypted and never exported",
                }))
                .collect::<Vec<Value>>()
        }))
    }

    async fn purge_data(&self, ctx: &mut ModuleCtx<'_>, tx: &mut MySqlTx) -> Result<(), AppError> {
        let channels = store::list_channels(ctx.pool).await?;
        let state = aphel_kernel::state::runtime_state()?;
        let ctx = Ctx::from_state(&state, ctx.scope).await?;
        for channel in channels {
            channels::remove_credentials(&ctx, tx, &channel).await?;
        }
        Ok(())
    }
}
