use std::time::Duration;

use sqlx::mysql::{MySqlConnectOptions, MySqlPoolOptions};
use sqlx::{Connection, MySqlPool};

const IDLE_BEFORE_PING: Duration = Duration::from_secs(30);

pub async fn connect(database_url: &str) -> Result<MySqlPool, sqlx::Error> {
    MySqlPoolOptions::new()
        .max_connections(30)
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
