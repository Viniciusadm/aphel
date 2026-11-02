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

pub mod records {
    pub use aphel_kernel::records::{
        EntityWriteBehavior, GenericRecordHandler, PendingCommit, RecordCtx, RecordTypeHandler,
    };
}
