use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use chrono_tz::Tz;
use sqlx::mysql::MySqlRow;
use sqlx::{MySqlPool, Row};
use uuid::Uuid;

use crate::domain::{ExistingRenewal, RenewalOwnerMode, RenewalSettings};
use aphel_kernel::Ctx;
use aphel_kernel::application::access::{AccessRepository, build_access_scope};
use aphel_kernel::database::transaction::MySqlTx;
use aphel_kernel::domain::UserId;
use aphel_kernel::domain::access::AccessScope;
use aphel_kernel::errors::AppError;
use aphel_kernel::infrastructure::repositories::mapper::parse_uuid;
use aphel_kernel::infrastructure::repositories::mapper::{invalid_column, naive, utc};
use aphel_kernel::infrastructure::repositories::mysql_access::MySqlAccessRepository;

macro_rules! settings_select {
    ($condition:literal) => {
        concat!(
            "SELECT pipeline_id, is_enabled, days_before, target_pipeline_id, target_stage_id, ",
            "title_prefix, owner_mode, updated_by, created_at, updated_at ",
            "FROM pipeline_renewal_settings WHERE TRUE ",
            $condition
        )
    };
}

const MAX_DUE_DEALS: i64 = 500;

fn settings_from_row(row: &MySqlRow) -> Result<RenewalSettings, AppError> {
    let owner_mode: String = row.try_get("owner_mode")?;
    Ok(RenewalSettings {
        pipeline_id: parse_uuid(
            &row.try_get::<String, _>("pipeline_id")?,
            "pipeline_renewal_settings.pipeline_id",
        )?,
        is_enabled: row.try_get("is_enabled")?,
        days_before: row.try_get("days_before")?,
        target_pipeline_id: parse_uuid(
            &row.try_get::<String, _>("target_pipeline_id")?,
            "pipeline_renewal_settings.target_pipeline_id",
        )?,
        target_stage_id: parse_uuid(
            &row.try_get::<String, _>("target_stage_id")?,
            "pipeline_renewal_settings.target_stage_id",
        )?,
        title_prefix: row.try_get("title_prefix")?,
        owner_mode: RenewalOwnerMode::parse(&owner_mode)
            .ok_or_else(|| invalid_column("pipeline_renewal_settings.owner_mode", &owner_mode))?,
        updated_by: row
            .try_get::<Option<String>, _>("updated_by")?
            .map(UserId::new)
            .transpose()?,
        created_at: utc(row.try_get::<NaiveDateTime, _>("created_at")?),
        updated_at: utc(row.try_get::<NaiveDateTime, _>("updated_at")?),
    })
}

#[derive(Clone)]
pub(crate) struct RenewalStore {
    pool: MySqlPool,
}

impl RenewalStore {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    pub(crate) fn of(ctx: &Ctx<'_>) -> Self {
        Self::new(ctx.pool().clone())
    }

    async fn settings_of(&self, only_enabled: bool) -> Result<Vec<RenewalSettings>, AppError> {
        let sql = if only_enabled {
            settings_select!("AND is_enabled = TRUE ORDER BY pipeline_id")
        } else {
            settings_select!("ORDER BY pipeline_id")
        };
        let rows = sqlx::query(sql).fetch_all(&self.pool).await?;
        rows.iter().map(settings_from_row).collect()
    }

    pub(crate) async fn find(
        &self,
        pipeline_id: Uuid,
    ) -> Result<Option<RenewalSettings>, AppError> {
        let row = sqlx::query(settings_select!("AND pipeline_id = ?"))
            .bind(pipeline_id.to_string())
            .fetch_optional(&self.pool)
            .await?;
        row.as_ref().map(settings_from_row).transpose()
    }

    pub(crate) async fn list(&self) -> Result<Vec<RenewalSettings>, AppError> {
        self.settings_of(false).await
    }

    pub(crate) async fn enabled(&self) -> Result<Vec<RenewalSettings>, AppError> {
        self.settings_of(true).await
    }

    pub(crate) async fn save(
        &self,
        tx: &mut MySqlTx,
        settings: &RenewalSettings,
    ) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO pipeline_renewal_settings (pipeline_id, is_enabled, \
             days_before, target_pipeline_id, target_stage_id, title_prefix, owner_mode, updated_by, \
             created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?) \
             ON DUPLICATE KEY UPDATE is_enabled = VALUES(is_enabled), \
             days_before = VALUES(days_before), target_pipeline_id = VALUES(target_pipeline_id), \
             target_stage_id = VALUES(target_stage_id), title_prefix = VALUES(title_prefix), \
             owner_mode = VALUES(owner_mode), updated_by = VALUES(updated_by), \
             updated_at = VALUES(updated_at)",
        )
        .bind(settings.pipeline_id.to_string())
        .bind(settings.is_enabled)
        .bind(settings.days_before)
        .bind(settings.target_pipeline_id.to_string())
        .bind(settings.target_stage_id.to_string())
        .bind(&settings.title_prefix)
        .bind(settings.owner_mode.as_str())
        .bind(settings.updated_by.as_ref().map(UserId::as_str))
        .bind(naive(settings.created_at))
        .bind(naive(settings.updated_at))
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub(crate) async fn timezone(&self) -> Result<Tz, AppError> {
        let timezone: Option<String> =
            sqlx::query_scalar("SELECT timezone FROM organization_settings WHERE id = 1")
                .fetch_optional(&self.pool)
                .await?;
        Ok(timezone
            .and_then(|timezone| timezone.parse::<Tz>().ok())
            .unwrap_or(chrono_tz::UTC))
    }

    pub(crate) async fn member_scope(
        &self,
        user_id: &UserId,
    ) -> Result<Option<AccessScope>, AppError> {
        let access = MySqlAccessRepository::new(self.pool.clone())
            .load_access(user_id)
            .await?;
        Ok(access.map(|access| build_access_scope(user_id.clone(), access)))
    }

    pub(crate) async fn due_deals(
        &self,
        settings: &RenewalSettings,
        today: NaiveDate,
    ) -> Result<Vec<Uuid>, AppError> {
        let ids: Vec<String> = sqlx::query_scalar(
            "SELECT d.id FROM deals d \
             WHERE d.pipeline_id = ? AND d.status = 'won' \
             AND d.deleted_at IS NULL AND d.contract_end IS NOT NULL AND d.contract_end >= ? \
             AND DATE_SUB(d.contract_end, INTERVAL ? DAY) <= ? \
             AND NOT EXISTS (SELECT 1 FROM deal_renewals r \
             WHERE r.original_deal_id = d.id) \
             ORDER BY d.contract_end, d.id LIMIT ?",
        )
        .bind(settings.pipeline_id.to_string())
        .bind(today)
        .bind(settings.days_before)
        .bind(today)
        .bind(MAX_DUE_DEALS)
        .fetch_all(&self.pool)
        .await?;
        ids.iter().map(|id| parse_uuid(id, "deals.id")).collect()
    }

    pub(crate) async fn find_renewal(
        &self,
        original_deal_id: Uuid,
    ) -> Result<Option<ExistingRenewal>, AppError> {
        let row = sqlx::query("SELECT id, deleted_at FROM deals WHERE renewal_of_deal_id = ?")
            .bind(original_deal_id.to_string())
            .fetch_optional(&self.pool)
            .await?;
        row.map(|row| {
            Ok(ExistingRenewal {
                deal_id: parse_uuid(&row.try_get::<String, _>("id")?, "deals.id")?,
                deleted: row
                    .try_get::<Option<NaiveDateTime>, _>("deleted_at")?
                    .is_some(),
            })
        })
        .transpose()
    }

    pub(crate) async fn mark_renewed(
        &self,
        tx: &mut MySqlTx,
        original_deal_id: Uuid,
        renewal_deal_id: Uuid,
        at: DateTime<Utc>,
    ) -> Result<bool, AppError> {
        let result = sqlx::query(
            "INSERT IGNORE INTO deal_renewals (original_deal_id, renewal_deal_id, created_at) \
             VALUES (?, ?, ?)",
        )
        .bind(original_deal_id.to_string())
        .bind(renewal_deal_id.to_string())
        .bind(naive(at))
        .execute(&mut **tx)
        .await?;
        Ok(result.rows_affected() == 1)
    }
}
