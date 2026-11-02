use tracing::info;

use crate::cli::CliResult;
use aphel_kernel::application::fields::RecordTypeRegistry;
use aphel_kernel::config::AppConfig;
use aphel_kernel::database;
use aphel_kernel::errors::AppError;
use aphel_kernel::infrastructure::custom_objects::sync_custom_object_fields;
use aphel_kernel::infrastructure::defaults::sync_defaults;
use aphel_kernel::infrastructure::fields::sync_system_fields;

pub(crate) async fn run(config: AppConfig) -> CliResult {
    let database_url = config
        .database_url
        .as_deref()
        .ok_or_else(|| AppError::Configuration("DATABASE_URL is not configured".to_owned()))?;

    let pool = database::pool::connect(database_url).await?;
    info!("running database migrations");
    database::migrations::run(&pool).await?;
    sync_system_fields(&pool, &RecordTypeRegistry::builtin()).await?;
    sync_custom_object_fields(&pool).await?;
    sync_defaults(&pool).await?;
    info!("database migrations are up to date");
    pool.close().await;

    Ok(())
}
