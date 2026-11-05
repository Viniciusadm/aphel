use chrono::NaiveDateTime;
use sqlx::mysql::MySqlRow;
use sqlx::{MySqlPool, Row};
use uuid::Uuid;

use crate::guidance::domain::{ChecklistItem, ChecklistProgress, GuidanceText};
use aphel_kernel::Ctx;
use aphel_kernel::domain::UserId;
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::database::transaction::MySqlTx;
use aphel_sdk::platform::infrastructure::repositories::mapper::{naive, parse_uuid, utc};

macro_rules! item_select {
    ($condition:literal) => {
        concat!(
            "SELECT id, stage_id, `key`, label, is_required, position, created_at, updated_at ",
            "FROM workflow_stage_checklist_items ",
            $condition
        )
    };
}

macro_rules! guidance_select {
    ($condition:literal) => {
        concat!(
            "SELECT stage_id, guidance, updated_by, created_at, updated_at ",
            "FROM workflow_stage_guidance ",
            $condition
        )
    };
}

fn optional_user(row: &MySqlRow, column: &str) -> Result<Option<UserId>, AppError> {
    row.try_get::<Option<String>, _>(column)?
        .map(UserId::new)
        .transpose()
        .map_err(Into::into)
}

fn item_from_row(row: &MySqlRow) -> Result<ChecklistItem, AppError> {
    Ok(ChecklistItem {
        id: parse_uuid(
            &row.try_get::<String, _>("id")?,
            "workflow_stage_checklist_items.id",
        )?,
        stage_id: parse_uuid(
            &row.try_get::<String, _>("stage_id")?,
            "workflow_stage_checklist_items.stage_id",
        )?,
        key: row.try_get("key")?,
        label: row.try_get("label")?,
        is_required: row.try_get("is_required")?,
        position: row.try_get("position")?,
        created_at: utc(row.try_get::<NaiveDateTime, _>("created_at")?),
        updated_at: utc(row.try_get::<NaiveDateTime, _>("updated_at")?),
    })
}

fn guidance_from_row(row: &MySqlRow) -> Result<GuidanceText, AppError> {
    Ok(GuidanceText {
        stage_id: parse_uuid(
            &row.try_get::<String, _>("stage_id")?,
            "workflow_stage_guidance.stage_id",
        )?,
        guidance: row.try_get("guidance")?,
        updated_by: optional_user(row, "updated_by")?,
        created_at: utc(row.try_get::<NaiveDateTime, _>("created_at")?),
        updated_at: utc(row.try_get::<NaiveDateTime, _>("updated_at")?),
    })
}

#[derive(Clone)]
pub(crate) struct GuidanceStore {
    pool: MySqlPool,
}

impl GuidanceStore {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    pub(crate) fn of(ctx: &Ctx<'_>) -> Self {
        Self::new(ctx.pool().clone())
    }

    pub(crate) async fn guidance(&self, stage_id: Uuid) -> Result<Option<GuidanceText>, AppError> {
        let row = sqlx::query(guidance_select!("WHERE stage_id = ?"))
            .bind(stage_id.to_string())
            .fetch_optional(&self.pool)
            .await?;
        row.as_ref().map(guidance_from_row).transpose()
    }

    pub(crate) async fn list_guidance(&self) -> Result<Vec<GuidanceText>, AppError> {
        let rows = sqlx::query(guidance_select!("ORDER BY stage_id"))
            .fetch_all(&self.pool)
            .await?;
        rows.iter().map(guidance_from_row).collect()
    }

    pub(crate) async fn items(&self, stage_id: Uuid) -> Result<Vec<ChecklistItem>, AppError> {
        let rows = sqlx::query(item_select!(
            "WHERE stage_id = ? ORDER BY position, created_at, id"
        ))
        .bind(stage_id.to_string())
        .fetch_all(&self.pool)
        .await?;
        rows.iter().map(item_from_row).collect()
    }

    pub(crate) async fn all_items(&self) -> Result<Vec<ChecklistItem>, AppError> {
        let rows = sqlx::query(item_select!("ORDER BY stage_id, position, created_at, id"))
            .fetch_all(&self.pool)
            .await?;
        rows.iter().map(item_from_row).collect()
    }

    pub(crate) async fn find_item(&self, item_id: Uuid) -> Result<Option<ChecklistItem>, AppError> {
        let row = sqlx::query(item_select!("WHERE id = ?"))
            .bind(item_id.to_string())
            .fetch_optional(&self.pool)
            .await?;
        row.as_ref().map(item_from_row).transpose()
    }

    pub(crate) async fn save_guidance(
        &self,
        tx: &mut MySqlTx,
        guidance: &GuidanceText,
    ) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO workflow_stage_guidance (stage_id, guidance, updated_by, \
             created_at, updated_at) VALUES (?, ?, ?, ?, ?) \
             ON DUPLICATE KEY UPDATE guidance = VALUES(guidance), updated_by = VALUES(updated_by), \
             updated_at = VALUES(updated_at)",
        )
        .bind(guidance.stage_id.to_string())
        .bind(&guidance.guidance)
        .bind(guidance.updated_by.as_ref().map(UserId::as_str))
        .bind(naive(guidance.created_at))
        .bind(naive(guidance.updated_at))
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub(crate) async fn insert_item(
        &self,
        tx: &mut MySqlTx,
        item: &ChecklistItem,
    ) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO workflow_stage_checklist_items (id, stage_id, `key`, \
             label, is_required, position, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(item.id.to_string())
        .bind(item.stage_id.to_string())
        .bind(&item.key)
        .bind(&item.label)
        .bind(item.is_required)
        .bind(item.position)
        .bind(naive(item.created_at))
        .bind(naive(item.updated_at))
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub(crate) async fn update_item(
        &self,
        tx: &mut MySqlTx,
        item: &ChecklistItem,
    ) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE workflow_stage_checklist_items SET `key` = ?, label = ?, is_required = ?, \
             position = ?, updated_at = ? WHERE id = ?",
        )
        .bind(&item.key)
        .bind(&item.label)
        .bind(item.is_required)
        .bind(item.position)
        .bind(naive(item.updated_at))
        .bind(item.id.to_string())
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub(crate) async fn delete_item(
        &self,
        tx: &mut MySqlTx,
        item_id: Uuid,
    ) -> Result<(), AppError> {
        sqlx::query("DELETE FROM workflow_stage_checklist_items WHERE id = ?")
            .bind(item_id.to_string())
            .execute(&mut **tx)
            .await?;
        Ok(())
    }

    pub(crate) async fn progress(
        &self,
        record_type: &aphel_sdk::platform::application::shared::RecordType,
        record_id: Uuid,
    ) -> Result<Vec<ChecklistProgress>, AppError> {
        let rows = sqlx::query(
            "SELECT p.item_id, p.done_by, u.display_name, p.done_at \
             FROM workflow_checklist_progress p LEFT JOIN users u ON u.id = p.done_by \
             WHERE p.record_type = ? AND p.record_id = ? ORDER BY p.done_at",
        )
        .bind(record_type.as_str())
        .bind(record_id.to_string())
        .fetch_all(&self.pool)
        .await?;
        rows.iter()
            .map(|row| {
                Ok(ChecklistProgress {
                    item_id: parse_uuid(
                        &row.try_get::<String, _>("item_id")?,
                        "workflow_checklist_progress.item_id",
                    )?,
                    done_by: optional_user(row, "done_by")?,
                    done_by_name: row.try_get("display_name")?,
                    done_at: utc(row.try_get::<NaiveDateTime, _>("done_at")?),
                })
            })
            .collect()
    }

    pub(crate) async fn mark_done(
        &self,
        tx: &mut MySqlTx,
        record_type: &aphel_sdk::platform::application::shared::RecordType,
        record_id: Uuid,
        progress: &ChecklistProgress,
    ) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO workflow_checklist_progress (record_type, record_id, item_id, done_by, \
             done_at) VALUES (?, ?, ?, ?, ?) \
             ON DUPLICATE KEY UPDATE done_by = VALUES(done_by), done_at = VALUES(done_at)",
        )
        .bind(record_type.as_str())
        .bind(record_id.to_string())
        .bind(progress.item_id.to_string())
        .bind(progress.done_by.as_ref().map(UserId::as_str))
        .bind(naive(progress.done_at))
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub(crate) async fn mark_pending(
        &self,
        tx: &mut MySqlTx,
        record_type: &aphel_sdk::platform::application::shared::RecordType,
        record_id: Uuid,
        item_id: Uuid,
    ) -> Result<(), AppError> {
        sqlx::query(
            "DELETE FROM workflow_checklist_progress \
             WHERE record_type = ? AND record_id = ? AND item_id = ?",
        )
        .bind(record_type.as_str())
        .bind(record_id.to_string())
        .bind(item_id.to_string())
        .execute(&mut **tx)
        .await?;
        Ok(())
    }
}
