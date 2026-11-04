use aphel_contract_automation::AutomationBuilder;
mod automation;
mod cnpj;
mod http;
mod plugin_bridge;
mod service;
mod sources;
mod store;
mod support;

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{Value, json};

use aphel_kernel::application::public_events::{PublicEventDefinition, event};
use aphel_kernel::database::transaction::MySqlTx;
use aphel_kernel::domain::access::{Permission, PermissionDefinition};
use aphel_kernel::errors::AppError;
use aphel_kernel::modules::{
    AppBuilder, Manifest, Migration, Module, ModuleCtx, SettingField, SettingKind, SettingOption,
};

pub(crate) const KEY: &str = "enrichment";
pub(crate) const VERSION: &str = "0.1.0";
pub(crate) const USE_PERMISSION: &str = "enrichment.use";
pub(crate) const USE: Permission = Permission::from_static(USE_PERMISSION);
pub(crate) const COMPANY_ENRICHED: &str = "company.enriched";

const PUBLIC_EVENTS: &[PublicEventDefinition] = &[event(
    COMPANY_ENRICHED,
    aphel_sdk::organizations::RESOURCE,
    "Fields of the company were filled from a public CNPJ lookup",
)];
pub(crate) const SOURCE_SETTING: &str = "source";
pub(crate) const CNPJ_FIELD_SETTING: &str = "cnpj_field";
pub(crate) const AUTOMATIC_SETTING: &str = "automatic";

pub(crate) const COMPANY: &str = "organizations.organization";

pub(crate) struct DataItem {
    pub(crate) key: &'static str,
    pub(crate) setting: &'static str,
    pub(crate) label: &'static str,
    pub(crate) default_field: Option<&'static str>,
    pub(crate) field_types: &'static [&'static str],
}

const TEXT: &[&str] = &["text"];
const TEXTS: &[&str] = &["text", "long_text"];

pub(crate) const DATA_ITEMS: &[DataItem] = &[
    DataItem {
        key: "legal_name",
        setting: "field_legal_name",
        label: "Razão social",
        default_field: None,
        field_types: TEXT,
    },
    DataItem {
        key: "trade_name",
        setting: "field_trade_name",
        label: "Nome fantasia",
        default_field: None,
        field_types: TEXT,
    },
    DataItem {
        key: "activity",
        setting: "field_activity",
        label: "Atividade principal (CNAE)",
        default_field: Some("industry"),
        field_types: TEXTS,
    },
    DataItem {
        key: "size",
        setting: "field_size",
        label: "Porte",
        default_field: None,
        field_types: TEXT,
    },
    DataItem {
        key: "registration_status",
        setting: "field_registration_status",
        label: "Situação cadastral",
        default_field: None,
        field_types: TEXT,
    },
    DataItem {
        key: "opened_on",
        setting: "field_opened_on",
        label: "Data de abertura",
        default_field: None,
        field_types: &["date", "text"],
    },
    DataItem {
        key: "address",
        setting: "field_address",
        label: "Endereço completo",
        default_field: None,
        field_types: TEXTS,
    },
    DataItem {
        key: "address_street",
        setting: "field_address_street",
        label: "Logradouro",
        default_field: Some("address_street"),
        field_types: TEXT,
    },
    DataItem {
        key: "address_number",
        setting: "field_address_number",
        label: "Número",
        default_field: Some("address_number"),
        field_types: TEXT,
    },
    DataItem {
        key: "address_complement",
        setting: "field_address_complement",
        label: "Complemento",
        default_field: Some("address_complement"),
        field_types: TEXT,
    },
    DataItem {
        key: "address_district",
        setting: "field_address_district",
        label: "Bairro",
        default_field: Some("address_district"),
        field_types: TEXT,
    },
    DataItem {
        key: "address_city",
        setting: "field_address_city",
        label: "Cidade",
        default_field: Some("address_city"),
        field_types: TEXT,
    },
    DataItem {
        key: "address_state",
        setting: "field_address_state",
        label: "Estado (UF)",
        default_field: Some("address_state"),
        field_types: TEXT,
    },
    DataItem {
        key: "address_postal_code",
        setting: "field_address_postal_code",
        label: "CEP",
        default_field: Some("address_postal_code"),
        field_types: TEXT,
    },
    DataItem {
        key: "phone",
        setting: "field_phone",
        label: "Telefone",
        default_field: Some("phone"),
        field_types: &["phone", "text"],
    },
    DataItem {
        key: "email",
        setting: "field_email",
        label: "E-mail",
        default_field: Some("email"),
        field_types: &["email", "text"],
    },
];

pub(crate) fn data_item(key: &str) -> Option<&'static DataItem> {
    DATA_ITEMS.iter().find(|item| item.key == key)
}

pub struct EnrichmentModule;

impl EnrichmentModule {
    fn settings(mut manifest: Manifest) -> Manifest {
        manifest = manifest
            .setting(
                SettingField::new(
                    SOURCE_SETTING,
                    "Fonte de dados",
                    SettingKind::Select {
                        options: vec![
                            SettingOption {
                                value: sources::BRASILAPI_THEN_RECEITAWS,
                                label: "BrasilAPI, com ReceitaWS como reserva",
                            },
                            SettingOption {
                                value: sources::BRASILAPI,
                                label: "Somente BrasilAPI",
                            },
                            SettingOption {
                                value: sources::RECEITAWS,
                                label: "Somente ReceitaWS (limite de 3 consultas por minuto)",
                            },
                        ],
                    },
                )
                .required()
                .default_value(json!(sources::BRASILAPI_THEN_RECEITAWS)),
            )
            .setting(
                SettingField::new(
                    CNPJ_FIELD_SETTING,
                    "Campo do CNPJ",
                    SettingKind::Field {
                        record_type: COMPANY,
                        field_types: vec!["text"],
                    },
                )
                .help("Campo de texto da empresa que guarda o CNPJ; sem ele, o CNPJ é digitado na consulta"),
            )
            .setting(
                SettingField::new(
                    AUTOMATIC_SETTING,
                    "Permitir enriquecimento automático",
                    SettingKind::Boolean,
                )
                .default_value(json!(false))
                .help("Só com esta opção ligada a ação de automação \"Enriquecer empresa pelo CNPJ\" preenche campos sem clique"),
            );
        for item in DATA_ITEMS {
            let mut field = SettingField::new(
                item.setting,
                item.label,
                SettingKind::Field {
                    record_type: COMPANY,
                    field_types: item.field_types.to_vec(),
                },
            )
            .help("Campo da empresa que recebe este dado; vazio = não preencher");
            if let Some(default) = item.default_field {
                field = field.default_value(json!(default));
            }
            manifest = manifest.setting(field);
        }
        manifest
    }
}

#[async_trait]
impl Module for EnrichmentModule {
    fn manifest(&self) -> Manifest {
        let mut manifest = Manifest::new(KEY, "CNPJ enrichment", VERSION)
            .description(
                "Looks up public company data by CNPJ (BrasilAPI and ReceitaWS) and fills the chosen fields, recording the source of each value. Always triggered manually unless the automation is enabled",
            )
            .requires_aphel("0.1.0")
            .optional("deals")
            .optional("tasks");
        manifest.base = manifest
            .base
            .permission(
                USE_PERMISSION,
                "Consultar CNPJ e aplicar dados públicos às empresas",
            )
            .event(
                COMPANY_ENRICHED,
                "Campos de uma empresa foram preenchidos com dados públicos do CNPJ",
            );
        Self::settings(manifest).migration(Migration::new(
            1,
            "create enrichment lookups and field sources",
            include_str!("../migrations/0001_enrichment.sql"),
        ))
    }

    fn register(&self, app: &mut AppBuilder) -> Result<(), AppError> {
        app.openapi_tag(http::TAG, "Company enrichment by CNPJ from public sources")
            .routes(|_| http::router())
            .action(Arc::new(automation::EnrichCompanyAction))
            .trigger(Arc::new(automation::CompanyEnrichedTrigger));
        app.permission(PermissionDefinition::administrative(USE));
        for operation in plugin_bridge::operations() {
            app.contribute::<aphel_kernel::modules::PluginOperation>(operation);
        }
        for definition in PUBLIC_EVENTS {
            app.public_event(*definition);
        }
        Ok(())
    }

    async fn export_data(&self, ctx: &mut ModuleCtx<'_>) -> Result<Value, AppError> {
        let (lookups, sources) = store::counts(ctx.pool).await?;
        Ok(json!({ "lookups_total": lookups, "field_sources_total": sources }))
    }

    async fn purge_data(&self, _ctx: &mut ModuleCtx<'_>, tx: &mut MySqlTx) -> Result<(), AppError> {
        store::purge(tx).await
    }
}
