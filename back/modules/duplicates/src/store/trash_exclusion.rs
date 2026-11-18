use aphel_sdk::platform::application::fields::EntityDefinition;
use sqlx::{MySql, QueryBuilder};

pub(crate) struct MergedRecords;

impl aphel_sdk::trash::TrashExclusion for MergedRecords {
    fn push_hidden(
        &self,
        builder: &mut QueryBuilder<MySql>,
        definition: &EntityDefinition,
        alias: &str,
    ) {
        builder
            .push(" AND NOT EXISTS (SELECT 1 FROM record_merges m WHERE m.record_type = ")
            .push_bind(definition.record_type.as_str().to_owned())
            .push(format!(" AND m.merged_id = {alias}.id)"));
    }
}
