use std::sync::Arc;

use sqlx::MySqlPool;

use crate::application::sections::FieldsSection;
use crate::application::{
    ConfigPackagePorts, ConfigSection, ConfigSectionRegistry, UnavailableFields,
};
use crate::infrastructure::{EmbeddedTemplates, MySqlConfigInstallRepository};
use aphel_kernel::application::fields::RecordTypeRegistry;
use aphel_kernel::database::transaction::MySqlTx;
use aphel_kernel::infrastructure::auth::SystemClock;
use aphel_kernel::infrastructure::fields::{
    MySqlCustomFieldValueRepository, MySqlFieldDefinitionRepository,
};
use aphel_kernel::infrastructure::jobs::MySqlJobQueue;
use aphel_kernel::infrastructure::platform::{SharedAuditLog, SharedEventPublisher};
use aphel_kernel::infrastructure::transactions::MySqlTransactionManager;

pub use aphel_sdk::configuration::{ConfigSectionBuild, ConfigSectionFactory};

pub fn builtin_sections(
    pool: &MySqlPool,
    registry: Arc<RecordTypeRegistry>,
) -> ConfigSectionRegistry<MySqlTx> {
    let definitions = || MySqlFieldDefinitionRepository::new(pool.clone());
    let mut sections: Vec<(u16, Arc<dyn ConfigSection<MySqlTx>>)> = vec![(
        20,
        Arc::new(FieldsSection::new(
            registry.clone(),
            definitions(),
            MySqlCustomFieldValueRepository::new(pool.clone()),
            MySqlJobQueue::new(pool.clone()),
        )),
    )];
    let registry_types = registry.clone();
    let app = aphel_kernel::modules::app();
    let mut unavailable = Vec::new();
    for (module, factory) in app.all_contributions::<ConfigSectionFactory>() {
        let section = (factory.build)(pool, &registry);
        if !app.is_enabled(module) {
            unavailable.push(section.key());
        }
        sections.push((factory.order, section));
    }
    sections.sort_by_key(|(order, _)| *order);
    let mut registry = ConfigSectionRegistry::new()
        .with_all(sections.into_iter().map(|(_, section)| section).collect())
        .with_all(app.contributions::<dyn ConfigSection<MySqlTx>>());
    for key in unavailable {
        registry.mark_unavailable(key);
    }
    let mut fields = UnavailableFields::default();
    for definition in registry_types.definitions() {
        for field in definition
            .system_fields
            .iter()
            .filter(|field| !field.is_available())
        {
            fields.insert(definition.record_type.as_str(), field.key.as_str());
        }
    }
    registry.with_unavailable_fields(fields)
}

pub struct MySqlConfigPackagePorts {
    sections: ConfigSectionRegistry<MySqlTx>,
    transactions: MySqlTransactionManager,
    installs: MySqlConfigInstallRepository,
    events: SharedEventPublisher,
    audit: SharedAuditLog,
}

impl MySqlConfigPackagePorts {
    pub fn new(
        pool: MySqlPool,
        registry: Arc<RecordTypeRegistry>,
        events: SharedEventPublisher,
        audit: SharedAuditLog,
    ) -> Self {
        Self {
            sections: builtin_sections(&pool, registry),
            transactions: MySqlTransactionManager::new(pool.clone()),
            installs: MySqlConfigInstallRepository::new(pool),
            events,
            audit,
        }
    }
}

impl ConfigPackagePorts for MySqlConfigPackagePorts {
    type Transactions = MySqlTransactionManager;
    type Installs = MySqlConfigInstallRepository;
    type Templates = EmbeddedTemplates;
    type Events = SharedEventPublisher;
    type Audit = SharedAuditLog;
    type Clock = SystemClock;

    fn sections(&self) -> &ConfigSectionRegistry<MySqlTx> {
        &self.sections
    }

    fn transactions(&self) -> &Self::Transactions {
        &self.transactions
    }

    fn installs(&self) -> &Self::Installs {
        &self.installs
    }

    fn templates(&self) -> &Self::Templates {
        &EmbeddedTemplates
    }

    fn events(&self) -> &Self::Events {
        &self.events
    }

    fn audit(&self) -> &Self::Audit {
        &self.audit
    }

    fn clock(&self) -> &Self::Clock {
        &SystemClock
    }
}
