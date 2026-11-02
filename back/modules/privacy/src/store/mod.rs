pub(crate) mod privacy;
pub(crate) mod retention;

pub(crate) use privacy::{ColumnAssignment, PrivacyStore};
pub(crate) use retention::RetentionStore;

use aphel_kernel::application::fields::EntityDefinition;

pub(crate) fn free_text_columns(definition: &EntityDefinition) -> &[String] {
    definition.capabilities.free_text_fields.as_slice()
}
