pub mod builtin;
pub mod capabilities;
pub mod commands;
pub mod computed;
pub mod field_set;
pub mod permissions;
pub mod pipeline;
pub mod ports;
pub mod reader;
pub mod registry;
pub mod rules;
pub mod use_cases;
pub mod values;
pub mod writer;

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
pub use storage::{EntityIndex, EntityKeyAliases, EntityStorage, managed_table};

mod declaration;
pub use declaration::{EntityDeclaration, FieldDef, OptionDef};

mod references;
pub use references::{DeletePolicy, ReferenceTarget, SoftReference};
