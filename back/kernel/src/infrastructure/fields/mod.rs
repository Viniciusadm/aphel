pub mod mysql_definitions;
pub(crate) mod mysql_references;
pub(crate) mod mysql_values;
pub mod permissions;
pub(crate) mod ports;
pub mod unique_values;

pub use mysql_definitions::{
    MySqlFieldDefinitionRepository, seed_managed_fields, seed_system_fields, sync_system_fields,
};
pub use mysql_references::MySqlReferenceChecker;
pub use mysql_values::MySqlCustomFieldValueRepository;
pub use permissions::{MySqlFieldPermissionPorts, MySqlFieldPermissionRepository};
pub use ports::{MySqlCustomFieldPorts, MySqlFieldPorts};

mod managed;
pub use managed::{column_type, ensure_extension_schemas, ensure_managed_schema, managed_schema};

mod entity_keys;
pub(crate) use entity_keys::migrate_entity_keys;
