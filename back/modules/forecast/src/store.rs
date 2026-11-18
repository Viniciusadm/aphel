use chrono::{DateTime, NaiveDateTime, Utc};
use sqlx::mysql::MySqlRow;
use sqlx::{MySqlPool, Row};

use crate::domain::{ForecastMethod, ForecastSettings};
use aphel_kernel::Ctx;
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::database::transaction::Tx;
use aphel_sdk::platform::domain::access::AccessScope;
use aphel_sdk::platform::infrastructure::repositories::mapper;

fn settings_from_row(row: &MySqlRow) -> Result<ForecastSettings, AppError> {
    let method: String = row.try_get("method")?;
    let updated_at: NaiveDateTime = row.try_get("updated_at")?;
    Ok(ForecastSettings {
        method: ForecastMethod::parse(&method)
            .ok_or_else(|| mapper::invalid_column("method", &method))?,
        categories_enabled: row.try_get("categories_enabled")?,
        best_case_min_probability: row.try_get("best_case_min_probability")?,
        commit_min_probability: row.try_get("commit_min_probability")?,
        horizon_months: row.try_get("horizon_months")?,
        updated_at: Some(mapper::utc(updated_at)),
    })
}

#[derive(Clone)]
pub(crate) struct ForecastStore {
    pool: MySqlPool,
}

impl ForecastStore {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    pub(crate) fn of(ctx: &Ctx<'_>) -> Self {
        Self::new(ctx.pool().clone())
    }

    pub(crate) async fn currency(&self) -> Result<String, AppError> {
        let currency: Option<String> =
            sqlx::query_scalar("SELECT currency FROM organization_settings WHERE id = 1")
                .fetch_optional(&self.pool)
                .await?;
        Ok(currency.unwrap_or_else(|| "BRL".to_owned()))
    }

    pub(crate) async fn settings(&self) -> Result<Option<ForecastSettings>, AppError> {
        sqlx::query(
            "SELECT method, categories_enabled, best_case_min_probability, commit_min_probability, \
             horizon_months, updated_at FROM forecast_settings",
        )
        .fetch_optional(&self.pool)
        .await?
        .as_ref()
        .map(settings_from_row)
        .transpose()
    }

    pub(crate) async fn save_settings(
        &self,
        tx: &mut Tx,
        scope: &AccessScope,
        settings: &ForecastSettings,
        at: DateTime<Utc>,
    ) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO forecast_settings (id, method, categories_enabled, \
             best_case_min_probability, commit_min_probability, horizon_months, updated_by, \
             created_at, updated_at) VALUES (1, ?, ?, ?, ?, ?, ?, ?, ?) \
             ON DUPLICATE KEY UPDATE method = VALUES(method), \
             categories_enabled = VALUES(categories_enabled), \
             best_case_min_probability = VALUES(best_case_min_probability), \
             commit_min_probability = VALUES(commit_min_probability), \
             horizon_months = VALUES(horizon_months), updated_by = VALUES(updated_by), \
             updated_at = VALUES(updated_at)",
        )
        .bind(settings.method.as_str())
        .bind(settings.categories_enabled)
        .bind(settings.best_case_min_probability)
        .bind(settings.commit_min_probability)
        .bind(settings.horizon_months)
        .bind(scope.user_id().as_str())
        .bind(mapper::naive(at))
        .bind(mapper::naive(at))
        .execute(&mut **tx)
        .await?;
        Ok(())
    }
}
