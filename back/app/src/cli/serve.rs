use std::net::SocketAddr;
use std::sync::Arc;

use tokio::net::TcpListener;
use tracing::{error, info, warn};

use crate::cli::{CliResult, ServeArgs};
use aphel_kernel::api;
use aphel_kernel::api::state::AppState;
use aphel_kernel::application::custom_objects::RecordTypeCatalog;
use aphel_kernel::application::fields::RecordTypeRegistry;
use aphel_kernel::application::organization_profile::PublicBaseUrl;
use aphel_kernel::config::{AppConfig, MultiTenancyConfig, Tenancy};
use aphel_kernel::database;
use aphel_kernel::infrastructure::custom_objects::{
    MySqlCustomSchemaSource, sync_custom_object_fields,
};
use aphel_kernel::infrastructure::fields::sync_system_fields;
use aphel_kernel::infrastructure::jobs::{shutdown_channel, termination_signal};
use aphel_kernel::infrastructure::mail::SmtpMailerProvider;
use aphel_kernel::infrastructure::noop::UnavailableSecretVault;
use aphel_kernel::infrastructure::organization_profile::{
    organization_config_catalog, spawn_config_refresh,
};
use aphel_kernel::infrastructure::platform::SharedSecretVault;
use aphel_kernel::infrastructure::secrets::{EncryptionKeyring, build_secret_vault};
use aphel_kernel::infrastructure::storage::DynamicFileStorage;
use aphel_kernel::modules::WorkerEnv;
use aphel_kernel::tenancy::{self, ControlPlane, TenantDirectory, TenantRuntimes};
use aphel_kernel::worker::WorkerRuntime;

pub(crate) async fn run(config: AppConfig, args: ServeArgs) -> CliResult {
    match config.tenancy()? {
        Tenancy::Single => run_single(config, args).await,
        Tenancy::Multi(tenancy) => run_multi(config, tenancy, args).await,
    }
}

async fn run_multi(config: AppConfig, tenancy: MultiTenancyConfig, args: ServeArgs) -> CliResult {
    tenancy::ensure_multi_tenant_ready()?;
    let control = ControlPlane::connect(&tenancy).await?;
    if config.run_migrations {
        info!("running control and tenant database migrations");
        let summary = tenancy::migrate_all(&control).await?;
        info!(
            migrated = summary.migrated,
            failed = summary.failed,
            "tenant migrations finished"
        );
    }
    let directory = TenantDirectory::load(control.pool().clone()).await?;
    let runtimes = TenantRuntimes::new(control.clone(), config.clone());
    let refresher = tenancy::spawn_directory_refresh(directory.clone(), runtimes.clone());
    let app = tenancy::router(&control, &config, directory, runtimes.clone());
    let (trigger, shutdown) = shutdown_channel();
    let worker = args.with_worker.then(|| {
        tokio::spawn(tenancy::run_workers(
            control.clone(),
            config.clone(),
            shutdown,
        ))
    });
    let listener = TcpListener::bind(config.http_addr()).await?;

    info!(
        "backend listening on {} for {} companies",
        config.http_addr(),
        directory_size(&control).await
    );
    let served = axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(async move {
        termination_signal().await;
        info!("shutdown requested");
    })
    .await;

    trigger.trigger();
    if let Some(worker) = worker {
        match worker.await {
            Ok(Ok(())) => {}
            Ok(Err(failure)) => error!("tenant workers stopped with an error: {failure}"),
            Err(join_error) => error!("tenant workers stopped unexpectedly: {join_error}"),
        }
    }
    refresher.abort();
    runtimes.clear();
    control.close().await;
    served?;
    Ok(())
}

async fn directory_size(control: &ControlPlane) -> i64 {
    super::store::active_tenant_count(control.pool()).await
}

async fn run_single(config: AppConfig, args: ServeArgs) -> CliResult {
    let db = if let Some(database_url) = &config.database_url {
        let pool = database::pool::connect(database_url).await?;
        if config.run_migrations {
            info!("running database migrations");
            database::migrations::run(&pool).await?;
            sync_system_fields(&pool, &RecordTypeRegistry::builtin()).await?;
            sync_custom_object_fields(&pool).await?;
        }
        Some(pool)
    } else {
        warn!(
            "DATABASE_URL not configured; API endpoints that require persistence will return configuration errors"
        );
        None
    };

    let secrets: SharedSecretVault = match &db {
        Some(pool) => build_secret_vault(pool, &config.encryption)?,
        None => Arc::new(UnavailableSecretVault),
    };
    let refresher = db.as_ref().map(|pool| {
        let catalog = organization_config_catalog(pool);
        spawn_config_refresh(catalog)
    });
    let modules_refresher = match &db {
        Some(pool) => {
            let enabled = aphel_kernel::modules::app().enabled().clone();
            enabled.refresh(pool).await?;
            Some(enabled.spawn_refresh(pool.clone()))
        }
        None => None,
    };
    if let Some(pool) = &db {
        organization_config_catalog(pool).refresh().await;
    }
    let (trigger, shutdown) = shutdown_channel();
    let worker = match (&db, args.with_worker) {
        (Some(pool), true) => {
            let catalog = organization_config_catalog(pool);
            let mailer = SmtpMailerProvider::shared(pool.clone(), secrets.clone());
            let storage = DynamicFileStorage::shared(catalog.clone(), secrets.clone());
            let public_url = PublicBaseUrl::live(catalog.clone());
            let env = WorkerEnv {
                pool: pool.clone(),
                secrets: secrets.clone(),
                mailer,
                storage,
                public_url,
                outbound_http: config.outbound_http.clone(),
                record_types: Arc::new(RecordTypeCatalog::new(
                    RecordTypeRegistry::builtin(),
                    Arc::new(MySqlCustomSchemaSource::new(pool.clone())),
                )),
            };
            let runtime =
                WorkerRuntime::from_app(aphel_kernel::modules::app(), &env, &config.worker);
            Some(tokio::spawn(runtime.run(shutdown)))
        }
        (None, true) => {
            warn!("--with-worker ignored: DATABASE_URL not configured");
            None
        }
        (_, false) => None,
    };

    let state = AppState::new(db)
        .with_secrets(secrets)
        .with_encryption(EncryptionKeyring::from_config(&config.encryption)?)
        .with_outbound_http(config.outbound_http.clone())
        .with_mysql_platform()
        .with_organization_storage();
    let app = api::router(state);
    let listener = TcpListener::bind(config.http_addr()).await?;

    info!("backend listening on {}", config.http_addr());
    let trigger = Arc::new(trigger);
    let server_trigger = trigger.clone();
    let served = axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(async move {
        termination_signal().await;
        info!("shutdown requested");
        server_trigger.trigger();
    })
    .await;

    trigger.trigger();
    if let Some(refresher) = refresher {
        refresher.abort();
    }
    if let Some(refresher) = modules_refresher {
        refresher.abort();
    }
    if let Some(worker) = worker {
        match worker.await {
            Ok(Ok(())) => {}
            Ok(Err(failure)) => error!("job worker stopped with an error: {failure}"),
            Err(join_error) => error!("job worker stopped unexpectedly: {join_error}"),
        }
    }
    served?;

    Ok(())
}
