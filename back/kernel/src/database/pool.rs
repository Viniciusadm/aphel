use std::time::Duration;

use sqlx::mysql::{MySqlConnectOptions, MySqlPoolOptions};
use sqlx::{Connection, MySqlPool};

const IDLE_BEFORE_PING: Duration = Duration::from_secs(30);
const IDLE_BEFORE_CLOSE: Duration = Duration::from_secs(60);
const CONNECTION_LIFETIME: Duration = Duration::from_secs(300);
const BASE_CONNECTIONS: u32 = 30;
const CONNECTIONS_BESIDES_WORKER: u32 = 6;

fn max_connections() -> u32 {
    let worker =
        u32::try_from(crate::config::WorkerConfig::from_env().concurrency).unwrap_or(u32::MAX);
    BASE_CONNECTIONS.max(worker.saturating_add(CONNECTIONS_BESIDES_WORKER))
}

pub async fn connect(database_url: &str) -> Result<MySqlPool, sqlx::Error> {
    MySqlPoolOptions::new()
        .max_connections(max_connections())
        .idle_timeout(IDLE_BEFORE_CLOSE)
        .max_lifetime(CONNECTION_LIFETIME)
        .acquire_timeout(Duration::from_secs(10))
        .test_before_acquire(false)
        .before_acquire(|connection, metadata| {
            Box::pin(async move {
                if metadata.idle_for > IDLE_BEFORE_PING {
                    connection.ping().await?;
                }
                Ok(true)
            })
        })
        .connect_with(
            database_url
                .parse::<MySqlConnectOptions>()?
                .collation("utf8mb4_unicode_ci"),
        )
        .await
}
