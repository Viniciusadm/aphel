pub(crate) mod domain;
pub(crate) mod http;
pub(crate) mod notifications;
pub(crate) mod runtime;
pub(crate) mod section;
pub(crate) mod service;
pub(crate) mod store;
pub(crate) mod widget;

use aphel_contract_crm::CrmRecordTypes;
use std::sync::Arc;

use aphel_kernel::modules::Migration;

use crate::domain::RENEWAL_OF_DEAL_FIELD;
use aphel_kernel::application::fields::builtin::record_field;
use aphel_kernel::application::public_events::{PublicEventDefinition, event};
use aphel_kernel::application::shared::RecordType;
use aphel_kernel::errors::AppError;
use aphel_kernel::modules::builder::APHEL_VERSION;
use aphel_kernel::modules::{AppBuilder, Manifest, Module};

pub(crate) const KEY: &str = "renewals";

const PUBLIC_EVENTS: &[PublicEventDefinition] = &[event(
    "deal.renewal_created",
    deals::DEAL_RESOURCE,
    "A renewal deal was created for a won deal whose contract is ending (the body is the renewal; `renewal_of_deal_id` points to the original)",
)];

pub struct RenewalsModule;

impl Module for RenewalsModule {
    fn manifest(&self) -> Manifest {
        Manifest::new(KEY, "Renewals", APHEL_VERSION)
            .description(
                "Automatic renewal of won deals whose contract is ending, configured per pipeline",
            )
            .enabled_by_default()
            .depends_on(deals::KEY, "*")
            .depends_on(pipelines::KEY, "*")
            .optional(deal_items::KEY)
            .owned_tables(&["deal_renewals", "pipeline_renewal_settings"])
            .migration(Migration::new(
                1,
                "create renewals",
                include_str!("../migrations/0001_create_renewals.sql"),
            ))
    }

    fn register(&self, app: &mut AppBuilder) -> Result<(), AppError> {
        app.contribute(std::sync::Arc::new(
            ::config_packages::infrastructure::ConfigSectionFactory::new(80, |pool, registry| {
                let _ = registry;
                std::sync::Arc::new(crate::section::pipeline_renewals_section(pool))
            }),
        ));
        let renewal_of = record_field(
            RENEWAL_OF_DEAL_FIELD,
            crate::labels("Renewal of"),
            RecordType::DEAL.as_str(),
        )?;
        app.extend(RecordType::DEAL.as_str(), |deal| {
            deal.system_field(renewal_of.clone());
        })
        .routes(|_| http::router())
        .job(|env| {
            Arc::new(runtime::RenewalScanHandler::new(
                env.pool.clone(),
                env.record_types.clone(),
            ))
        })
        .schedule(runtime::renewals_scan_schedule())
        .contribute::<dyn ::dashboard::engine::WidgetProvider<::dashboard::repository::MySqlDashboardPorts>>(Arc::new(widget::RenewalsDueWidget))
        .contribute(std::sync::Arc::new(notifications::RENEWAL_NOTIFICATION_KIND))
        .consumer(|env| notifications::RenewalNotificationConsumer::shared(env.pool.clone()));
        for definition in PUBLIC_EVENTS {
            app.public_event(*definition);
        }
        Ok(())
    }
}

pub(crate) fn labels(key: &str) -> aphel_kernel::application::fields::LocalizedLabels {
    aphel_kernel::application::fields::LocalizedLabels::from_catalog(
        include_str!("../locales/labels.json"),
        key,
    )
}
