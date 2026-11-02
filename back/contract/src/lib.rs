mod catalog;
mod key;
mod macros;

pub use catalog::{Catalog, CatalogError, ContractDescriptor, KeyEntry, KeyKind};
pub use key::{
    Contract, EventKey, ExtensionPoint, HookKey, HookKind, OperationKey, ReferenceKind,
    is_valid_key,
};
