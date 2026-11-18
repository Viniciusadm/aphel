pub(crate) mod dashboard;
pub(crate) mod merge_references;
pub(crate) mod privacy;
pub(crate) mod search;
pub(crate) mod timeline;

use chrono::{DateTime, NaiveDateTime, Utc};
use sqlx::mysql::MySqlRow;
use sqlx::{MySql, MySqlPool, QueryBuilder, Row};
use uuid::Uuid;

use crate::domain::{NoteData, NoteView, NoteVisibility};
use aphel_kernel::Ctx;
use aphel_kernel::domain::UserId;
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::application::organization_admin::{ListWindow, Listing};
use aphel_sdk::platform::application::shared::RecordRef;
use aphel_sdk::platform::database::transaction::Tx;
use aphel_sdk::platform::domain::access::{AccessScope, RecordScopeFilter};
use aphel_sdk::platform::domain::users::UserStatus;
use aphel_sdk::platform::infrastructure::fields::mysql_definitions::uuid_column;
use aphel_sdk::platform::infrastructure::repositories::mapper::{self, user_reference};

const NOTE_SELECT: &str = "SELECT n.id, n.author_id, au.display_name AS author_name, n.record_type, n.record_id, \
     n.body, n.visibility, n.pinned, n.created_at, n.updated_at \
     FROM notes n LEFT JOIN users au ON au.id = n.author_id ";

pub fn note_from_row(row: &MySqlRow) -> Result<NoteView, AppError> {
    let record_type: String = row.try_get("record_type")?;
    let record_id: String = row.try_get("record_id")?;
    let visibility: String = row.try_get("visibility")?;
    let created_at: NaiveDateTime = row.try_get("created_at")?;
    let updated_at: NaiveDateTime = row.try_get("updated_at")?;
    Ok(NoteView {
        id: uuid_column(row, "id")?,
        author: user_reference(row, "author_id", "author_name")?,
        record: RecordRef::parse(&record_type, &record_id)
            .map_err(|_| mapper::invalid_column("record_type", &record_type))?,
        body: row.try_get("body")?,
        visibility: NoteVisibility::parse(&visibility)
            .ok_or_else(|| mapper::invalid_column("visibility", &visibility))?,
        pinned: row.try_get("pinned")?,
        created_at: mapper::utc(created_at),
        updated_at: mapper::utc(updated_at),
    })
}

#[derive(Clone)]
pub(crate) struct NoteStore {
    pool: MySqlPool,
}

impl NoteStore {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    pub(crate) fn of(ctx: &Ctx<'_>) -> Self {
        Self::new(ctx.pool().clone())
    }
}

impl NoteStore {
    pub(crate) async fn insert(
        &self,
        tx: &mut Tx,
        scope: &AccessScope,
        id: Uuid,
        record: &RecordRef,
        data: &NoteData,
        now: DateTime<Utc>,
    ) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO notes (id, author_id, record_type, record_id, body, visibility, pinned, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(id.to_string())
        .bind(scope.user_id().as_str())
        .bind(record.record_type.as_str())
        .bind(record.record_id.to_string())
        .bind(&data.body)
        .bind(data.visibility.as_str())
        .bind(data.pinned)
        .bind(mapper::naive(now))
        .bind(mapper::naive(now))
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub(crate) async fn update(
        &self,
        tx: &mut Tx,
        id: Uuid,
        data: &NoteData,
        updated_at: DateTime<Utc>,
    ) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE notes SET body = ?, visibility = ?, pinned = ?, updated_at = ? \
             WHERE id = ? AND deleted_at IS NULL",
        )
        .bind(&data.body)
        .bind(data.visibility.as_str())
        .bind(data.pinned)
        .bind(mapper::naive(updated_at))
        .bind(id.to_string())
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub(crate) async fn soft_delete(
        &self,
        tx: &mut Tx,
        id: Uuid,
        now: DateTime<Utc>,
    ) -> Result<bool, AppError> {
        let result = sqlx::query(
            "UPDATE notes SET deleted_at = ?, updated_at = updated_at \
             WHERE id = ? AND deleted_at IS NULL",
        )
        .bind(mapper::naive(now))
        .bind(id.to_string())
        .execute(&mut **tx)
        .await?;
        Ok(result.rows_affected() == 1)
    }

    pub(crate) async fn find_deleted(
        &self,
        id: Uuid,
        since: DateTime<Utc>,
    ) -> Result<Option<NoteView>, AppError> {
        let row = sqlx::query(sqlx::AssertSqlSafe(format!(
            "{NOTE_SELECT} WHERE n.id = ? AND n.deleted_at IS NOT NULL AND n.deleted_at >= ?"
        )))
        .bind(id.to_string())
        .bind(mapper::naive(since))
        .fetch_optional(&self.pool)
        .await?;
        row.as_ref().map(note_from_row).transpose()
    }

    pub(crate) async fn restore(&self, tx: &mut Tx, id: Uuid) -> Result<bool, AppError> {
        let result = sqlx::query(
            "UPDATE notes SET deleted_at = NULL, updated_at = updated_at \
             WHERE id = ? AND deleted_at IS NOT NULL",
        )
        .bind(id.to_string())
        .execute(&mut **tx)
        .await?;
        Ok(result.rows_affected() == 1)
    }

    pub(crate) async fn find(&self, id: Uuid) -> Result<Option<NoteView>, AppError> {
        let row = sqlx::query(sqlx::AssertSqlSafe(format!(
            "{NOTE_SELECT} WHERE n.id = ? AND n.deleted_at IS NULL"
        )))
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await?;
        row.as_ref().map(note_from_row).transpose()
    }

    pub(crate) async fn list_visible(
        &self,
        scope: &AccessScope,
        record: &RecordRef,
        window: ListWindow,
    ) -> Result<Listing<NoteView>, AppError> {
        let filter = "n.record_type = ? AND n.record_id = ? \
             AND n.deleted_at IS NULL AND (n.visibility = 'team' OR n.author_id = ?)";
        let total: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
            "SELECT COUNT(*) FROM notes n WHERE {filter}"
        )))
        .bind(record.record_type.as_str())
        .bind(record.record_id.to_string())
        .bind(scope.user_id().as_str())
        .fetch_one(&self.pool)
        .await?;
        let rows = sqlx::query(sqlx::AssertSqlSafe(format!(
            "{NOTE_SELECT} WHERE {filter} ORDER BY n.pinned DESC, n.created_at DESC, n.id DESC LIMIT ? OFFSET ?"
        )))
        .bind(record.record_type.as_str())
        .bind(record.record_id.to_string())
        .bind(scope.user_id().as_str())
        .bind(window.limit)
        .bind(window.offset)
        .fetch_all(&self.pool)
        .await?;
        Ok(Listing {
            items: rows.iter().map(note_from_row).collect::<Result<_, _>>()?,
            total: u64::try_from(total).unwrap_or_default(),
        })
    }

    pub(crate) async fn author_within(
        &self,
        author_id: &UserId,
        filter: &RecordScopeFilter,
    ) -> Result<bool, AppError> {
        match filter {
            RecordScopeFilter::Organization => Ok(true),
            RecordScopeFilter::Owner { user_id, .. } => Ok(user_id == author_id),
            RecordScopeFilter::Teams {
                user_id, team_ids, ..
            } => {
                if user_id == author_id {
                    return Ok(true);
                }
                if team_ids.is_empty() {
                    return Ok(false);
                }
                let mut builder = QueryBuilder::<MySql>::new(
                    "SELECT EXISTS(SELECT 1 FROM team_memberships tm JOIN teams t ON t.id = tm.team_id \
                     WHERE ",
                );
                builder
                    .push("tm.user_id = ")
                    .push_bind(author_id.as_str().to_owned())
                    .push(" AND tm.team_id IN (");
                let mut separated = builder.separated(", ");
                for team in team_ids {
                    separated.push_bind(team.as_str().to_owned());
                }
                builder.push("))");
                let exists: bool = builder.build_query_scalar().fetch_one(&self.pool).await?;
                Ok(exists)
            }
        }
    }

    pub(crate) async fn active_members(
        &self,
        user_ids: &[UserId],
    ) -> Result<Vec<UserId>, AppError> {
        if user_ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut builder = QueryBuilder::<MySql>::new("SELECT id FROM users WHERE status = ");
        builder
            .push_bind(UserStatus::Active.as_str())
            .push(" AND id IN (");
        let mut separated = builder.separated(", ");
        for user_id in user_ids {
            separated.push_bind(user_id.as_str().to_owned());
        }
        builder.push(")");
        let ids: Vec<String> = builder.build_query_scalar().fetch_all(&self.pool).await?;
        ids.into_iter()
            .map(|id| mapper::typed_id(id, UserId::new))
            .collect()
    }
}

pub(crate) async fn team_note_exists(
    pool: &aphel_sdk::platform::database::Db,
    id: &str,
) -> Result<bool, AppError> {
    Ok(sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM notes WHERE id = ? AND deleted_at IS NULL AND visibility = 'team')",
    )
    .bind(id)
    .fetch_one(pool)
    .await?)
}

pub(crate) async fn delete_for_record(
    tx: &mut aphel_sdk::platform::database::transaction::Tx,
    record_type: &str,
    record_id: &str,
) -> Result<(), AppError> {
    sqlx::query("DELETE FROM notes WHERE record_type = ? AND record_id = ?")
        .bind(record_type)
        .bind(record_id)
        .execute(&mut **tx)
        .await?;
    Ok(())
}
