pub use aphel_contract_people::*;
use aphel_kernel::application::public_events::PublicResource;
pub const RESOURCE: PublicResource = PublicResource::new(RECORD_TYPE, "ContactResponse")
    .with_nested(&[("company", crate::organizations::RECORD_TYPE)]);

#[derive(Debug, Clone, Copy)]
pub struct PrimaryOrganization {
    pub person_id: uuid::Uuid,
    pub organization_id: uuid::Uuid,
}
#[async_trait::async_trait]
pub trait PrimaryOrganizationReader: Send + Sync {
    async fn by_people(
        &self,
        pool: &sqlx::MySqlPool,
        scope: &crate::access::AccessScope,
        ids: &[uuid::Uuid],
        include_deleted: bool,
    ) -> Result<Vec<PrimaryOrganization>, crate::AppError>;
    async fn people_page(
        &self,
        pool: &sqlx::MySqlPool,
        scope: &crate::access::AccessScope,
        organizations: &[uuid::Uuid],
        after: Option<uuid::Uuid>,
    ) -> Result<Vec<PrimaryOrganization>, crate::AppError>;
}
pub fn primary_organizations() -> Option<std::sync::Arc<dyn PrimaryOrganizationReader>> {
    aphel_kernel::modules::app()
        .services()
        .get::<dyn PrimaryOrganizationReader>()
}

pub struct PersonCreation {
    pub values: serde_json::Map<String, serde_json::Value>,
    pub custom_fields: serde_json::Map<String, serde_json::Value>,
    pub ip: Option<String>,
}
#[async_trait::async_trait]
pub trait PersonWriter: Send + Sync {
    async fn create_in_tx(
        &self,
        ctx: &crate::Ctx<'_>,
        tx: &mut aphel_kernel::database::transaction::MySqlTx,
        creation: PersonCreation,
    ) -> Result<crate::records::PendingCommit, crate::AppError>;
}
pub fn writer() -> Result<std::sync::Arc<dyn PersonWriter>, crate::AppError> {
    aphel_kernel::modules::app()
        .services()
        .get::<dyn PersonWriter>()
        .ok_or_else(|| {
            crate::AppError::unavailable_code(
                "person_writer_unavailable",
                "person creation is not enabled",
            )
        })
}
