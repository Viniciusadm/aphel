use sqlx::pool::PoolConnection;
use sqlx::{MySql, MySqlPool};

use aphel_kernel::Ctx;
use aphel_kernel::errors::AppError;

#[derive(Clone)]
pub(crate) struct AutomationLocks {
    pool: MySqlPool,
}

impl AutomationLocks {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    pub(crate) fn of(ctx: &Ctx<'_>) -> Self {
        Self::new(ctx.pool().clone())
    }

    pub(crate) async fn acquire(
        &self,
        name: &str,
        wait_seconds: u32,
    ) -> Result<Option<NamedLock>, AppError> {
        let mut connection = self.pool.acquire().await?;
        let acquired: Option<i64> = sqlx::query_scalar("SELECT GET_LOCK(?, ?)")
            .bind(name)
            .bind(wait_seconds)
            .fetch_one(&mut *connection)
            .await?;
        if acquired != Some(1) {
            return Ok(None);
        }
        Ok(Some(NamedLock {
            name: name.to_owned(),
            connection: Some(connection),
        }))
    }
}

pub(crate) struct NamedLock {
    name: String,
    connection: Option<PoolConnection<MySql>>,
}

impl NamedLock {
    pub(crate) async fn release(mut self) -> Result<(), AppError> {
        let Some(mut connection) = self.connection.take() else {
            return Ok(());
        };
        let released: Result<Option<i64>, sqlx::Error> =
            sqlx::query_scalar("SELECT RELEASE_LOCK(?)")
                .bind(&self.name)
                .fetch_one(&mut *connection)
                .await;
        match released {
            Ok(_) => Ok(()),
            Err(error) => {
                drop(connection.detach());
                Err(error.into())
            }
        }
    }
}

impl Drop for NamedLock {
    fn drop(&mut self) {
        if let Some(connection) = self.connection.take() {
            drop(connection.detach());
        }
    }
}
