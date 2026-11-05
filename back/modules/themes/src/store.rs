use chrono::{DateTime, Utc};
use sqlx::mysql::MySqlRow;
use sqlx::types::Json;
use sqlx::{MySqlPool, Row};

use crate::package::TokenSets;
use crate::validate::DecodedFont;
use aphel_kernel::Ctx;
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::database::transaction::MySqlTx;
use aphel_sdk::platform::infrastructure::repositories::mapper;

const THEME_COLUMNS: &str = "theme_key, name, version, author, description, license, \
    min_aphel_version, tokens, source, checksum, installed_by, created_at, updated_at";

#[derive(Debug, Clone)]
pub(crate) struct ThemeRecord {
    pub(crate) key: String,
    pub(crate) name: String,
    pub(crate) version: String,
    pub(crate) author: String,
    pub(crate) description: Option<String>,
    pub(crate) license: String,
    pub(crate) min_aphel_version: Option<String>,
    pub(crate) tokens: TokenSets,
    pub(crate) source: String,
    pub(crate) checksum: String,
    pub(crate) installed_by: Option<String>,
    pub(crate) created_at: DateTime<Utc>,
    pub(crate) updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub(crate) struct StoredFont {
    pub(crate) family: String,
    pub(crate) weight: u16,
    pub(crate) style: String,
    pub(crate) sha256: String,
    pub(crate) bytes: Vec<u8>,
}

#[derive(Debug, Clone)]
pub(crate) struct ThemeState {
    pub(crate) active_key: Option<String>,
    pub(crate) version: i64,
}

fn theme_from_row(row: &MySqlRow) -> Result<ThemeRecord, AppError> {
    let tokens: Json<TokenSets> = row.try_get("tokens")?;
    Ok(ThemeRecord {
        key: row.try_get("theme_key")?,
        name: row.try_get("name")?,
        version: row.try_get("version")?,
        author: row.try_get("author")?,
        description: row.try_get("description")?,
        license: row.try_get("license")?,
        min_aphel_version: row.try_get("min_aphel_version")?,
        tokens: tokens.0,
        source: row.try_get("source")?,
        checksum: row.try_get("checksum")?,
        installed_by: row.try_get("installed_by")?,
        created_at: mapper::utc(row.try_get("created_at")?),
        updated_at: mapper::utc(row.try_get("updated_at")?),
    })
}

fn font_from_row(row: &MySqlRow) -> Result<StoredFont, AppError> {
    let weight: i16 = row.try_get("weight")?;
    Ok(StoredFont {
        family: row.try_get("family")?,
        weight: u16::try_from(weight)
            .map_err(|_| mapper::invalid_column("weight", &weight.to_string()))?,
        style: row.try_get("style")?,
        sha256: row.try_get("sha256")?,
        bytes: row.try_get("data")?,
    })
}

pub(crate) struct ThemeStore {
    pool: MySqlPool,
}

impl ThemeStore {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    pub(crate) fn of(ctx: &Ctx<'_>) -> Self {
        Self::new(ctx.pool().clone())
    }

    pub(crate) async fn state(&self) -> Result<ThemeState, AppError> {
        let row = sqlx::query("SELECT active_key, version FROM themes_state WHERE id = 1")
            .fetch_one(&self.pool)
            .await?;
        Ok(ThemeState {
            active_key: row.try_get("active_key")?,
            version: row.try_get("version")?,
        })
    }

    pub(crate) async fn list(&self) -> Result<Vec<ThemeRecord>, AppError> {
        let rows = sqlx::query(sqlx::AssertSqlSafe(format!(
            "SELECT {THEME_COLUMNS} FROM themes_installed ORDER BY name, theme_key"
        )))
        .fetch_all(&self.pool)
        .await?;
        rows.iter().map(theme_from_row).collect()
    }

    pub(crate) async fn find(&self, key: &str) -> Result<Option<ThemeRecord>, AppError> {
        let row = sqlx::query(sqlx::AssertSqlSafe(format!(
            "SELECT {THEME_COLUMNS} FROM themes_installed WHERE theme_key = ?"
        )))
        .bind(key)
        .fetch_optional(&self.pool)
        .await?;
        row.as_ref().map(theme_from_row).transpose()
    }

    pub(crate) async fn count(&self) -> Result<i64, AppError> {
        let row = sqlx::query("SELECT COUNT(*) AS total FROM themes_installed")
            .fetch_one(&self.pool)
            .await?;
        Ok(row.try_get("total")?)
    }

    pub(crate) async fn fonts(&self, key: &str) -> Result<Vec<StoredFont>, AppError> {
        let rows = sqlx::query(
            "SELECT family, weight, style, sha256, data FROM themes_fonts \
             WHERE theme_key = ? ORDER BY family, weight, style",
        )
        .bind(key)
        .fetch_all(&self.pool)
        .await?;
        rows.iter().map(font_from_row).collect()
    }

    pub(crate) async fn font_by_hash(&self, sha256: &str) -> Result<Option<Vec<u8>>, AppError> {
        let row = sqlx::query("SELECT data FROM themes_fonts WHERE sha256 = ? LIMIT 1")
            .bind(sha256)
            .fetch_optional(&self.pool)
            .await?;
        row.map(|row| row.try_get("data").map_err(AppError::from))
            .transpose()
    }

    pub(crate) async fn upsert(
        &self,
        tx: &mut MySqlTx,
        theme: &ThemeRecord,
    ) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO themes_installed (theme_key, name, version, author, description, license, \
             min_aphel_version, tokens, source, checksum, installed_by, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) \
             ON DUPLICATE KEY UPDATE name = VALUES(name), version = VALUES(version), \
             author = VALUES(author), description = VALUES(description), license = VALUES(license), \
             min_aphel_version = VALUES(min_aphel_version), tokens = VALUES(tokens), \
             source = VALUES(source), checksum = VALUES(checksum), installed_by = VALUES(installed_by), \
             updated_at = VALUES(updated_at)",
        )
        .bind(&theme.key)
        .bind(&theme.name)
        .bind(&theme.version)
        .bind(&theme.author)
        .bind(&theme.description)
        .bind(&theme.license)
        .bind(&theme.min_aphel_version)
        .bind(Json(&theme.tokens))
        .bind(&theme.source)
        .bind(&theme.checksum)
        .bind(&theme.installed_by)
        .bind(theme.created_at.naive_utc())
        .bind(theme.updated_at.naive_utc())
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub(crate) async fn replace_fonts(
        &self,
        tx: &mut MySqlTx,
        key: &str,
        fonts: &[DecodedFont],
    ) -> Result<(), AppError> {
        sqlx::query("DELETE FROM themes_fonts WHERE theme_key = ?")
            .bind(key)
            .execute(&mut **tx)
            .await?;
        for font in fonts {
            sqlx::query(
                "INSERT INTO themes_fonts (theme_key, family, weight, style, sha256, size, data) \
                 VALUES (?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(key)
            .bind(&font.family)
            .bind(i16::try_from(font.weight).unwrap_or(400))
            .bind(&font.style)
            .bind(&font.sha256)
            .bind(i32::try_from(font.bytes.len()).unwrap_or(i32::MAX))
            .bind(&font.bytes)
            .execute(&mut **tx)
            .await?;
        }
        Ok(())
    }

    pub(crate) async fn delete(&self, tx: &mut MySqlTx, key: &str) -> Result<bool, AppError> {
        let result = sqlx::query("DELETE FROM themes_installed WHERE theme_key = ?")
            .bind(key)
            .execute(&mut **tx)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    pub(crate) async fn set_active(
        &self,
        tx: &mut MySqlTx,
        key: Option<&str>,
    ) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE themes_state SET active_key = ?, version = version + 1, \
             updated_at = CURRENT_TIMESTAMP(6) WHERE id = 1",
        )
        .bind(key)
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub(crate) async fn bump_version(&self, tx: &mut MySqlTx) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE themes_state SET version = version + 1, updated_at = CURRENT_TIMESTAMP(6) \
             WHERE id = 1",
        )
        .execute(&mut **tx)
        .await?;
        Ok(())
    }
}
