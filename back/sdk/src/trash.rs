pub trait TrashExclusion: Send + Sync {
    fn push_hidden(
        &self,
        builder: &mut sqlx::QueryBuilder<sqlx::MySql>,
        definition: &crate::fields::EntityDefinition,
        alias: &str,
    );
}
