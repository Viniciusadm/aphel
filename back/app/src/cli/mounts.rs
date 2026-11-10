use clap::Subcommand;
use tracing::info;

use crate::cli::CliResult;
use aphel_kernel::api::state::AppState;
use aphel_kernel::config::AppConfig;
use aphel_kernel::errors::AppError;
use aphel_kernel::infrastructure::organization_profile::reset_app_mount;
use aphel_kernel::infrastructure::secrets::build_secret_vault;

#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub(crate) enum MountsCommand {
    #[command(
        about = "Serve the internal app at / again (use when a wrong base path left it unreachable)"
    )]
    Reset,
}

pub(crate) async fn run(config: AppConfig, command: MountsCommand) -> CliResult {
    match command {
        MountsCommand::Reset => reset(config).await,
    }
}

async fn reset(config: AppConfig) -> CliResult {
    let database_url = config
        .database_url
        .as_deref()
        .ok_or_else(|| AppError::Configuration("DATABASE_URL is not configured".to_owned()))?;
    let pool = aphel_kernel::modules::connect_with_modules(database_url).await?;
    let secrets = build_secret_vault(&pool, &config.encryption)?;
    let state = AppState::new(Some(pool.clone()))
        .with_secrets(secrets)
        .with_mysql_platform();
    let app_url = reset_app_mount(
        pool.clone(),
        state.secrets.clone(),
        state.events.clone(),
        state.audit.clone(),
        state.organization_config.clone(),
    )
    .await?;
    info!("the internal app is served at {app_url}/ again; restart is not needed");
    pool.close().await;
    Ok(())
}
