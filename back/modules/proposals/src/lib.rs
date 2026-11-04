use aphel_contract_automation::AutomationBuilder;
mod automation;
mod config;
mod errors;
mod format;
mod http;
mod markup;
mod pdf;
mod plugin_bridge;
mod records;
mod render;
mod service;
mod store;
mod variables;

use std::sync::Arc;

use async_trait::async_trait;

use ::config_packages::application::ConfigSection;
use aphel_kernel::application::files::{FileReferenceColumn, FileReferenceSource};
use aphel_kernel::application::public_events::event;
use aphel_kernel::context::Ctx;
use aphel_kernel::database::transaction::MySqlTx;
use aphel_kernel::domain::access::PermissionDefinition;
use aphel_kernel::errors::AppError;
use aphel_kernel::modules::{AppBuilder, Manifest, Migration, Module, ModuleCtx};
use aphel_kernel::state::runtime_state;

pub(crate) use service::{
    GENERATE, MANAGE, PROPOSAL_GENERATED, TEMPLATE_CREATED, TEMPLATE_DELETED, TEMPLATE_UPDATED,
};

pub(crate) const KEY: &str = "proposals";
pub(crate) const VERSION: &str = "0.1.0";

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct ProposalFileReferences;

impl FileReferenceSource for ProposalFileReferences {
    fn file_columns(&self) -> Vec<FileReferenceColumn> {
        vec![FileReferenceColumn::new("proposals_documents", "file_id")]
    }
}

pub struct ProposalsModule;

#[async_trait]
impl Module for ProposalsModule {
    fn manifest(&self) -> Manifest {
        let mut manifest = Manifest::new(KEY, "Proposals", VERSION)
            .depends_on(aphel_sdk::documents::KEY, "*")
            .description("Document templates and PDF proposals generated from deals")
            .requires_aphel("0.1.0")
            .migration(Migration::new(
                1,
                "create proposal templates, counters and documents",
                include_str!("../migrations/0001_templates_documents.sql"),
            ));
        manifest.base = manifest
            .base
            .permission(service::MANAGE_PERMISSION, "Gerenciar modelos de proposta")
            .permission(service::GENERATE_PERMISSION, "Gerar e ver propostas")
            .event(
                PROPOSAL_GENERATED,
                "Uma proposta em PDF foi gerada e anexada à oportunidade",
            )
            .event(TEMPLATE_CREATED, "Um modelo de proposta foi criado")
            .event(TEMPLATE_UPDATED, "Um modelo de proposta foi alterado")
            .event(TEMPLATE_DELETED, "Um modelo de proposta foi excluído");
        manifest
    }

    fn register(&self, app: &mut AppBuilder) -> Result<(), AppError> {
        app.openapi_tag(
            http::TAG,
            "Proposal templates and PDF proposals generated from deals",
        )
        .routes(|_| http::router())
        .action(Arc::new(automation::GenerateProposalAction))
        .trigger(Arc::new(automation::ProposalGeneratedTrigger))
        .contribute::<dyn ConfigSection<MySqlTx>>(Arc::new(config::TemplatesSection))
        .contribute::<dyn FileReferenceSource>(Arc::new(ProposalFileReferences));
        app.permission(PermissionDefinition::administrative(MANAGE));
        app.public_event(event(
            service::PROPOSAL_GENERATED,
            aphel_kernel::application::public_events::PublicResource::new(
                "deals.deal",
                "DealDetailResponse",
            ),
            "A proposal PDF was generated and attached to the deal",
        ));
        app.permission(PermissionDefinition::administrative(GENERATE));
        for operation in plugin_bridge::operations() {
            app.contribute::<aphel_kernel::modules::PluginOperation>(operation);
        }
        Ok(())
    }

    async fn on_enable(&self, ctx: &mut ModuleCtx<'_>) -> Result<(), AppError> {
        let state = runtime_state()?;
        let module_ctx = Ctx::from_state(&state, ctx.scope).await?;
        service::seed_starter_template(&module_ctx).await
    }
}
