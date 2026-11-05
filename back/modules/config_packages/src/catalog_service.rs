use crate::application::{ConfigInstallRepository, TemplateCatalog};
use crate::infrastructure::{EmbeddedTemplates, MySqlConfigInstallRepository};
use aphel_kernel::{Ctx, errors::AppError};
use aphel_sdk::configuration::{ConfigurationCatalog, InstallationReference};
use aphel_sdk::platform::application::access::Authorizer;
use aphel_sdk::platform::domain::access::Permission;
use uuid::Uuid;

pub(crate) struct Catalog;

#[async_trait::async_trait]
impl ConfigurationCatalog for Catalog {
    async fn installed_versions(
        &self,
        pool: &sqlx::MySqlPool,
        scope: &aphel_sdk::access::AccessScope,
    ) -> Result<Vec<(String, String)>, AppError> {
        use crate::application::versioning::ConfigVersionRepository;
        scope.require(&Permission::SETTINGS_MANAGE)?;
        Ok(
            crate::infrastructure::versioning::MySqlConfigVersionRepository::new(pool.clone())
                .list()
                .await?
                .into_iter()
                .map(|version| (version.package_id, version.version))
                .collect(),
        )
    }

    fn template_exists(&self, ctx: &Ctx<'_>, key: &str, locale: &str) -> Result<bool, AppError> {
        ctx.scope.require(&Permission::SETTINGS_MANAGE)?;
        Ok(EmbeddedTemplates.template(key, locale).is_some())
    }

    async fn installation(
        &self,
        ctx: &Ctx<'_>,
        id: Uuid,
    ) -> Result<Option<InstallationReference>, AppError> {
        ctx.scope.require(&Permission::SETTINGS_MANAGE)?;
        Ok(MySqlConfigInstallRepository::new(ctx.pool().clone())
            .find(id)
            .await?
            .map(|install| InstallationReference {
                id: install.id,
                template_key: install.template_key,
            }))
    }
}
