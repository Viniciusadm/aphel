mod config;
mod context;
mod exposed_operations;
mod features;
mod http;
mod model;
mod provider;
mod query;
mod records;
mod service;
mod store;

use async_trait::async_trait;
use serde_json::{Value, json};

use crate::config::{
    DEFAULT_DAILY_LIMIT, DEFAULT_MODEL, DEFAULT_USER_HOURLY_LIMIT, Feature, SETTING_API_KEY,
    SETTING_BASE_URL, SETTING_DAILY_LIMIT, SETTING_MODEL, SETTING_PROVIDER,
    SETTING_SEND_PERSONAL_DATA, SETTING_USER_HOURLY_LIMIT, has_valid_scheme,
};
use crate::provider::{AnthropicProvider, OpenAiCompatibleProvider};
use aphel_kernel::database::transaction::MySqlTx;
use aphel_kernel::domain::access::{Permission, PermissionDefinition};
use aphel_kernel::errors::{AppError, FieldViolation};
use aphel_kernel::modules::{
    AppBuilder, Manifest, Migration, Module, ModuleCtx, SettingField, SettingKind, SettingOption,
};

pub(crate) const KEY: &str = "ai";
pub(crate) const VERSION: &str = "0.1.0";
pub(crate) const TAG: &str = "ai";
pub(crate) const USE_PERMISSION: &str = "ai.use";
pub(crate) const VIEW_USAGE_PERMISSION: &str = "ai.view_usage";
pub(crate) const USE: Permission = Permission::from_static(USE_PERMISSION);
pub(crate) const VIEW_USAGE: Permission = Permission::from_static(VIEW_USAGE_PERMISSION);

pub struct AiModule;

fn feature_setting(feature: Feature, label: &'static str) -> SettingField {
    SettingField::new(feature.setting_key(), label, SettingKind::Boolean).default_value(json!(true))
}

#[async_trait]
impl Module for AiModule {
    fn manifest(&self) -> Manifest {
        let mut manifest = Manifest::new(KEY, "AI assistant", VERSION)
            .description(
                "Summaries, next-step suggestions, a writing assistant and natural language queries with a configurable AI provider",
            )
            .requires_aphel("0.1.0")
            .optional("deals")
            .optional("tasks");
        manifest.base = manifest
            .base
            .permission(USE_PERMISSION, "Usar o assistente de IA")
            .permission(
                VIEW_USAGE_PERMISSION,
                "Ver o uso e o registro de chamadas da IA",
            );
        manifest
            .setting(
                SettingField::new(
                    SETTING_PROVIDER,
                    "Provedor",
                    SettingKind::Select {
                        options: vec![
                            SettingOption {
                                value: AnthropicProvider::NAME,
                                label: "Anthropic (Claude)",
                            },
                            SettingOption {
                                value: OpenAiCompatibleProvider::NAME,
                                label: "Compatível com OpenAI (OpenAI, Ollama, LM Studio)",
                            },
                        ],
                    },
                )
                .required()
                .default_value(json!(AnthropicProvider::NAME))
                .help("Serviço que recebe os dados enviados pela IA"),
            )
            .setting(
                SettingField::new(SETTING_BASE_URL, "URL do provedor", SettingKind::Url).help(
                    "Vazio usa https://api.anthropic.com ou https://api.openai.com/v1. Para Ollama local: http://localhost:11434/v1",
                ),
            )
            .setting(
                SettingField::new(SETTING_MODEL, "Modelo", SettingKind::Text)
                    .required()
                    .default_value(json!(DEFAULT_MODEL))
                    .help("Ex.: claude-sonnet-5-5, claude-opus-5-5, claude-haiku-5-5, gpt-4o-mini, llama3.1"),
            )
            .setting(
                SettingField::new(SETTING_API_KEY, "Chave de API", SettingKind::Secret)
                    .help("Opcional para servidores locais compatíveis com OpenAI"),
            )
            .setting(
                SettingField::new(
                    SETTING_DAILY_LIMIT,
                    "Limite diário de chamadas",
                    SettingKind::Number {
                        min: Some(1),
                        max: Some(100_000),
                    },
                )
                .required()
                .default_value(json!(DEFAULT_DAILY_LIMIT))
                .help("Chamadas ao provedor por dia"),
            )
            .setting(
                SettingField::new(
                    SETTING_USER_HOURLY_LIMIT,
                    "Limite por usuário por hora",
                    SettingKind::Number {
                        min: Some(1),
                        max: Some(10_000),
                    },
                )
                .required()
                .default_value(json!(DEFAULT_USER_HOURLY_LIMIT)),
            )
            .setting(
                SettingField::new(
                    SETTING_SEND_PERSONAL_DATA,
                    "Enviar dados pessoais",
                    SettingKind::Boolean,
                )
                .default_value(json!(true))
                .help("Desligado, campos marcados como dados pessoais (nome, e-mail, telefone…) e nomes de pessoas não são enviados. Campos sensíveis nunca são enviados"),
            )
            .setting(feature_setting(Feature::Summary, "Resumo do registro"))
            .setting(feature_setting(
                Feature::NextActions,
                "Sugestões de próximas ações",
            ))
            .setting(feature_setting(Feature::Compose, "Assistente de redação"))
            .setting(feature_setting(
                Feature::Query,
                "Consulta em linguagem natural",
            ))
            .setting(feature_setting(
                Feature::Plugin,
                "Uso por plugins",
            ))
            .migration(Migration::new(
                1,
                "create ai call log and daily usage",
                include_str!("../migrations/0001_ai.sql"),
            ))
    }

    fn register(&self, app: &mut AppBuilder) -> Result<(), AppError> {
        app.openapi_tag(
            TAG,
            "AI assistant: record summaries, next actions, writing help and natural language queries",
        )
        .routes(|_| http::router());
        app.permission(PermissionDefinition::administrative(USE));
        app.permission(PermissionDefinition::administrative(VIEW_USAGE));
        exposed_operations::register(app);
        Ok(())
    }

    fn validate_settings(&self, settings: &Value) -> Vec<FieldViolation> {
        let mut violations = Vec::new();
        if let Some(url) = settings
            .get(SETTING_BASE_URL)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|url| !url.is_empty())
            && !has_valid_scheme(url)
        {
            violations.push(FieldViolation::new(
                SETTING_BASE_URL,
                "must start with http:// or https://",
            ));
        }
        if settings
            .get(SETTING_MODEL)
            .and_then(Value::as_str)
            .is_some_and(|model| model.trim().chars().count() > 120)
        {
            violations.push(FieldViolation::new(
                SETTING_MODEL,
                "must have at most 120 characters",
            ));
        }
        violations
    }

    async fn export_data(&self, ctx: &mut ModuleCtx<'_>) -> Result<Value, AppError> {
        let totals = store::totals_since(ctx.pool, store::thirty_days_ago()).await?;
        Ok(json!({
            "note": "ai_calls holds only call metadata; prompts and answers are never stored",
            "last_30_days": {
                "calls": totals.calls,
                "errors": totals.errors,
                "input_tokens": totals.input_tokens,
                "output_tokens": totals.output_tokens,
            }
        }))
    }

    async fn purge_data(&self, _ctx: &mut ModuleCtx<'_>, tx: &mut MySqlTx) -> Result<(), AppError> {
        store::purge(tx).await
    }
}
