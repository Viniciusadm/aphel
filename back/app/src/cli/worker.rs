use std::sync::Arc;

use tracing::info;

use crate::cli::CliResult;
use aphel_kernel::application::custom_objects::RecordTypeCatalog;
use aphel_kernel::application::fields::RecordTypeRegistry;
use aphel_kernel::application::organizations::PublicBaseUrl;
use aphel_kernel::config::AppConfig;
use aphel_kernel::database;
use aphel_kernel::errors::AppError;
use aphel_kernel::infrastructure::custom_objects::{
    MySqlCustomSchemaSource, sync_custom_object_fields,
};
use aphel_kernel::infrastructure::defaults::sync_defaults;
use aphel_kernel::infrastructure::fields::sync_system_fields;
use aphel_kernel::infrastructure::jobs::{shutdown_channel, termination_signal};
use aphel_kernel::infrastructure::mail::SmtpMailerProvider;
use aphel_kernel::infrastructure::organizations::{
    organization_config_catalog, spawn_config_refresh,
};
use aphel_kernel::infrastructure::secrets::build_secret_vault;
use aphel_kernel::infrastructure::storage::DynamicFileStorage;
use aphel_kernel::modules::WorkerEnv;
use aphel_kernel::worker::WorkerRuntime;

pub(crate) async fn run(config: AppConfig) -> CliResult {
    let database_url = config
        .database_url
        .as_deref()
        .ok_or_else(|| AppError::Configuration("DATABASE_URL is not configured".to_owned()))?;

    let pool = database::pool::connect(database_url).await?;
    if config.run_migrations {
        info!("running database migrations");
        database::migrations::run(&pool).await?;
        sync_system_fields(&pool, &RecordTypeRegistry::builtin()).await?;
        sync_custom_object_fields(&pool).await?;
        sync_defaults(&pool).await?;
    }

    let secrets = build_secret_vault(&pool, &config.encryption)?;
    let (trigger, shutdown) = shutdown_channel();
    let signal = tokio::spawn(async move {
        termination_signal().await;
        info!("shutdown requested; waiting for running jobs to finish");
        trigger.trigger();
    });

    let catalog = organization_config_catalog(&pool);
    catalog.refresh().await;
    let refresher = spawn_config_refresh(catalog.clone());
    let mailer = SmtpMailerProvider::shared(pool.clone(), secrets.clone());
    let storage = DynamicFileStorage::shared(catalog.clone(), secrets.clone());
    let public_url = PublicBaseUrl::live(catalog.clone());
    let app = aphel_kernel::modules::app();
    app.enabled().refresh(&pool).await?;
    let modules_refresher = app.enabled().clone().spawn_refresh(pool.clone());
    let env = WorkerEnv {
        pool: pool.clone(),
        secrets,
        mailer,
        storage,
        public_url,
        outbound_http: config.outbound_http.clone(),
        record_types: Arc::new(RecordTypeCatalog::new(
            RecordTypeRegistry::builtin(),
            Arc::new(MySqlCustomSchemaSource::new(pool.clone())),
        )),
    };
    let result = WorkerRuntime::from_app(app, &env, &config.worker)
        .run(shutdown)
        .await;
    modules_refresher.abort();
    refresher.abort();
    signal.abort();
    pool.close().await;
    result?;
    Ok(())
}
