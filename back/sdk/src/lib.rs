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
        DomainEvent, JobContext, JobHandler, RecordRef, RecordType,
    };
}

pub mod catalog {
    pub use aphel_kernel::modules::unified_catalog::{
        CatalogContributor, CatalogDocument, HookEntry,
    };
}

pub mod automation {
    pub use aphel_kernel::application::automations::actions::{ADD_TAG, REMOVE_TAG};
}
