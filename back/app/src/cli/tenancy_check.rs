use crate::cli::{AlreadyReported, CliResult};
use aphel_kernel::config::{AppConfig, Tenancy};
use aphel_kernel::errors::AppError;
use aphel_kernel::tenancy::{self, ControlPlane};

pub(crate) async fn run(config: AppConfig) -> CliResult {
    let Tenancy::Multi(multi) = config.tenancy()? else {
        return Err(AppError::Configuration(
            "the isolation check needs CONTROL_DATABASE_URL".to_owned(),
        )
        .into());
    };
    tenancy::ensure_multi_tenant_ready()?;
    let control = ControlPlane::connect(&multi).await?;
    let report = tenancy::run_isolation_check(&control, &config).await;
    control.close().await;
    let report = report?;
    for check in &report.checks {
        println!(
            "{} {}: {}",
            if check.passed { "ok  " } else { "FAIL" },
            check.name,
            check.detail
        );
    }
    if report.passed() {
        println!("Tenants are isolated.");
        Ok(())
    } else {
        eprintln!("Isolation check failed.");
        Err(Box::new(AlreadyReported))
    }
}
