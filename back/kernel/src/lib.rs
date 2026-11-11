pub mod api;
pub mod application;
pub mod config;
pub mod context;
mod context_capabilities;
pub mod database;
pub mod domain;
pub mod errors;
pub mod infrastructure;
pub(crate) mod kernel_module;
pub mod modules;
pub mod records;
pub mod state;
pub mod tenancy;
pub mod worker;

pub use context::Ctx;
pub use kernel_module::KernelModule;

pub(crate) fn labels(key: &str) -> crate::application::fields::LocalizedLabels {
    crate::application::fields::LocalizedLabels::from_catalog(
        include_str!("../locales/labels.json"),
        key,
    )
}
