pub use aphel_contract::{
    Catalog as ContractCatalog, Contract, ContractDescriptor, EventKey, ExtensionPoint, HookKey,
    HookKind, KeyEntry, KeyKind, OperationKey, ReferenceKind, action, contract, event,
    extension_point, filter, operation, reference, veto,
};
pub use aphel_kernel::Ctx;
pub use aphel_kernel::errors::{AppError, CodedKind, FieldViolation};

pub mod module {
    pub use aphel_kernel::modules::builder::APHEL_VERSION;
    pub use aphel_kernel::modules::{
        AppBuilder, AutomationBinding, BoxFuture, CommandResult, Effect, Exposure, Hook, HookCtx,
        HookRef, Manifest, Migration, Module, ModuleCommand, OperationEntry, OperationFuture,
        OperationSpec, PluginOperation, ReferenceCheck, SettingField, SettingKind, TargetSelector,
        WorkerEnv, WorkerParts, connect_with_modules, module_secret_key, module_settings,
        operation_input, operation_output,
    };
}

pub mod access {
    pub use aphel_kernel::domain::access::{
        AccessScope, Permission, PermissionDefinition, RecordScopeFilter,
    };
}

pub mod shared {
    pub use aphel_kernel::application::shared::{
        Actor, DomainEvent, EventCausation, EventOrigin, JobContext, JobHandler, RecordRef,
        RecordType,
    };
}

pub mod catalog {
    pub use aphel_kernel::modules::unified_catalog::{
        CatalogContributor, CatalogDocument, HookEntry,
    };
}

pub mod automation {
    pub use aphel_contract_automation::application::actions::{ADD_TAG, REMOVE_TAG};
}

pub mod points {
    pub use aphel_kernel::application::files::GeneratedFileKind;
    pub use aphel_kernel::application::system::{FailureEntry, FailureSource};
    pub use aphel_kernel::domain::UserId;
    pub use aphel_kernel::modules::{
        FileReferenceSource, HealthSource, ReferenceProvider, lookup_exists,
    };
}

pub mod fields {
    pub use aphel_kernel::application::fields::{
        DeletePolicy, EntityDeclaration, EntityDefinition, EntityIndex, EntityStorage,
        EntityTraits, ExtensionColumn, FieldDef, FieldSpec, FieldTypeDefinition, FieldValidator,
        LocalizedLabels, RecordTypeRegistry, ReferenceTarget, SoftReference, SystemField,
        managed_table,
    };
    pub use aphel_kernel::application::query::FilterOperator;
    pub use aphel_kernel::domain::fields::{
        FieldConfig, FieldDefinition, FieldKey, FieldType, FieldValue, FieldValueError, RawValue,
        ValueColumn, validate_native_value,
    };
}

mod record_includes;
mod record_labels;

pub mod records {
    pub use crate::record_includes::{RecordIncludeSource, RecordIncludes, record_includes};
    pub use crate::record_labels::{
        VisibleRecordLabel, visible_record_label, visible_record_labels,
    };
    pub use aphel_kernel::records::{
        EntityWriteBehavior, GenericRecordHandler, PendingCommit, RecordCtx, RecordTypeHandler,
        StoredTextField, TextAssignment, TextComparison, archive_entity, existing_entity_ids,
        inspect_entity, lock_entity_version, mark_entity_deleted, matching_entity_text_ids,
        purge_entity, update_entity_text,
    };
    pub fn enabled(
        ctx: &aphel_kernel::Ctx<'_>,
        record_type: &aphel_kernel::application::shared::RecordType,
    ) -> bool {
        ctx.app()
            .record_contributions()
            .iter()
            .find(|contribution| contribution.registers.as_ref() == Some(record_type))
            .is_none_or(|contribution| ctx.app().is_enabled(contribution.module))
    }
}

pub mod party;

pub mod workflow;

pub mod calendar;

pub mod messaging;

pub mod assignment;

pub mod money;

pub mod documents;

pub mod line_items;

pub mod deals;

pub mod dashboard;

pub mod timeline;

pub mod privacy;

pub mod favorites;

pub mod followers;

pub mod imports;

mod generated_files;
pub mod files {
    pub use crate::generated_files::*;
    pub use aphel_kernel::infrastructure::storage::references::delete_unreferenced_files;
}

pub mod approvals;
pub mod email;
pub mod pricing;

pub mod tokens;

pub mod notifications;

pub mod configuration;

pub mod exports;

pub mod search;
pub mod text;

pub use aphel_contract_tagging as tagging;

pub mod views;

pub mod telephony;

pub mod objects;

pub mod conditions;

pub mod tasks;
