pub use aphel_contract_people::*;
use aphel_kernel::application::public_events::PublicResource;
pub const RESOURCE: PublicResource = PublicResource::new(RECORD_TYPE, "ContactResponse")
    .with_nested(&[("company", crate::organizations::RECORD_TYPE)]);
