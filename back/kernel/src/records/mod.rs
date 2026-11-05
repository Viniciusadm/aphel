pub mod contributed;
pub mod duplicates;
pub mod handler;
pub mod hooks;
pub mod input;
pub mod mutation;
pub mod pipeline;
pub mod purge;
pub mod runtime;
pub mod types;
pub mod version;

pub use contributed::*;
pub use duplicates::{DuplicateDetector, PossibleDuplicate, merge_survivor, possible_duplicates};
pub use handler::*;
pub use hooks::*;
pub use input::*;
pub use mutation::*;
pub use pipeline::{
    MySqlRecordPipeline, constrain_query, filter_schema, filter_search_plan,
    serialize_nested_records, serialize_records,
};
pub use purge::purge_record_type_data;
pub use runtime::{field_set, query_ports};
pub use types::{RecordContributionInfo, RecordTypeExtension};
pub use version::lock_version;

pub mod snapshot;

mod generic;
pub use generic::{EntityWriteBehavior, GenericRecordHandler};

pub(crate) mod references;

mod maintenance;
pub use maintenance::{
    StoredTextField, TextAssignment, TextComparison, archive_entity, existing_entity_ids,
    inspect_entity, lock_entity_version, mark_entity_deleted, matching_entity_text_ids,
    purge_entity, update_entity_text,
};

pub use references::{copy_custom_columns, existing_tables, move_core_references, retire_custom};

pub use purge::{
    backdate_dataset_history, cleanup_dataset_files, cleanup_dataset_records,
    cleanup_dataset_references, dataset_core_counts,
};
