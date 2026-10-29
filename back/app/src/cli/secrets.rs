use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use clap::Subcommand;
use tracing::info;

use crate::cli::CliResult;
use aphel_kernel::config::AppConfig;
use aphel_kernel::database;
use aphel_kernel::errors::AppError;
use aphel_kernel::infrastructure::secrets::{EncryptionKeyring, rotate_secrets};
use aphel_kernel::infrastructure::two_factor::rotate_two_factor_secrets;

#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub(crate) enum SecretsCommand {
    #[command(about = "Print a new random value for APP_ENCRYPTION_KEY")]
    GenerateKey,
    #[command(
        about = "Re-encrypt every stored secret with APP_ENCRYPTION_KEY (old keys must be in APP_ENCRYPTION_PREVIOUS_KEYS)"
    )]
    Rotate,
}

pub(crate) async fn run(config: AppConfig, command: SecretsCommand) -> CliResult {
    match command {
        SecretsCommand::GenerateKey => generate_key(),
        SecretsCommand::Rotate => rotate(config).await,
    }
}

fn generate_key() -> CliResult {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes)
        .map_err(|error| AppError::Internal(format!("failed to generate key: {error}")))?;
    println!("{}", STANDARD.encode(bytes));
    Ok(())
}

async fn rotate(config: AppConfig) -> CliResult {
    let database_url = config
        .database_url
        .as_deref()
        .ok_or_else(|| AppError::Configuration("DATABASE_URL is not configured".to_owned()))?;
    let keyring = EncryptionKeyring::from_config(&config.encryption)?;
    let pool = database::pool::connect(database_url).await?;
    let report = rotate_secrets(&pool, &keyring).await?;
    info!(
        "re-encrypted {} secrets; {} secrets stored in total",
        report.rotated, report.current
    );
    let two_factor = rotate_two_factor_secrets(&pool, &keyring).await?;
    info!("re-encrypted {two_factor} two-factor secrets");
    pool.close().await;
    Ok(())
}
