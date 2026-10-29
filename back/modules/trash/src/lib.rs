pub(crate) mod domain;
pub(crate) mod dto;
pub(crate) mod http;
pub(crate) mod runtime;
pub(crate) mod service;
pub(crate) mod store;

use aphel_kernel::modules::Migration;

use aphel_kernel::errors::AppError;
use aphel_kernel::modules::builder::APHEL_VERSION;
use aphel_kernel::modules::{AppBuilder, Manifest, Module};

pub use domain::{is_trashable, purged_event, restored_event, trashable_type, trashable_types};
pub use store::{TrashExclusion, TrashSource, TrashStore};

pub const KEY: &str = "trash";

pub struct TrashModule;

impl Module for TrashModule {
    fn manifest(&self) -> Manifest {
        Manifest::new(KEY, "Trash", APHEL_VERSION)
            .description("Deleted records: restore, permanent deletion and retention")
            .enabled_by_default()
            .migration(Migration::new(
                1,
                "create trash settings",
                include_str!("../migrations/0001_create_trash_settings.sql"),
            ))
    }

    fn register(&self, app: &mut AppBuilder) -> Result<(), AppError> {
        app.openapi_tag(
            http::TAG,
            "Deleted records: restore, permanent deletion and retention",
        )
        .routes(|_| http::router());
        app.worker(runtime::trash_worker);
        Ok(())
    }
}
