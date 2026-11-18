use async_trait::async_trait;
use chrono::{DateTime, NaiveDateTime, Utc};
use sqlx::mysql::MySqlRow;
use sqlx::types::Json;
use sqlx::{MySqlPool, Row};
use uuid::Uuid;

use crate::application::api_tokens::{ApiToken, ApiTokenRepository};
use crate::database::transaction::Tx;
use crate::domain::UserId;
use crate::domain::api_tokens::ApiTokenScopes;
use crate::errors::AppError;
use crate::infrastructure::repositories::mapper;

const TOKEN_COLUMNS: &str = "t.id, t.created_by, u.display_name AS created_by_name, \
     t.name, t.token_prefix, t.scopes, t.expires_at, t.last_used_at, t.last_used_ip, t.revoked_at, t.created_at";

#[derive(Clone)]
pub struct MySqlApiTokenRepository {
    pool: MySqlPool,
}

impl MySqlApiTokenRepository {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

fn token_from_row(row: &MySqlRow) -> Result<ApiToken, AppError> {
    let id: String = row.try_get("id")?;
    let Json(scopes): Json<Vec<String>> = row.try_get("scopes")?;
    let parsed = ApiTokenScopes::parse(scopes.iter().map(String::as_str))
        .map_err(|_| mapper::invalid_column("api_tokens.scopes", &scopes.join(",")))?;
    let optional = |column: &str| -> Result<Option<DateTime<Utc>>, AppError> {
        Ok(row
            .try_get::<Option<NaiveDateTime>, _>(column)?
            .map(mapper::utc))
    };
    Ok(ApiToken {
        id: Uuid::parse_str(&id).map_err(|_| mapper::invalid_column("api_tokens.id", &id))?,
        created_by: mapper::typed_id(row.try_get("created_by")?, UserId::new)?,
        created_by_name: row.try_get("created_by_name")?,
        name: row.try_get("name")?,
        token_prefix: row.try_get("token_prefix")?,
        scopes: parsed,
        expires_at: optional("expires_at")?,
        last_used_at: optional("last_used_at")?,
        last_used_ip: row.try_get("last_used_ip")?,
        revoked_at: optional("revoked_at")?,
        created_at: mapper::utc(row.try_get("created_at")?),
    })
}

#[async_trait]
impl ApiTokenRepository for MySqlApiTokenRepository {
    type Tx = Tx;

    async fn insert(
        &self,
        tx: &mut Self::Tx,
        token: &ApiToken,
        token_hash: &str,
    ) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO api_tokens (id, created_by, name, token_prefix, token_hash, scopes, expires_at, created_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(token.id.to_string())
        .bind(token.created_by.as_str())
        .bind(&token.name)
        .bind(&token.token_prefix)
        .bind(token_hash)
        .bind(Json(token.scopes.to_strings()))
        .bind(token.expires_at.map(mapper::naive))
        .bind(mapper::naive(token.created_at))
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    async fn list(&self, created_by: Option<&UserId>) -> Result<Vec<ApiToken>, AppError> {
        let sql = format!(
            "SELECT {TOKEN_COLUMNS} FROM api_tokens t LEFT JOIN users u ON u.id = t.created_by \
             WHERE (? IS NULL OR t.created_by = ?) \
             ORDER BY t.revoked_at IS NOT NULL, t.created_at DESC, t.id DESC"
        );
        let created_by = created_by.map(UserId::as_str);
        sqlx::query(sqlx::AssertSqlSafe(sql))
            .bind(created_by)
            .bind(created_by)
            .fetch_all(&self.pool)
            .await?
            .iter()
            .map(token_from_row)
            .collect()
    }

    async fn find(&self, id: Uuid) -> Result<Option<ApiToken>, AppError> {
        let sql = format!(
            "SELECT {TOKEN_COLUMNS} FROM api_tokens t LEFT JOIN users u ON u.id = t.created_by \
             WHERE t.id = ?"
        );
        sqlx::query(sqlx::AssertSqlSafe(sql))
            .bind(id.to_string())
            .fetch_optional(&self.pool)
            .await?
            .as_ref()
            .map(token_from_row)
            .transpose()
    }

    async fn count_usable(&self, created_by: &UserId, now: DateTime<Utc>) -> Result<u64, AppError> {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM api_tokens WHERE created_by = ? \
             AND revoked_at IS NULL AND (expires_at IS NULL OR expires_at > ?)",
        )
        .bind(created_by.as_str())
        .bind(mapper::naive(now))
        .fetch_one(&self.pool)
        .await?;
        Ok(count.max(0) as u64)
    }

    async fn revoke(
        &self,
        tx: &mut Self::Tx,
        id: Uuid,
        at: DateTime<Utc>,
    ) -> Result<bool, AppError> {
        let result =
            sqlx::query("UPDATE api_tokens SET revoked_at = ? WHERE id = ? AND revoked_at IS NULL")
                .bind(mapper::naive(at))
                .bind(id.to_string())
                .execute(&mut **tx)
                .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn find_by_hash(&self, token_hash: &str) -> Result<Option<ApiToken>, AppError> {
        let sql = format!(
            "SELECT {TOKEN_COLUMNS} FROM api_tokens t LEFT JOIN users u ON u.id = t.created_by \
             WHERE t.token_hash = ?"
        );
        sqlx::query(sqlx::AssertSqlSafe(sql))
            .bind(token_hash)
            .fetch_optional(&self.pool)
            .await?
            .as_ref()
            .map(token_from_row)
            .transpose()
    }

    async fn record_use(
        &self,
        id: Uuid,
        at: DateTime<Utc>,
        ip: Option<&str>,
    ) -> Result<(), AppError> {
        sqlx::query("UPDATE api_tokens SET last_used_at = ?, last_used_ip = ? WHERE id = ?")
            .bind(mapper::naive(at))
            .bind(ip)
            .bind(id.to_string())
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
