use std::collections::HashSet;
use std::env;
use std::path::{Path, PathBuf};
use std::time::Duration;

use chrono::Utc;
use sqlx::mysql::MySqlPoolOptions;

use crate::cli::CliResult;
use aphel_kernel::application::organization_profile::{StorageSettings, storage_secret_key};
use aphel_kernel::application::shared::SharedFileStorage;
use aphel_kernel::application::system::doctor::{
    DoctorCheck, DoctorReport, DoctorStatus, collect_settings, is_sensitive_name, render_markdown,
};
use aphel_kernel::application::system::health::age_seconds;
use aphel_kernel::application::system::upgrade::{MIN_MYSQL_MAJOR, mysql_version_supported};
use aphel_kernel::application::system::{APHEL_VERSION, MigrationStatus, WORKER_ROLE};
use aphel_kernel::config::AppConfig;
use aphel_kernel::database::Db;
use aphel_kernel::domain::access::AccessScope;
use aphel_kernel::infrastructure::organization_profile::organization_config_catalog;
use aphel_kernel::infrastructure::secrets::EncryptionKeyring;
use aphel_kernel::infrastructure::secrets::build_secret_vault;
use aphel_kernel::infrastructure::storage::build_file_storage;
use aphel_kernel::infrastructure::system::migrations::{
    compiled_modules, enabled_modules, migration_status,
};
use aphel_kernel::infrastructure::system::{SystemSettings, disk, monitor, probe};

const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);

pub(crate) async fn connect_quietly(database_url: &str) -> Result<Db, String> {
    let pool = MySqlPoolOptions::new()
        .max_connections(2)
        .acquire_timeout(CONNECT_TIMEOUT)
        .connect(database_url);
    match tokio::time::timeout(CONNECT_TIMEOUT * 2, pool).await {
        Ok(Ok(pool)) => Ok(pool),
        Ok(Err(error)) => Err(describe_sqlx_error(&error)),
        Err(_) => Err("timed out while connecting".to_owned()),
    }
}

fn describe_sqlx_error(error: &sqlx::Error) -> String {
    match error {
        sqlx::Error::Io(io) => format!("network error: {}", io.kind()),
        sqlx::Error::Tls(_) => "TLS negotiation failed".to_owned(),
        sqlx::Error::Database(database) => format!(
            "database refused the connection (code {})",
            database.code().as_deref().unwrap_or("unknown")
        ),
        sqlx::Error::PoolTimedOut => "timed out while connecting".to_owned(),
        sqlx::Error::Configuration(_) => "invalid DATABASE_URL".to_owned(),
        _ => "could not connect".to_owned(),
    }
}

pub(crate) async fn mysql_version(pool: &Db) -> Option<String> {
    super::store::server_version(pool).await
}

pub(crate) fn mysql_check(version: Option<&str>) -> DoctorCheck {
    match version {
        Some(version) if mysql_version_supported(version) => {
            DoctorCheck::new("mysql version", DoctorStatus::Ok, version)
        }
        Some(version) => DoctorCheck::new(
            "mysql version",
            DoctorStatus::Fail,
            format!("{version} is not supported (MySQL {MIN_MYSQL_MAJOR}.0 or newer is required)"),
        ),
        None => DoctorCheck::new("mysql version", DoctorStatus::Skipped, "unknown"),
    }
}

async fn configured_storage(
    config: &AppConfig,
    pool: &Db,
) -> Result<(StorageSettings, SharedFileStorage), String> {
    let settings = organization_config_catalog(pool)
        .current()
        .await
        .map_err(|error| error.client_message().to_owned())?
        .storage
        .clone();
    let secret = match &settings {
        StorageSettings::S3(_) => build_secret_vault(pool, &config.encryption)
            .map_err(|error| error.client_message().to_owned())?
            .reveal(&AccessScope::system(), &storage_secret_key())
            .await
            .map_err(|error| error.client_message().to_owned())?,
        StorageSettings::Local { .. } => None,
    };
    let storage = build_file_storage(&settings, secret.as_ref())
        .map_err(|error| error.client_message().to_owned())?;
    Ok((settings, storage))
}

pub(crate) async fn storage_check(config: &AppConfig, pool: &Db) -> DoctorCheck {
    let (settings, storage) = match configured_storage(config, pool).await {
        Ok(configured) => configured,
        Err(reason) => {
            return DoctorCheck::new(
                "file storage",
                DoctorStatus::Fail,
                format!("invalid configuration: {reason}"),
            );
        }
    };
    let driver = match settings {
        StorageSettings::Local { .. } => "local disk",
        StorageSettings::S3(_) => "S3 compatible",
    };
    match tokio::time::timeout(Duration::from_secs(15), probe::probe_storage(&storage)).await {
        Ok(Ok(())) => DoctorCheck::new(
            "file storage",
            DoctorStatus::Ok,
            format!("{driver}: write and delete succeeded"),
        ),
        Ok(Err(_)) => DoctorCheck::new(
            "file storage",
            DoctorStatus::Fail,
            format!(
                "{driver}: could not write a probe file (check permissions, bucket and credentials)"
            ),
        ),
        Err(_) => DoctorCheck::new(
            "file storage",
            DoctorStatus::Fail,
            format!("{driver}: timed out"),
        ),
    }
}

pub(crate) async fn disk_space_checks(pool: &Db, min_free_mb: u64) -> Vec<DoctorCheck> {
    let mut targets = Vec::new();
    if let Ok(configuration) = organization_config_catalog(pool).current().await
        && let StorageSettings::Local { path } = &configuration.storage
    {
        targets.push(("disk space (file storage)", PathBuf::from(path)));
    }
    targets.push(("disk space (working directory)", PathBuf::from(".")));
    targets
        .into_iter()
        .map(|(name, path)| disk_space_check(name, &path, min_free_mb))
        .collect()
}

fn disk_space_check(name: &str, path: &Path, min_free_mb: u64) -> DoctorCheck {
    const MEGABYTE: u64 = 1024 * 1024;
    match disk::available_bytes(path) {
        None => DoctorCheck::new(name, DoctorStatus::Skipped, "could not read the free space"),
        Some(bytes) => {
            let free_mb = bytes / MEGABYTE;
            let status = if free_mb < min_free_mb {
                DoctorStatus::Fail
            } else {
                DoctorStatus::Ok
            };
            DoctorCheck::new(
                name,
                status,
                format!("{free_mb} MB free (minimum {min_free_mb} MB, UPGRADE_MIN_FREE_DISK_MB)"),
            )
        }
    }
}

async fn smtp_check(pool: &Db) -> DoctorCheck {
    let row = super::store::smtp_endpoint(pool).await;
    let Some((Some(host), Some(port))) = row else {
        return DoctorCheck::new(
            "smtp",
            DoctorStatus::Skipped,
            "outgoing email is not configured (Settings → Email)",
        );
    };
    let address = (host, u16::try_from(port).unwrap_or_default());
    match tokio::time::timeout(
        CONNECT_TIMEOUT,
        tokio::net::TcpStream::connect(address.clone()),
    )
    .await
    {
        Ok(Ok(_)) => DoctorCheck::new(
            "smtp",
            DoctorStatus::Ok,
            format!("{}:{} reachable (TCP)", address.0, address.1),
        ),
        Ok(Err(error)) => DoctorCheck::new(
            "smtp",
            DoctorStatus::Fail,
            format!("{}:{} unreachable: {}", address.0, address.1, error.kind()),
        ),
        Err(_) => DoctorCheck::new(
            "smtp",
            DoctorStatus::Fail,
            format!("{}:{} timed out", address.0, address.1),
        ),
    }
}

fn encryption_check(config: &AppConfig) -> DoctorCheck {
    match EncryptionKeyring::from_config(&config.encryption) {
        Ok(keyring) if keyring.is_configured() => DoctorCheck::new(
            "encryption key",
            DoctorStatus::Ok,
            "APP_ENCRYPTION_KEY is valid",
        ),
        Ok(_) => DoctorCheck::new(
            "encryption key",
            DoctorStatus::Warning,
            "APP_ENCRYPTION_KEY is not configured: secrets cannot be stored",
        ),
        Err(_) => DoctorCheck::new(
            "encryption key",
            DoctorStatus::Fail,
            "APP_ENCRYPTION_KEY or APP_ENCRYPTION_PREVIOUS_KEYS is invalid",
        ),
    }
}

pub(crate) fn migrations_check(status: &MigrationStatus) -> DoctorCheck {
    let problems = status.blocking_problems();
    let pending = status.pending_count();
    let incompatible: Vec<&str> = status
        .modules
        .iter()
        .filter(|module| !module.is_compatible())
        .map(|module| module.key.as_str())
        .collect();
    if !problems.is_empty() {
        return DoctorCheck::new("migrations", DoctorStatus::Fail, problems.join("; "));
    }
    let mut detail = format!(
        "{} core applied, {pending} pending (core and modules)",
        status.core.applied
    );
    if !incompatible.is_empty() {
        detail.push_str(&format!(
            "; incompatible modules skipped: {}",
            incompatible.join(", ")
        ));
    }
    let state = if pending > 0 || !incompatible.is_empty() {
        DoctorStatus::Warning
    } else {
        DoctorStatus::Ok
    };
    DoctorCheck::new("migrations", state, detail)
}

async fn worker_checks(pool: &Db, settings: &SystemSettings) -> Vec<DoctorCheck> {
    let now = Utc::now();
    let thresholds = settings.readiness;
    let mut checks = Vec::new();
    checks.push(match probe::latest_heartbeat(pool, WORKER_ROLE).await {
        Ok(Some(seen_at)) => {
            let age = age_seconds(now, seen_at);
            let status = if age > thresholds.worker_max_silence.as_secs() as i64 {
                DoctorStatus::Fail
            } else {
                DoctorStatus::Ok
            };
            DoctorCheck::new("worker heartbeat", status, format!("last seen {age}s ago"))
        }
        Ok(None) => DoctorCheck::new(
            "worker heartbeat",
            DoctorStatus::Fail,
            "no worker has reported (is `aphel worker` or `serve --with-worker` running?)",
        ),
        Err(_) => DoctorCheck::new(
            "worker heartbeat",
            DoctorStatus::Skipped,
            "table not available",
        ),
    });
    checks.push(match probe::oldest_due_schedule(pool).await {
        Ok(Some(next_run_at)) if next_run_at >= now => {
            DoctorCheck::new("scheduler", DoctorStatus::Ok, "recurring jobs are on time")
        }
        Ok(Some(next_run_at)) => {
            let age = age_seconds(now, next_run_at);
            let status = if age > thresholds.scheduler_max_delay.as_secs() as i64 {
                DoctorStatus::Fail
            } else {
                DoctorStatus::Ok
            };
            DoctorCheck::new(
                "scheduler",
                status,
                format!("most overdue schedule is {age}s late"),
            )
        }
        Ok(None) => DoctorCheck::new("scheduler", DoctorStatus::Fail, "never started"),
        Err(_) => DoctorCheck::new("scheduler", DoctorStatus::Skipped, "table not available"),
    });
    checks.push(match monitor::job_counts(pool, now).await {
        Ok(counts) => {
            let ready: u64 = counts.iter().map(|entry| entry.ready).sum();
            let failed: u64 = counts.iter().map(|entry| entry.failed).sum();
            let oldest = probe::oldest_ready_job(pool, now)
                .await
                .ok()
                .flatten()
                .map_or(0, |run_at| age_seconds(now, run_at));
            let status = if oldest > thresholds.queue_max_delay.as_secs() as i64 {
                DoctorStatus::Fail
            } else if failed > 0 {
                DoctorStatus::Warning
            } else {
                DoctorStatus::Ok
            };
            DoctorCheck::new(
                "job queue",
                status,
                format!("{ready} ready (oldest {oldest}s), {failed} failed"),
            )
        }
        Err(_) => DoctorCheck::new("job queue", DoctorStatus::Skipped, "table not available"),
    });
    checks
}

fn sensitive_environment() -> Vec<String> {
    env::vars()
        .filter(|(name, _)| is_sensitive_name(name) || name == "DATABASE_URL")
        .map(|(_, value)| value.trim().to_owned())
        .filter(|value| value.chars().count() >= 3)
        .collect()
}

fn aphel_variable_names() -> Vec<String> {
    const PREFIXES: &[&str] = &[
        "APP_",
        "DATABASE_",
        "METRICS_",
        "APHEL_",
        "WEBHOOK",
        "HEALTH_",
        "WORKER_",
    ];
    env::vars()
        .map(|(name, _)| name)
        .filter(|name| PREFIXES.iter().any(|prefix| name.starts_with(prefix)))
        .collect()
}

pub(crate) async fn run(config: AppConfig) -> CliResult {
    let settings = SystemSettings::from_env();
    let collected = collect_settings(|name| env::var(name).ok(), &aphel_variable_names());
    let mut sensitive = collected.sensitive_values.clone();
    sensitive.extend(sensitive_environment());

    let mut checks = Vec::new();
    let mut mysql = None;
    let mut migrations = None;
    let mut heartbeats = Vec::new();
    let mut enabled: HashSet<String> = HashSet::new();

    match config.database_url.as_deref() {
        None => checks.push(DoctorCheck::new(
            "database",
            DoctorStatus::Fail,
            "DATABASE_URL is not configured",
        )),
        Some(url) => match connect_quietly(url).await {
            Err(reason) => checks.push(DoctorCheck::new("database", DoctorStatus::Fail, reason)),
            Ok(pool) => {
                checks.push(DoctorCheck::new("database", DoctorStatus::Ok, "connected"));
                mysql = mysql_version(&pool).await;
                checks.push(mysql_check(mysql.as_deref()));
                match migration_status(&pool).await {
                    Ok(status) => {
                        checks.push(migrations_check(&status));
                        migrations = Some(status);
                    }
                    Err(_) => checks.push(DoctorCheck::new(
                        "migrations",
                        DoctorStatus::Fail,
                        "could not read the migration history",
                    )),
                }
                checks.extend(worker_checks(&pool, &settings).await);
                heartbeats = monitor::heartbeats(&pool).await.unwrap_or_default();
                enabled = enabled_modules(&pool).await.unwrap_or_default();
                checks.push(storage_check(&config, &pool).await);
                checks.extend(disk_space_checks(&pool, settings.min_free_disk_mb).await);
                checks.push(smtp_check(&pool).await);
                pool.close().await;
            }
        },
    }
    checks.push(encryption_check(&config));
    checks.push(DoctorCheck::new(
        "metrics endpoint",
        DoctorStatus::Ok,
        if settings.metrics_token.is_some() {
            "enabled (/metrics, bearer token)"
        } else {
            "disabled"
        },
    ));

    let report = DoctorReport {
        generated_at: Utc::now(),
        version: APHEL_VERSION.to_owned(),
        os: env::consts::OS.to_owned(),
        arch: env::consts::ARCH.to_owned(),
        mysql_version: mysql,
        settings: collected.lines,
        checks,
        migrations,
        modules: compiled_modules(&enabled),
        heartbeats,
    };
    print!("{}", render_markdown(&report, &sensitive));
    Ok(())
}
