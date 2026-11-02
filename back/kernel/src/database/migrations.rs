use sqlx::MySqlPool;
use sqlx::migrate::Migrator;

use crate::errors::AppError;

pub static MIGRATOR: Migrator = Migrator {
    ignore_missing: true,
    ..sqlx::migrate!("./migrations")
};

pub async fn run(pool: &MySqlPool) -> Result<(), AppError> {
    MIGRATOR
        .run(pool)
        .await
        .map_err(|error| AppError::Persistence(error.to_string()))?;
    crate::modules::migrations::run(pool, crate::modules::app()).await
}
