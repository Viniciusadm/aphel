use crate::application::ConfigPackageUseCases;
use crate::infrastructure::MySqlConfigPackagePorts;
use aphel_kernel::{Ctx, errors::AppError};
use aphel_sdk::configuration::{
    ApplyCommand, ConfigPackage, ConfigurationPackages, ExportCommand, InstallResult, PackagePlan,
    PackageSource,
};

pub(crate) struct Packages;

fn packages(ctx: &Ctx<'_>) -> ConfigPackageUseCases<MySqlConfigPackagePorts> {
    ConfigPackageUseCases::new(MySqlConfigPackagePorts::new(
        ctx.pool().clone(),
        ctx.registry().clone(),
        ctx.events().clone(),
        ctx.audit().clone(),
    ))
}

#[async_trait::async_trait]
impl ConfigurationPackages for Packages {
    async fn export(
        &self,
        ctx: &Ctx<'_>,
        command: ExportCommand,
        locale: &str,
    ) -> Result<ConfigPackage, AppError> {
        packages(ctx).export(ctx.scope, command, locale).await
    }
    async fn plan(&self, ctx: &Ctx<'_>, source: PackageSource) -> Result<PackagePlan, AppError> {
        packages(ctx).plan(ctx.scope, source).await
    }
    async fn apply(&self, ctx: &Ctx<'_>, command: ApplyCommand) -> Result<InstallResult, AppError> {
        packages(ctx).apply(ctx.scope, command).await
    }
}

#[async_trait::async_trait]
impl aphel_sdk::configuration::versioning::ConfigurationVersions for Packages {
    async fn compare(
        &self,
        ctx: &Ctx<'_>,
        package: &serde_json::Value,
    ) -> Result<aphel_sdk::configuration::versioning::VersionComparison, AppError> {
        versions(ctx).compare(ctx.scope, package).await
    }
    async fn apply(
        &self,
        ctx: &Ctx<'_>,
        command: aphel_sdk::configuration::versioning::ApplyVersionCommand,
    ) -> Result<aphel_sdk::configuration::versioning::VersionApplyResult, AppError> {
        versions(ctx).apply(ctx.scope, command).await
    }
}
fn versions(
    ctx: &Ctx<'_>,
) -> crate::application::versioning::ConfigVersionUseCases<
    crate::infrastructure::versioning::MySqlConfigVersionPorts,
> {
    crate::application::versioning::ConfigVersionUseCases::new(
        crate::infrastructure::versioning::MySqlConfigVersionPorts::new(
            ctx.pool().clone(),
            ctx.registry().clone(),
            ctx.events().clone(),
            ctx.audit().clone(),
        ),
    )
}
