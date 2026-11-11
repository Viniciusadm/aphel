use std::env;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::Duration;

use crate::domain::Secret;
use crate::errors::AppError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppConfig {
    pub host: IpAddr,
    pub port: u16,
    pub database_url: Option<String>,
    pub control_database_url: Option<String>,
    pub console_host: Option<String>,
    pub tenant_max_connections: u32,
    pub tenant_export_dir: String,
    pub run_migrations: bool,
    pub worker: WorkerConfig,
    pub encryption: EncryptionConfig,
    pub outbound_http: OutboundHttpConfig,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutboundHttpConfig {
    pub allow_private_networks: bool,
    pub require_https: bool,
    pub timeout: Duration,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EncryptionConfig {
    pub key: Option<Secret>,
    pub previous_keys: Vec<Secret>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkerConfig {
    pub concurrency: usize,
    pub poll_interval: Duration,
    pub job_timeout: Duration,
    pub scheduler_interval: Duration,
}

impl AppConfig {
    pub fn from_env() -> Self {
        let host = env::var("HTTP_HOST")
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(IpAddr::V4(Ipv4Addr::LOCALHOST));
        let port = env::var("HTTP_PORT")
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(3001);
        let database_url = non_empty_var("DATABASE_URL");
        let run_migrations = env::var("RUN_MIGRATIONS")
            .map(|value| !is_falsy(&value))
            .unwrap_or(true);

        Self {
            host,
            port,
            database_url,
            control_database_url: non_empty_var("CONTROL_DATABASE_URL"),
            console_host: non_empty_var("CONSOLE_HOST"),
            tenant_max_connections: parsed_var("TENANT_DATABASE_MAX_CONNECTIONS")
                .filter(|value: &u32| *value > 0)
                .unwrap_or(10),
            tenant_export_dir: non_empty_var("TENANT_EXPORT_DIR")
                .unwrap_or_else(|| "storage/tenant-exports".to_owned()),
            run_migrations,
            worker: WorkerConfig::from_env(),
            encryption: EncryptionConfig::from_env(),
            outbound_http: OutboundHttpConfig::from_env(),
        }
    }

    pub fn http_addr(&self) -> SocketAddr {
        SocketAddr::new(self.host, self.port)
    }

    pub fn tenancy(&self) -> Result<Tenancy, AppError> {
        match (&self.database_url, &self.control_database_url) {
            (Some(_), Some(_)) => Err(AppError::Configuration(
                "DATABASE_URL and CONTROL_DATABASE_URL are mutually exclusive: set DATABASE_URL for a single organization or CONTROL_DATABASE_URL to serve many".to_owned(),
            )),
            (_, Some(control_database_url)) => Ok(Tenancy::Multi(MultiTenancyConfig {
                control_database_url: control_database_url.clone(),
                console_host: self.console_host.clone(),
                tenant_max_connections: self.tenant_max_connections,
                tenant_export_dir: self.tenant_export_dir.clone(),
            })),
            (_, None) if self.console_host.is_some() => Err(AppError::Configuration(
                "CONSOLE_HOST requires CONTROL_DATABASE_URL".to_owned(),
            )),
            (_, None) => Ok(Tenancy::Single),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Tenancy {
    Single,
    Multi(MultiTenancyConfig),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MultiTenancyConfig {
    pub control_database_url: String,
    pub console_host: Option<String>,
    pub tenant_max_connections: u32,
    pub tenant_export_dir: String,
}

impl EncryptionConfig {
    pub fn from_env() -> Self {
        Self {
            key: non_empty_var("APP_ENCRYPTION_KEY").map(Secret::new),
            previous_keys: non_empty_var("APP_ENCRYPTION_PREVIOUS_KEYS")
                .map(|value| {
                    value
                        .split(',')
                        .map(str::trim)
                        .filter(|key| !key.is_empty())
                        .map(Secret::new)
                        .collect()
                })
                .unwrap_or_default(),
        }
    }
}

impl OutboundHttpConfig {
    pub fn from_env() -> Self {
        Self {
            allow_private_networks: env::var("WEBHOOKS_ALLOW_PRIVATE_NETWORKS")
                .map(|value| !is_falsy(&value))
                .unwrap_or(false),
            ..Self::default()
        }
    }
}

impl Default for OutboundHttpConfig {
    fn default() -> Self {
        Self {
            allow_private_networks: false,
            require_https: false,
            timeout: Duration::from_secs(10),
        }
    }
}

impl WorkerConfig {
    pub fn from_env() -> Self {
        Self {
            concurrency: parsed_var("WORKER_CONCURRENCY")
                .filter(|value: &usize| *value > 0)
                .unwrap_or(4),
            poll_interval: Duration::from_millis(
                parsed_var("WORKER_POLL_INTERVAL_MS").unwrap_or(1000),
            ),
            job_timeout: Duration::from_secs(parsed_var("WORKER_JOB_TIMEOUT_SECS").unwrap_or(300)),
            scheduler_interval: Duration::from_secs(
                parsed_var("SCHEDULER_INTERVAL_SECS").unwrap_or(5),
            ),
        }
    }
}

fn non_empty_var(name: &str) -> Option<String> {
    env::var(name)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

fn parsed_var<T: std::str::FromStr>(name: &str) -> Option<T> {
    non_empty_var(name).and_then(|value| value.parse().ok())
}

fn is_falsy(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "0" | "false" | "no" | "off"
    )
}
