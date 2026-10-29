pub(crate) mod keyring;
pub(crate) mod mysql_secret_vault;

use std::sync::Arc;

use sqlx::MySqlPool;

pub use keyring::{EncryptedValue, EncryptionKeyring};
pub use mysql_secret_vault::{MySqlSecretVault, RotationReport, rotate_secrets};

use crate::config::EncryptionConfig;
use crate::errors::AppError;
use crate::infrastructure::noop::UnavailableSecretVault;
use crate::infrastructure::platform::SharedSecretVault;

pub fn build_secret_vault(
    pool: &MySqlPool,
    config: &EncryptionConfig,
) -> Result<SharedSecretVault, AppError> {
    let keyring = EncryptionKeyring::from_config(config)?;
    if !keyring.is_configured() {
        tracing::warn!(
            "APP_ENCRYPTION_KEY not configured; saving webhook and integration secrets will fail"
        );
        return Ok(Arc::new(UnavailableSecretVault));
    }
    Ok(Arc::new(MySqlSecretVault::new(pool.clone(), keyring)))
}
