pub(crate) mod privacy;
pub(crate) mod retention;

pub(crate) use privacy::{ColumnAssignment, PrivacyStore};
pub(crate) use retention::RetentionStore;

use aphel_kernel::application::fields::RecordTypeDefinition;

pub(crate) fn free_text_columns(definition: &RecordTypeDefinition) -> &[String] {
    definition.capabilities.free_text_fields.as_slice()
}
