use sqlx::{MySqlPool, Row};

use crate::DirectoryMember;
use aphel_kernel::context::Ctx;
use aphel_kernel::errors::AppError;

#[derive(Clone)]
pub(crate) struct MemberStore {
    pool: MySqlPool,
}

impl MemberStore {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    pub(crate) fn of(ctx: &Ctx<'_>) -> Self {
        Self::new(ctx.pool().clone())
    }

    pub(crate) async fn active_members(&self) -> Result<Vec<DirectoryMember>, AppError> {
        sqlx::query("SELECT display_name, email FROM users WHERE status = 'active'")
            .fetch_all(&self.pool)
            .await?
            .iter()
            .map(|row| {
                Ok(DirectoryMember {
                    display_name: row.try_get("display_name")?,
                    email: row.try_get("email")?,
                })
            })
            .collect()
    }
}
