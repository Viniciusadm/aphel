mod fields;
pub mod items;
mod permissions;
mod records;
pub mod refs;
pub mod traits;

pub use fields::{activities_relation, address_fields, email_engagement_fields, phone_field};
pub use permissions::CrmPermissions;
pub use records::CrmRecordTypes;

pub(crate) fn labels(key: &str) -> aphel_kernel::application::fields::LocalizedLabels {
    aphel_kernel::application::fields::LocalizedLabels::from_catalog(
        include_str!("../locales/labels.json"),
        key,
    )
}
