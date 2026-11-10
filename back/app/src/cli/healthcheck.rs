use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::time::Duration;

use chrono::Utc;
use clap::Args;

use crate::cli::doctor::connect_quietly;
use crate::cli::{AlreadyReported, CliResult};
use aphel_kernel::application::system::WORKER_ROLE;
use aphel_kernel::application::system::health::age_seconds;
use aphel_kernel::config::AppConfig;
use aphel_kernel::infrastructure::system::{SystemSettings, hostname, probe};

const REQUEST_TIMEOUT: Duration = Duration::from_secs(3);

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Args)]
pub(crate) struct HealthcheckArgs {
    #[arg(
        long,
        conflicts_with = "worker",
        help = "Check /_aphel/health/ready (database, storage, worker and scheduler) instead of /_aphel/health"
    )]
    pub(crate) ready: bool,
    #[arg(
        long,
        help = "Check the heartbeat of the worker running on this host instead of the HTTP server"
    )]
    pub(crate) worker: bool,
}

pub(crate) async fn run(config: AppConfig, args: HealthcheckArgs) -> CliResult {
    let outcome = if args.worker {
        check_worker(&config).await
    } else {
        check_http(&config, args.ready).await
    };
    match outcome {
        Ok(message) => {
            println!("healthy: {message}");
            Ok(())
        }
        Err(message) => {
            eprintln!("unhealthy: {message}");
            Err(Box::new(AlreadyReported))
        }
    }
}

fn probe_host(host: IpAddr) -> IpAddr {
    match host {
        IpAddr::V4(address) if address.is_unspecified() => IpAddr::V4(Ipv4Addr::LOCALHOST),
        IpAddr::V6(address) if address.is_unspecified() => IpAddr::V6(Ipv6Addr::LOCALHOST),
        other => other,
    }
}

async fn check_http(config: &AppConfig, ready: bool) -> Result<String, String> {
    let address = std::net::SocketAddr::new(probe_host(config.host), config.port);
    let path = if ready {
        "/_aphel/health/ready"
    } else {
        "/_aphel/health"
    };
    let url = format!("http://{address}{path}");
    let client = reqwest::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .connect_timeout(REQUEST_TIMEOUT)
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|_| "could not build the HTTP client".to_owned())?;
    match client.get(&url).send().await {
        Ok(response) if response.status().is_success() => {
            Ok(format!("{url} answered {}", response.status().as_u16()))
        }
        Ok(response) => Err(format!("{url} answered {}", response.status().as_u16())),
        Err(error) if error.is_timeout() => Err(format!("{url} timed out")),
        Err(_) => Err(format!("{url} is not reachable")),
    }
}

async fn check_worker(config: &AppConfig) -> Result<String, String> {
    let database_url = config
        .database_url
        .as_deref()
        .ok_or_else(|| "DATABASE_URL is not configured".to_owned())?;
    let max_silence = SystemSettings::from_env().readiness.worker_max_silence;
    let pool = connect_quietly(database_url).await?;
    let host = hostname();
    let latest = match &host {
        Some(host) => probe::latest_heartbeat_on_host(&pool, WORKER_ROLE, host).await,
        None => probe::latest_heartbeat(&pool, WORKER_ROLE).await,
    };
    pool.close().await;
    let scope = host.map_or_else(|| "any host".to_owned(), |host| format!("host {host}"));
    match latest {
        Ok(Some(seen_at)) => {
            let age = age_seconds(Utc::now(), seen_at);
            if age > max_silence.as_secs() as i64 {
                Err(format!(
                    "the worker on {scope} was last seen {age}s ago (limit {}s)",
                    max_silence.as_secs()
                ))
            } else {
                Ok(format!("the worker on {scope} was seen {age}s ago"))
            }
        }
        Ok(None) => Err(format!("no worker heartbeat recorded for {scope}")),
        Err(_) => Err("could not read the worker heartbeats".to_owned()),
    }
}
