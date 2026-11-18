use chrono::{DateTime, NaiveDateTime, Utc};
use sqlx::mysql::MySqlRow;
use sqlx::{MySql, MySqlPool, QueryBuilder, Row};
use uuid::Uuid;

use crate::tracking::{NewTrackingObjection, TrackingObjection, TrackingSettings};
use aphel_kernel::Ctx;
use aphel_kernel::domain::UserId;
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::database::transaction::Tx;
use aphel_sdk::platform::infrastructure::fields::mysql_definitions::uuid_column;
use aphel_sdk::platform::infrastructure::repositories::mapper;

fn objection_from_row(row: &MySqlRow) -> Result<TrackingObjection, AppError> {
    let created_by: Option<String> = row.try_get("created_by")?;
    let created_at: NaiveDateTime = row.try_get("created_at")?;
    Ok(TrackingObjection {
        id: uuid_column(row, "id")?,
        label: row.try_get("label")?,
        reason: row.try_get("reason")?,
        created_by: created_by
            .map(|value| mapper::typed_id(value, UserId::new))
            .transpose()?,
        created_at: mapper::utc(created_at),
    })
}

#[derive(Clone)]
pub(crate) struct TrackingStore {
    pool: MySqlPool,
}

impl TrackingStore {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    pub(crate) fn of(ctx: &Ctx<'_>) -> Self {
        Self::new(ctx.pool().clone())
    }
}

impl TrackingStore {
    pub(crate) async fn settings(&self) -> Result<TrackingSettings, AppError> {
        let row = sqlx::query(
            "SELECT opens_enabled, clicks_enabled, default_on FROM email_tracking_settings \
             WHERE id = 1",
        )
        .fetch_optional(&self.pool)
        .await?;
        let Some(row) = row else {
            return Ok(TrackingSettings::default());
        };
        Ok(TrackingSettings {
            opens_enabled: row.try_get("opens_enabled")?,
            clicks_enabled: row.try_get("clicks_enabled")?,
            default_on: row.try_get("default_on")?,
        })
    }

    pub(crate) async fn save_settings(
        &self,
        tx: &mut Tx,
        settings: &TrackingSettings,
        updated_by: &UserId,
        now: DateTime<Utc>,
    ) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO email_tracking_settings (id, opens_enabled, clicks_enabled, default_on, updated_by, updated_at) \
             VALUES (1, ?, ?, ?, ?, ?) \
             ON DUPLICATE KEY UPDATE opens_enabled = VALUES(opens_enabled), clicks_enabled = VALUES(clicks_enabled), \
             default_on = VALUES(default_on), updated_by = VALUES(updated_by), updated_at = VALUES(updated_at)",
        )
        .bind(settings.opens_enabled)
        .bind(settings.clicks_enabled)
        .bind(settings.default_on)
        .bind(updated_by.as_str())
        .bind(mapper::naive(now))
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub(crate) async fn objections(&self) -> Result<Vec<TrackingObjection>, AppError> {
        let rows = sqlx::query(
            "SELECT id, label, reason, created_by, created_at FROM email_tracking_objections \
             ORDER BY created_at DESC, id DESC",
        )
        .fetch_all(&self.pool)
        .await?;
        rows.iter().map(objection_from_row).collect()
    }

    pub(crate) async fn find_objection(
        &self,
        id: Uuid,
    ) -> Result<Option<TrackingObjection>, AppError> {
        let row = sqlx::query(
            "SELECT id, label, reason, created_by, created_at FROM email_tracking_objections \
             WHERE id = ?",
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await?;
        row.as_ref().map(objection_from_row).transpose()
    }

    pub(crate) async fn insert_objection(
        &self,
        tx: &mut Tx,
        objection: &NewTrackingObjection,
        now: DateTime<Utc>,
    ) -> Result<bool, AppError> {
        let result = sqlx::query(
            "INSERT IGNORE INTO email_tracking_objections (id, address_hash, label, reason, created_by, created_at) \
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(objection.id.to_string())
        .bind(&objection.address_hash)
        .bind(&objection.label)
        .bind(&objection.reason)
        .bind(objection.created_by.as_ref().map(|user| user.as_str().to_owned()))
        .bind(mapper::naive(now))
        .execute(&mut **tx)
        .await?;
        Ok(result.rows_affected() == 1)
    }

    pub(crate) async fn delete_objection(&self, tx: &mut Tx, id: Uuid) -> Result<bool, AppError> {
        let result = sqlx::query("DELETE FROM email_tracking_objections WHERE id = ?")
            .bind(id.to_string())
            .execute(&mut **tx)
            .await?;
        Ok(result.rows_affected() == 1)
    }

    pub(crate) async fn has_objection(&self, address_hashes: &[String]) -> Result<bool, AppError> {
        if address_hashes.is_empty() {
            return Ok(false);
        }
        let mut builder = QueryBuilder::<MySql>::new(
            "SELECT 1 FROM email_tracking_objections WHERE address_hash IN (",
        );
        let mut separated = builder.separated(", ");
        for hash in address_hashes {
            separated.push_bind(hash.clone());
        }
        separated.push_unseparated(") LIMIT 1");
        let found: Option<i32> = builder
            .build_query_scalar()
            .fetch_optional(&self.pool)
            .await?;
        Ok(found.is_some())
    }
}
