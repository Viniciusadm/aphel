use std::sync::Arc;

use sqlx::MySqlPool;

use crate::application::{ValidationRuleGuard, ValidationRulePorts, ValidationRulesSection};
use crate::infrastructure::{MySqlProposedFieldValues, MySqlValidationRuleRepository};
use ::formulas::infrastructure::MySqlFormulaSource;
use aphel_kernel::application::fields::RecordTypeRegistry;
use aphel_kernel::infrastructure::auth::SystemClock;
use aphel_kernel::infrastructure::fields::MySqlFieldDefinitionRepository;
use aphel_kernel::infrastructure::platform::{SharedAuditLog, SharedEventPublisher};
use aphel_kernel::infrastructure::transactions::MySqlTransactionManager;

pub type MySqlValidationRuleGuard = ValidationRuleGuard<
    MySqlValidationRuleRepository,
    MySqlFieldDefinitionRepository,
    MySqlFormulaSource,
    MySqlProposedFieldValues,
>;

pub fn validation_rule_guard(
    pool: &MySqlPool,
    registry: &Arc<RecordTypeRegistry>,
) -> MySqlValidationRuleGuard {
    ValidationRuleGuard::new(
        registry.clone(),
        MySqlValidationRuleRepository::new(pool.clone()),
        MySqlFieldDefinitionRepository::new(pool.clone()),
        MySqlFormulaSource::new(pool.clone()),
        MySqlProposedFieldValues,
    )
}

pub fn validation_rules_section(
    pool: &MySqlPool,
) -> ValidationRulesSection<MySqlValidationRuleRepository> {
    ValidationRulesSection::new(MySqlValidationRuleRepository::new(pool.clone()))
}

pub struct MySqlValidationRulePorts {
    registry: Arc<RecordTypeRegistry>,
    transactions: MySqlTransactionManager,
    rules: MySqlValidationRuleRepository,
    definitions: MySqlFieldDefinitionRepository,
    events: SharedEventPublisher,
    audit: SharedAuditLog,
}

impl MySqlValidationRulePorts {
    pub fn new(
        pool: MySqlPool,
        registry: Arc<RecordTypeRegistry>,
        events: SharedEventPublisher,
        audit: SharedAuditLog,
    ) -> Self {
        Self {
            registry,
            transactions: MySqlTransactionManager::new(pool.clone()),
            rules: MySqlValidationRuleRepository::new(pool.clone()),
            definitions: MySqlFieldDefinitionRepository::new(pool),
            events,
            audit,
        }
    }
}

impl ValidationRulePorts for MySqlValidationRulePorts {
    type Transactions = MySqlTransactionManager;
    type Rules = MySqlValidationRuleRepository;
    type Definitions = MySqlFieldDefinitionRepository;
    type Events = SharedEventPublisher;
    type Audit = SharedAuditLog;
    type Clock = SystemClock;

    fn registry(&self) -> &RecordTypeRegistry {
        &self.registry
    }

    fn transactions(&self) -> &Self::Transactions {
        &self.transactions
    }

    fn rules(&self) -> &Self::Rules {
        &self.rules
    }

    fn definitions(&self) -> &Self::Definitions {
        &self.definitions
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
