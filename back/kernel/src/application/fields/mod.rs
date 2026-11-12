pub mod builtin;
pub(crate) mod capabilities;
pub(crate) mod commands;
pub(crate) mod computed;
pub(crate) mod field_set;
pub mod permissions;
pub(crate) mod pipeline;
pub(crate) mod ports;
pub(crate) mod reader;
pub(crate) mod registry;
pub(crate) mod rules;
pub(crate) mod use_cases;
pub(crate) mod values;
pub(crate) mod writer;

pub use builtin::*;
pub use capabilities::*;
pub use commands::*;
pub use computed::*;
pub use field_set::*;
pub use pipeline::*;
pub use ports::*;
pub use reader::*;
pub use registry::*;
pub use rules::*;
pub use use_cases::*;
pub use values::*;
pub use writer::*;

pub(crate) mod field_types;
pub use field_types::{FieldTypeDefinition, FieldValidator};

mod labels;
pub use labels::LocalizedLabels;

mod storage;
pub use storage::{EntityIndex, EntityStorage, managed_table};

mod declaration;
pub use declaration::{
    DRAFT_PREFIX, EntityDeclaration, FieldDef, OptionDef, PUBLICATION_STATES, PUBLISHED_AT_FIELD,
    REVISED_AT_FIELD, TRANSLATIONS_SUFFIX, publication_state_field, translations_field,
};

mod references;
pub use references::{DeletePolicy, ReferenceTarget, SoftReference};
