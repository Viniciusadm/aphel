pub trait TrashExclusion: Send + Sync {
    fn push_hidden(
        &self,
        builder: &mut aphel_kernel::database::SqlBuilder,
        definition: &crate::fields::EntityDefinition,
        alias: &str,
    );
}
