pub(crate) mod css;
pub(crate) mod domain;
pub(crate) mod http;
pub(crate) mod package;
pub(crate) mod service;
pub(crate) mod store;
pub(crate) mod tokens;
pub(crate) mod validate;

use aphel_kernel::domain::access::{Permission, PermissionDefinition};
use aphel_kernel::errors::AppError;
use aphel_kernel::modules::builder::APHEL_VERSION;
use aphel_kernel::modules::{AppBuilder, Manifest, Migration, Module};

pub(crate) const KEY: &str = "themes";
pub(crate) const MANAGE: Permission = Permission::from_static("themes.manage");

pub struct ThemesModule;

impl Module for ThemesModule {
    fn manifest(&self) -> Manifest {
        Manifest::new(KEY, "Themes", APHEL_VERSION)
            .description("Installable visual themes: design tokens and fonts applied to the whole installation, public pages and plugins included")
            .enabled_by_default()
            .owned_tables(&["themes_installed", "themes_fonts", "themes_state"])
            .migration(Migration::new(
                1,
                "create themes",
                include_str!("../migrations/0001_create_themes.sql"),
            ))
    }

    fn register(&self, app: &mut AppBuilder) -> Result<(), AppError> {
        app.openapi_tag(
            http::TAG,
            "Installable visual themes: design tokens and fonts applied to the whole installation",
        )
        .routes(|_| http::router());
        app.permission(PermissionDefinition::administrative(MANAGE))
            .grant_to(["admin"]);
        Ok(())
    }
}
