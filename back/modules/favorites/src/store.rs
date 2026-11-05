use chrono::NaiveDateTime;
use sqlx::mysql::MySqlRow;
use sqlx::{MySqlPool, Row};
use uuid::Uuid;

use crate::domain::{FAVORITE_ALREADY_EXISTS, Favorite, FavoriteKind};
use aphel_kernel::Ctx;
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::application::shared::RecordType;
use aphel_sdk::platform::database::transaction::MySqlTx;
use aphel_sdk::platform::domain::access::AccessScope;
use aphel_sdk::platform::infrastructure::repositories::mapper;

const FAVORITE_COLUMNS: &str =
    "id, favorite_type, record_type, record_id, view_id, position, created_at";

fn uuid_value(value: &str, column: &str) -> Result<Uuid, AppError> {
    Uuid::parse_str(value).map_err(|_| mapper::invalid_column(column, value))
}

fn favorite_from_row(row: &MySqlRow) -> Result<Favorite, AppError> {
    let id: String = row.try_get("id")?;
    let kind: String = row.try_get("favorite_type")?;
    let record_type: String = row.try_get("record_type")?;
    let record_id: Option<String> = row.try_get("record_id")?;
    let view_id: Option<String> = row.try_get("view_id")?;
    let created_at: NaiveDateTime = row.try_get("created_at")?;
    let kind =
        FavoriteKind::parse(&kind).ok_or_else(|| mapper::invalid_column("favorite_type", &kind))?;
    let target = match kind {
        FavoriteKind::Record => record_id.map(|value| (value, "record_id")),
        FavoriteKind::View => view_id.map(|value| (value, "view_id")),
    };
    let Some((target, column)) = target else {
        return Err(mapper::invalid_column("favorite_type", kind.as_str()));
    };
    Ok(Favorite {
        id: uuid_value(&id, "id")?,
        kind,
        record_type: RecordType::parse(record_type.clone())
            .map_err(|_| mapper::invalid_column("record_type", &record_type))?,
        target_id: uuid_value(&target, column)?,
        position: row.try_get("position")?,
        created_at: mapper::utc(created_at),
    })
}

fn is_unique_violation(error: &sqlx::Error) -> bool {
    matches!(error, sqlx::Error::Database(database_error) if database_error.is_unique_violation())
}

#[derive(Clone)]
pub(crate) struct FavoriteStore {
    pool: MySqlPool,
}

impl FavoriteStore {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    pub(crate) fn of(ctx: &Ctx<'_>) -> Self {
        Self::new(ctx.pool().clone())
    }

    pub(crate) async fn list(&self, scope: &AccessScope) -> Result<Vec<Favorite>, AppError> {
        let rows = sqlx::query(sqlx::AssertSqlSafe(format!(
            "SELECT {FAVORITE_COLUMNS} FROM favorites WHERE user_id = ? \
             ORDER BY position, created_at, id"
        )))
        .bind(scope.user_id().as_str())
        .fetch_all(&self.pool)
        .await?;
        rows.iter().map(favorite_from_row).collect()
    }

    pub(crate) async fn find_target(
        &self,
        scope: &AccessScope,
        kind: FavoriteKind,
        record_type: &RecordType,
        target_id: Uuid,
    ) -> Result<Option<Favorite>, AppError> {
        let row = sqlx::query(sqlx::AssertSqlSafe(format!(
            "SELECT {FAVORITE_COLUMNS} FROM favorites WHERE user_id = ? \
             AND favorite_type = ? AND record_type = ? AND target_id = ?"
        )))
        .bind(scope.user_id().as_str())
        .bind(kind.as_str())
        .bind(record_type.as_str())
        .bind(target_id.to_string())
        .fetch_optional(&self.pool)
        .await?;
        row.as_ref().map(favorite_from_row).transpose()
    }

    pub(crate) async fn lock_and_stats(
        &self,
        tx: &mut MySqlTx,
        scope: &AccessScope,
    ) -> Result<(usize, i32), AppError> {
        sqlx::query("SELECT id FROM users WHERE id = ? FOR UPDATE")
            .bind(scope.user_id().as_str())
            .fetch_optional(&mut **tx)
            .await?;
        let (count, max_position): (i64, i32) = sqlx::query_as(
            "SELECT COUNT(*), CAST(COALESCE(MAX(position), -1) AS SIGNED) FROM favorites WHERE user_id = ?",
        )
        .bind(scope.user_id().as_str())
        .fetch_one(&mut **tx)
        .await?;
        Ok((usize::try_from(count).unwrap_or(usize::MAX), max_position))
    }

    pub(crate) async fn insert(
        &self,
        tx: &mut MySqlTx,
        scope: &AccessScope,
        favorite: &Favorite,
    ) -> Result<(), AppError> {
        let result = sqlx::query(
            "INSERT INTO favorites (id, user_id, favorite_type, record_type, \
             record_id, view_id, target_id, position, created_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(favorite.id.to_string())
        .bind(scope.user_id().as_str())
        .bind(favorite.kind.as_str())
        .bind(favorite.record_type.as_str())
        .bind(favorite.record_id().map(|id| id.to_string()))
        .bind(favorite.view_id().map(|id| id.to_string()))
        .bind(favorite.target_id.to_string())
        .bind(favorite.position)
        .bind(mapper::naive(favorite.created_at))
        .execute(&mut **tx)
        .await;
        match result {
            Ok(_) => Ok(()),
            Err(error) if is_unique_violation(&error) => Err(AppError::conflict_code(
                FAVORITE_ALREADY_EXISTS,
                "this item is already a favorite",
            )),
            Err(error) => Err(error.into()),
        }
    }

    pub(crate) async fn delete(
        &self,
        tx: &mut MySqlTx,
        scope: &AccessScope,
        favorite_id: Uuid,
    ) -> Result<bool, AppError> {
        let result = sqlx::query("DELETE FROM favorites WHERE user_id = ? AND id = ?")
            .bind(scope.user_id().as_str())
            .bind(favorite_id.to_string())
            .execute(&mut **tx)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    pub(crate) async fn reorder(
        &self,
        tx: &mut MySqlTx,
        scope: &AccessScope,
        ordered: &[Uuid],
    ) -> Result<(), AppError> {
        for (position, id) in ordered.iter().enumerate() {
            sqlx::query(
                "UPDATE favorites SET position = ? \
                 WHERE user_id = ? AND id = ?",
            )
            .bind(i32::try_from(position).unwrap_or(i32::MAX))
            .bind(scope.user_id().as_str())
            .bind(id.to_string())
            .execute(&mut **tx)
            .await?;
        }
        Ok(())
    }
}
