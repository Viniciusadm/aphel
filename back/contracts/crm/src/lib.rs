mod fields;
pub mod items;
mod permissions;
mod records;
pub mod refs;
pub mod traits;

pub use fields::{activities_relation, email_engagement_fields};
pub use permissions::CrmPermissions;
pub use records::{CrmRecordTypes, keys};

pub(crate) fn labels(key: &str) -> aphel_kernel::application::fields::LocalizedLabels {
    aphel_kernel::application::fields::LocalizedLabels::from_catalog(
        include_str!("../locales/labels.json"),
        key,
    )
}
