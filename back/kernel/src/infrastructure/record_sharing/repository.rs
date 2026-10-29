use async_trait::async_trait;
use chrono::{DateTime, NaiveDateTime, Utc};
use sqlx::mysql::MySqlRow;
use sqlx::{MySqlPool, Row};
use uuid::Uuid;

use crate::application::files::UserReference;
use crate::application::record_sharing::{
    NewRecordShare, RECORD_SHARE_EXISTS, RecordShare, RecordShareRepository, ShareAccess,
    ShareGrantee, ShareGranteeType,
};
use crate::application::shared::RecordRef;
use crate::database::transaction::MySqlTx;
use crate::domain::UserId;
use crate::domain::users::UserStatus;
use crate::errors::AppError;
use crate::infrastructure::fields::mysql_definitions::uuid_column;
use crate::infrastructure::repositories::mapper;

const SHARE_SELECT: &str = "SELECT s.id, s.record_type, s.record_id, s.grantee_type, s.grantee_id, \
     s.access, s.shared_by, sb.display_name AS shared_by_name, s.created_at, s.updated_at, \
     CASE s.grantee_type WHEN 'user' THEN gu.display_name ELSE gt.name END AS grantee_name \
     FROM record_shares s \
     LEFT JOIN users gu ON s.grantee_type = 'user' AND gu.id = s.grantee_id \
     LEFT JOIN teams gt ON s.grantee_type = 'team' AND gt.id = s.grantee_id \
     LEFT JOIN users sb ON sb.id = s.shared_by \
     WHERE s.record_type = ? AND s.record_id = ? \
     ORDER BY s.created_at, s.id";

#[derive(Clone)]
pub struct MySqlRecordShareRepository {
    pool: MySqlPool,
}

impl MySqlRecordShareRepository {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

fn share_from_row(row: &MySqlRow) -> Result<RecordShare, AppError> {
    let record_type: String = row.try_get("record_type")?;
    let record_id: String = row.try_get("record_id")?;
    let grantee_type: String = row.try_get("grantee_type")?;
    let access: String = row.try_get("access")?;
    let shared_by: Option<String> = row.try_get("shared_by")?;
    let shared_by_name: Option<String> = row.try_get("shared_by_name")?;
    let created_at: NaiveDateTime = row.try_get("created_at")?;
    let updated_at: NaiveDateTime = row.try_get("updated_at")?;
    let shared_by = match (shared_by, shared_by_name) {
        (Some(id), Some(display_name)) => Some(UserReference {
            id: UserId::new(id)?,
            display_name,
        }),
        _ => None,
    };
    Ok(RecordShare {
        id: uuid_column(row, "id")?,
        record: RecordRef::parse(&record_type, &record_id)?,
        grantee: ShareGrantee {
            kind: ShareGranteeType::parse(&grantee_type)
                .ok_or_else(|| mapper::invalid_column("grantee_type", &grantee_type))?,
            id: row.try_get("grantee_id")?,
            name: row.try_get("grantee_name")?,
        },
        access: ShareAccess::parse(&access)
            .ok_or_else(|| mapper::invalid_column("access", &access))?,
        shared_by,
        created_at: mapper::utc(created_at),
        updated_at: mapper::utc(updated_at),
    })
}

#[async_trait]
impl RecordShareRepository for MySqlRecordShareRepository {
    type Tx = MySqlTx;

    async fn list(&self, record: &RecordRef) -> Result<Vec<RecordShare>, AppError> {
        let rows = sqlx::query(SHARE_SELECT)
            .bind(record.record_type.as_str())
            .bind(record.record_id.to_string())
            .fetch_all(&self.pool)
            .await?;
        rows.iter().map(share_from_row).collect()
    }

    async fn lock_list(
        &self,
        tx: &mut MySqlTx,
        record: &RecordRef,
    ) -> Result<Vec<RecordShare>, AppError> {
        let rows = sqlx::query(sqlx::AssertSqlSafe(format!(
            "{SHARE_SELECT} FOR UPDATE OF s"
        )))
        .bind(record.record_type.as_str())
        .bind(record.record_id.to_string())
        .fetch_all(&mut **tx)
        .await?;
        rows.iter().map(share_from_row).collect()
    }

    async fn grantee_name(
        &self,
        grantee_type: ShareGranteeType,
        grantee_id: &str,
    ) -> Result<Option<String>, AppError> {
        let name: Option<String> = match grantee_type {
            ShareGranteeType::User => {
                sqlx::query_scalar("SELECT display_name FROM users WHERE id = ? AND status = ?")
                    .bind(grantee_id)
                    .bind(UserStatus::Active.as_str())
                    .fetch_optional(&self.pool)
                    .await?
            }
            ShareGranteeType::Team => {
                sqlx::query_scalar("SELECT name FROM teams WHERE id = ?")
                    .bind(grantee_id)
                    .fetch_optional(&self.pool)
                    .await?
            }
        };
        Ok(name)
    }

    async fn insert(&self, tx: &mut MySqlTx, share: &NewRecordShare) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO record_shares (id, record_type, record_id, grantee_type, \
             grantee_id, access, shared_by, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(share.id.to_string())
        .bind(share.record.record_type.as_str())
        .bind(share.record.record_id.to_string())
        .bind(share.grantee_type.as_str())
        .bind(&share.grantee_id)
        .bind(share.access.as_str())
        .bind(share.shared_by.as_str())
        .bind(mapper::naive(share.at))
        .bind(mapper::naive(share.at))
        .execute(&mut **tx)
        .await
        .map_err(|error| match &error {
            sqlx::Error::Database(database) if database.is_unique_violation() => {
                AppError::conflict_code(
                    RECORD_SHARE_EXISTS,
                    "the record is already shared with this user or team",
                )
            }
            _ => AppError::from(error),
        })?;
        Ok(())
    }

    async fn update_access(
        &self,
        tx: &mut MySqlTx,
        share_id: Uuid,
        access: ShareAccess,
        at: DateTime<Utc>,
    ) -> Result<(), AppError> {
        sqlx::query("UPDATE record_shares SET access = ?, updated_at = ? WHERE id = ?")
            .bind(access.as_str())
            .bind(mapper::naive(at))
            .bind(share_id.to_string())
            .execute(&mut **tx)
            .await?;
        Ok(())
    }

    async fn delete(&self, tx: &mut MySqlTx, share_id: Uuid) -> Result<(), AppError> {
        sqlx::query("DELETE FROM record_shares WHERE id = ?")
            .bind(share_id.to_string())
            .execute(&mut **tx)
            .await?;
        Ok(())
    }
}
