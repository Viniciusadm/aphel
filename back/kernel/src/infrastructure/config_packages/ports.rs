use std::sync::Arc;

use sqlx::MySqlPool;

use crate::application::config_packages::sections::{
    CustomObjectsSection, FieldsSection, LayoutsSection, ViewsSection,
};
use crate::application::config_packages::{
    ConfigPackagePorts, ConfigSection, ConfigSectionRegistry, UnavailableFields,
};
use crate::application::fields::RecordTypeRegistry;
use crate::database::transaction::MySqlTx;
use crate::infrastructure::auth::SystemClock;
use crate::infrastructure::config_packages::{
    EmbeddedTemplates, MySqlConfigInstallRepository, MySqlConfigKeyRepository,
};
use crate::infrastructure::custom_objects::{
    MySqlCustomObjectRepository, MySqlRelationshipRepository,
};
use crate::infrastructure::fields::{
    MySqlCustomFieldValueRepository, MySqlFieldDefinitionRepository,
};
use crate::infrastructure::jobs::MySqlJobQueue;
use crate::infrastructure::layouts::MySqlFormLayoutRepository;
use crate::infrastructure::platform::{SharedAuditLog, SharedEventPublisher};
use crate::infrastructure::transactions::MySqlTransactionManager;
use crate::infrastructure::views::MySqlSavedViewRepository;

pub type ConfigSectionBuild = Arc<
    dyn Fn(&MySqlPool, &Arc<RecordTypeRegistry>) -> Arc<dyn ConfigSection<MySqlTx>> + Send + Sync,
>;

pub struct ConfigSectionFactory {
    pub(crate) order: u16,
    pub(crate) build: ConfigSectionBuild,
}

impl ConfigSectionFactory {
    pub fn new<F>(order: u16, build: F) -> Self
    where
        F: Fn(&MySqlPool, &Arc<RecordTypeRegistry>) -> Arc<dyn ConfigSection<MySqlTx>>
            + Send
            + Sync
            + 'static,
    {
        Self {
            order,
            build: Arc::new(build),
        }
    }
}

pub fn builtin_sections(
    pool: &MySqlPool,
    registry: Arc<RecordTypeRegistry>,
) -> ConfigSectionRegistry<MySqlTx> {
    let keys = || MySqlConfigKeyRepository::new(pool.clone());
    let definitions = || MySqlFieldDefinitionRepository::new(pool.clone());
    let mut sections: Vec<(u16, Arc<dyn ConfigSection<MySqlTx>>)> = vec![
        (
            10,
            Arc::new(CustomObjectsSection::new(
                registry.clone(),
                MySqlCustomObjectRepository::new(pool.clone()),
                MySqlRelationshipRepository::new(pool.clone()),
                definitions(),
            )),
        ),
        (
            20,
            Arc::new(FieldsSection::new(
                registry.clone(),
                definitions(),
                MySqlCustomFieldValueRepository::new(pool.clone()),
                MySqlJobQueue::new(pool.clone()),
            )),
        ),
        (
            110,
            Arc::new(LayoutsSection::new(
                registry.clone(),
                MySqlFormLayoutRepository::new(pool.clone()),
                definitions(),
            )),
        ),
        (
            120,
            Arc::new(ViewsSection::new(
                registry.clone(),
                MySqlSavedViewRepository::new(pool.clone()),
                keys(),
                definitions(),
            )),
        ),
        (
            190,
            Arc::new(
                crate::infrastructure::fields::validation_rules::validation_rules_section(pool),
            ),
        ),
    ];
    let registry_types = registry.clone();
    let app = crate::modules::app();
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
