pub(crate) mod clicksign;
pub(crate) mod http;
pub(crate) mod plugin_bridge;
pub(crate) mod provider;
pub(crate) mod runtime;
pub(crate) mod service;
pub(crate) mod store;
pub(crate) mod triggers;
pub(crate) mod webhook;

use async_trait::async_trait;
use serde_json::{Value, json};
use uuid::Uuid;

use aphel_kernel::Ctx;
use aphel_kernel::application::access::{AccessRepository, build_access_scope};
use aphel_kernel::application::files::{FileReferenceColumn, FileReferenceSource};
use aphel_kernel::application::public_events::{PublicEventDefinition, event};
use aphel_kernel::database::transaction::MySqlTx;
use aphel_kernel::domain::UserId;
use aphel_kernel::domain::access::{AccessScope, Permission, PermissionDefinition};
use aphel_kernel::errors::AppError;
use aphel_kernel::infrastructure::repositories::mysql_access::MySqlAccessRepository;
use aphel_kernel::modules::builder::APHEL_VERSION;
use aphel_kernel::modules::{
    AppBuilder, Manifest, Migration, Module, ModuleCtx, SettingField, SettingKind, SettingOption,
    module_secret_key, module_settings,
};

pub(crate) const KEY: &str = "esign";
pub(crate) const TAG: &str = "esign";
pub(crate) const SEND: Permission = Permission::from_static("esign.send");
pub(crate) const VIEW: Permission = Permission::from_static("esign.view");
pub(crate) const SENT: &str = "esign.sent";
pub(crate) const SIGNED: &str = "esign.signed";
pub(crate) const REFUSED: &str = "esign.refused";
pub(crate) const CANCELLED: &str = "esign.cancelled";
pub(crate) const COMPLETED: &str = "esign.completed";
pub(crate) const EXPIRED: &str = "esign.expired";
pub(crate) const SIGNED_FILE_ATTACHED: &str = "esign.signed_file_attached";

const PUBLIC_EVENTS: &[PublicEventDefinition] = &[
    event(
        SENT,
        deals::DEAL_RESOURCE,
        "A document of the deal was sent for electronic signature",
    ),
    event(
        SIGNED,
        deals::DEAL_RESOURCE,
        "A signer signed a document of the deal",
    ),
    event(
        REFUSED,
        deals::DEAL_RESOURCE,
        "A signer refused a document of the deal",
    ),
    event(
        CANCELLED,
        deals::DEAL_RESOURCE,
        "A signature request of the deal was cancelled",
    ),
    event(
        COMPLETED,
        deals::DEAL_RESOURCE,
        "Every signer signed a document of the deal",
    ),
    event(
        EXPIRED,
        deals::DEAL_RESOURCE,
        "A signature request of the deal expired",
    ),
    event(
        SIGNED_FILE_ATTACHED,
        deals::DEAL_RESOURCE,
        "The signed file was attached to the deal",
    ),
];
pub(crate) const ENVIRONMENT_SETTING: &str = "environment";
pub(crate) const ACCESS_TOKEN_SETTING: &str = "access_token";
pub(crate) const WEBHOOK_SECRET_SETTING: &str = "webhook_secret";
pub(crate) const ATTACH_SIGNED_SETTING: &str = "attach_signed_document";
pub(crate) const PRODUCTION: &str = "production";
pub(crate) const SANDBOX: &str = "sandbox";

pub(crate) const DEAL: &str = "deal";

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

pub(crate) async fn scope_for_user(
    ctx: &Ctx<'_>,
    user_id: Uuid,
) -> Result<Option<AccessScope>, AppError> {
    let user = UserId::new(user_id.to_string())?;
    let Some(access) = MySqlAccessRepository::new(ctx.pool().clone())
        .load_access(&user)
        .await?
    else {
        return Ok(None);
    };
    let scope = build_access_scope(user, access);
    Ok(scope.module_enabled(KEY).then_some(scope))
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct EsignFileReferences;

impl FileReferenceSource for EsignFileReferences {
    fn file_columns(&self) -> Vec<FileReferenceColumn> {
        vec![
            FileReferenceColumn::new("esign_requests", "file_id"),
            FileReferenceColumn::new("esign_requests", "signed_file_id"),
        ]
    }
}

pub struct EsignModule;

#[async_trait]
impl Module for EsignModule {
    fn manifest(&self) -> Manifest {
        Manifest::new(KEY, "Electronic signature", APHEL_VERSION)
            .description(
                "Sends PDFs attached to deals for electronic signature on Clicksign and tracks their status through webhooks. It has no legal validity of its own: validity comes from the provider",
            )
            .optional(deals::KEY)
            .optional(activities::KEY)
            .setting(
                SettingField::new(
                    ENVIRONMENT_SETTING,
                    "Ambiente",
                    SettingKind::Select {
                        options: vec![
                            SettingOption {
                                value: SANDBOX,
                                label: "Sandbox (testes)",
                            },
                            SettingOption {
                                value: PRODUCTION,
                                label: "Produção",
                            },
                        ],
                    },
                )
                .required()
                .default_value(json!(SANDBOX)),
            )
            .setting(
                SettingField::new(
                    ACCESS_TOKEN_SETTING,
                    "Token de acesso da Clicksign",
                    SettingKind::Secret,
                )
                .required()
                .help("Gerado em Configurações → API na Clicksign; guardado cifrado e nunca exibido"),
            )
            .setting(
                SettingField::new(
                    WEBHOOK_SECRET_SETTING,
                    "Segredo HMAC do webhook",
                    SettingKind::Secret,
                )
                .required()
                .help("Chave HMAC SHA256 do webhook cadastrado na Clicksign (cabeçalho Content-Hmac)"),
            )
            .setting(
                SettingField::new(
                    ATTACH_SIGNED_SETTING,
                    "Anexar o PDF assinado à oportunidade",
                    SettingKind::Boolean,
                )
                .default_value(json!(true)),
            )
            .migration(Migration::new(
                1,
                "create esign requests, signers and webhook receipts",
                include_str!("../migrations/0001_esign.sql"),
            ))
    }

    fn register(&self, app: &mut AppBuilder) -> Result<(), AppError> {
        app.openapi_tag(
            TAG,
            "Sends PDFs attached to deals for electronic signature on Clicksign and tracks their status through webhooks",
        )
        .routes(|_| http::router())
        .public_routes(|_| webhook::router())
        .triggers(triggers::esign_triggers())
        .worker(runtime::esign_worker)
        .contribute::<dyn FileReferenceSource>(std::sync::Arc::new(EsignFileReferences));
        app.permission(PermissionDefinition::administrative(SEND));
        for definition in PUBLIC_EVENTS {
            app.public_event(*definition);
        }
        app.permission(PermissionDefinition::administrative(VIEW));
        for operation in plugin_bridge::operations() {
            app.contribute::<aphel_kernel::modules::PluginOperation>(operation);
        }
        Ok(())
    }

    async fn export_data(&self, ctx: &mut ModuleCtx<'_>) -> Result<Value, AppError> {
        let total = store::count_requests(ctx.pool).await?;
        Ok(json!({
            "provider": clicksign::PROVIDER_KEY,
            "requests_total": total,
            "secrets": "not exported",
        }))
    }

    async fn purge_data(&self, _ctx: &mut ModuleCtx<'_>, tx: &mut MySqlTx) -> Result<(), AppError> {
        store::purge(tx).await
    }
}
