pub use aphel_contract_catalog_items::*;
use aphel_kernel::application::public_events::PublicResource;
use aphel_kernel::database::Db;
pub const RESOURCE: PublicResource = PublicResource::new(RECORD_TYPE, "ProductResponse");
pub fn product_not_found() -> crate::AppError {
    crate::AppError::not_found_code(PRODUCT_NOT_FOUND, "product was not found")
}

#[async_trait::async_trait]
pub trait CatalogReferences: Send + Sync {
    async fn existing(
        &self,
        pool: &Db,
        scope: &crate::access::AccessScope,
        ids: &[uuid::Uuid],
    ) -> Result<Vec<uuid::Uuid>, crate::AppError>;
    async fn codes(
        &self,
        pool: &Db,
        scope: &crate::access::AccessScope,
        ids: &[uuid::Uuid],
    ) -> Result<std::collections::HashMap<uuid::Uuid, Option<String>>, crate::AppError>;
    async fn by_codes(
        &self,
        pool: &Db,
        scope: &crate::access::AccessScope,
        codes: &[String],
    ) -> Result<Vec<(String, uuid::Uuid)>, crate::AppError>;
}

pub fn references() -> Result<std::sync::Arc<dyn CatalogReferences>, crate::AppError> {
    aphel_kernel::modules::app()
        .services()
        .get::<dyn CatalogReferences>()
        .ok_or_else(|| {
            crate::AppError::unavailable_code(
                "catalog_references_unavailable",
                "catalog references are not enabled",
            )
        })
}
