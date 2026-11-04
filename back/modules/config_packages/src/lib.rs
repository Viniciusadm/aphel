pub mod application;
pub mod dto;
pub mod http;
pub mod infrastructure;
mod key_service;
pub mod versions_dto;
pub mod versions_http;

use aphel_kernel::errors::AppError;
use aphel_kernel::modules::{AppBuilder, Manifest, Module};
pub const KEY: &str = "config_packages";
pub struct ConfigPackagesModule;
impl Module for ConfigPackagesModule {
    fn manifest(&self) -> Manifest {
        Manifest::new(
            KEY,
            "Configuration packages",
            aphel_kernel::modules::builder::APHEL_VERSION,
        )
        .plugin_toml(include_str!("../plugin.toml"))
        .enabled_by_default()
        .migration(aphel_kernel::modules::Migration::new(
            1,
            "create config package installs",
            include_str!("../migrations/1000_create_config_package_installs.sql"),
        ))
        .migration(aphel_kernel::modules::Migration::new(
            2,
            "create config package versions",
            include_str!("../migrations/2170_create_config_package_versions.sql"),
        ))
        .owned_tables(&[
            "config_package_installs",
            "config_package_versions",
            "config_package_version_items",
        ])
    }
    fn register(&self, app: &mut AppBuilder) -> Result<(), AppError> {
        app.contract(aphel_contract_configuration::DESCRIPTOR);
        app.provide::<dyn aphel_sdk::configuration::ConfigurationKeys>(std::sync::Arc::new(
            key_service::Keys,
        ));
        app.contribute(std::sync::Arc::new(
            crate::application::versioning::fragments::SectionFragmentLayout {
                section: "fields",
                collections: vec![
                    crate::application::versioning::fragments::FragmentCollection {
                        path: None,
                        kind: "field",
                        key_fields: &["record_type", "key"],
                    },
                ],
            },
        ));

        app.openapi_tag("config-packages", "Configuration packages")
            .routes(|_| http::router().merge(versions_http::router()));
        app.contribute::<dyn aphel_kernel::modules::ReferenceKeySource>(std::sync::Arc::new(
            KeySource,
        ));
        Ok(())
    }
}
struct KeySource;
#[async_trait::async_trait]
impl aphel_kernel::modules::ReferenceKeySource for KeySource {
    async fn stable_key(
        &self,
        ctx: &aphel_kernel::Ctx<'_>,
        kind: aphel_contract::ReferenceKind,
        id: &str,
    ) -> Result<Option<String>, AppError> {
        Ok(sqlx::query_scalar(
            "SELECT `key` FROM config_item_keys WHERE item_type = ? AND item_id = ?",
        )
        .bind(kind.key())
        .bind(id)
        .fetch_optional(ctx.pool())
        .await?)
    }
    async fn resolve_key(
        &self,
        ctx: &aphel_kernel::Ctx<'_>,
        kind: aphel_contract::ReferenceKind,
        key: &str,
    ) -> Result<Option<String>, AppError> {
        Ok(sqlx::query_scalar(
            "SELECT item_id FROM config_item_keys WHERE item_type = ? AND `key` = ?",
        )
        .bind(kind.key())
        .bind(key)
        .fetch_optional(ctx.pool())
        .await?)
    }
}
