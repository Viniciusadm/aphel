use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::MySqlPool;
use sqlx::types::Json;
use uuid::Uuid;

use aphel_kernel::context::Ctx;
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::database::transaction::MySqlTx;
use aphel_sdk::platform::domain::access::AccessScope;
use aphel_sdk::platform::infrastructure::repositories::mapper::naive;

#[derive(Clone)]
pub(crate) struct LayoutStore {
    pool: MySqlPool,
}

impl LayoutStore {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    pub(crate) fn of(ctx: &Ctx<'_>) -> Self {
        Self::new(ctx.pool().clone())
    }

    pub(crate) async fn find(&self, scope: &AccessScope) -> Result<Option<Value>, AppError> {
        let layout: Option<Json<Value>> =
            sqlx::query_scalar("SELECT layout FROM dashboard_layouts WHERE user_id = ?")
                .bind(scope.user_id().as_str())
                .fetch_optional(&self.pool)
                .await?;
        Ok(layout.map(|Json(value)| value))
    }

    pub(crate) async fn save(
        &self,
        scope: &AccessScope,
        layout: &Value,
        at: DateTime<Utc>,
    ) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO dashboard_layouts (id, user_id, layout, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?) \
             ON DUPLICATE KEY UPDATE layout = ?, updated_at = ?",
        )
        .bind(Uuid::now_v7().to_string())
        .bind(scope.user_id().as_str())
        .bind(Json(layout))
        .bind(naive(at))
        .bind(naive(at))
        .bind(Json(layout))
        .bind(naive(at))
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub(crate) async fn delete(&self, scope: &AccessScope) -> Result<(), AppError> {
        sqlx::query("DELETE FROM dashboard_layouts WHERE user_id = ?")
            .bind(scope.user_id().as_str())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub(crate) async fn find_default(&self) -> Result<Option<Value>, AppError> {
        let layout: Option<Json<Value>> =
            sqlx::query_scalar("SELECT layout FROM dashboard_default_layouts")
                .fetch_optional(&self.pool)
                .await?;
        Ok(layout.map(|Json(value)| value))
    }

    pub(crate) async fn save_default(
        &self,
        tx: &mut MySqlTx,
        layout: &Value,
        at: DateTime<Utc>,
    ) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO dashboard_default_layouts (id, layout, created_at, updated_at) \
             VALUES (1, ?, ?, ?) \
             ON DUPLICATE KEY UPDATE layout = ?, updated_at = ?",
        )
        .bind(Json(layout))
        .bind(naive(at))
        .bind(naive(at))
        .bind(Json(layout))
        .bind(naive(at))
        .execute(&mut **tx)
        .await?;
        Ok(())
    }
}
