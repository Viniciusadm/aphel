pub use aphel_contract_catalog_items::*;
use aphel_kernel::application::public_events::PublicResource;
pub const RESOURCE: PublicResource = PublicResource::new(RECORD_TYPE, "ProductResponse");
pub fn product_not_found() -> crate::AppError {
    crate::AppError::not_found_code(PRODUCT_NOT_FOUND, "product was not found")
}
