pub(crate) mod reference_cleanup;

use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use chrono_tz::Tz;
use sqlx::mysql::MySqlRow;
use sqlx::{MySqlPool, Row};
use uuid::Uuid;

use crate::domain::{ExistingRenewal, RenewalOwnerMode, RenewalSettings};
use aphel_kernel::Ctx;
use aphel_kernel::domain::UserId;
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::application::access::{AccessRepository, build_access_scope};
use aphel_sdk::platform::database::transaction::Tx;
use aphel_sdk::platform::domain::access::AccessScope;
use aphel_sdk::platform::infrastructure::repositories::mapper::parse_uuid;
use aphel_sdk::platform::infrastructure::repositories::mapper::{invalid_column, naive, utc};
use aphel_sdk::platform::infrastructure::repositories::mysql_access::MySqlAccessRepository;

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
        tx: &mut Tx,
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
        ctx: &Ctx<'_>,
        settings: &RenewalSettings,
        today: NaiveDate,
    ) -> Result<Vec<Uuid>, AppError> {
        let reader = ctx
            .services()
            .require::<dyn aphel_sdk::deals::DealRenewalReader>()?;
        let mut after = None;
        let mut due = Vec::new();
        while due.len() < MAX_DUE_DEALS as usize {
            let page = reader
                .due(
                    ctx,
                    aphel_sdk::deals::RenewalDueQuery {
                        pipeline_id: settings.pipeline_id,
                        today,
                        days_before: settings.days_before,
                        after,
                    },
                )
                .await?;
            if page.is_empty() {
                break;
            }
            after = page.last().copied();
            let mut query = sqlx::QueryBuilder::<sqlx::MySql>::new(
                "SELECT original_deal_id FROM deal_renewals WHERE original_deal_id IN (",
            );
            let mut list = query.separated(", ");
            for candidate in &page {
                list.push_bind(candidate.id.to_string());
            }
            query.push(")");
            let processed: std::collections::HashSet<String> = query
                .build_query_scalar()
                .fetch_all(&self.pool)
                .await?
                .into_iter()
                .collect();
            due.extend(
                page.iter()
                    .filter(|candidate| !processed.contains(&candidate.id.to_string()))
                    .map(|candidate| candidate.id)
                    .take(MAX_DUE_DEALS as usize - due.len()),
            );
            if page.len() < 100 {
                break;
            }
        }
        Ok(due)
    }

    pub(crate) async fn find_renewal(
        &self,
        ctx: &Ctx<'_>,
        original_deal_id: Uuid,
    ) -> Result<Option<ExistingRenewal>, AppError> {
        Ok(ctx
            .services()
            .require::<dyn aphel_sdk::deals::DealRenewalReader>()?
            .existing(ctx, original_deal_id)
            .await?
            .map(|record| ExistingRenewal {
                deal_id: record.deal_id,
                deleted: record.deleted,
            }))
    }

    pub(crate) async fn mark_renewed(
        &self,
        tx: &mut Tx,
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
