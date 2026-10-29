use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, NaiveDateTime, Utc};
use sqlx::{AssertSqlSafe, MySqlPool, Row};
use uuid::Uuid;

use crate::application::auth::two_factor::{
    TwoFactorPolicy, TwoFactorPolicyRepository, TwoFactorRecord, TwoFactorRepository,
    TwoFactorRequirement,
};
use crate::database::transaction::MySqlTx;
use crate::domain::access::RoleKey;
use crate::domain::{Secret, UserId};
use crate::errors::AppError;
use crate::infrastructure::repositories::mapper;
use crate::infrastructure::secrets::{EncryptedValue, EncryptionKeyring};

const ROTATION_BATCH: i64 = 200;

#[derive(Clone)]
pub struct MySqlTwoFactorRepository {
    pool: MySqlPool,
    keyring: Arc<EncryptionKeyring>,
}

impl MySqlTwoFactorRepository {
    pub(crate) fn new(pool: MySqlPool, keyring: Arc<EncryptionKeyring>) -> Self {
        Self { pool, keyring }
    }
}

fn context(user_id: &str) -> Vec<u8> {
    format!("aphel:two_factor:{user_id}:totp").into_bytes()
}

fn into_secret(bytes: Vec<u8>) -> Result<Secret, AppError> {
    String::from_utf8(bytes)
        .map(Secret::new)
        .map_err(|_| AppError::Internal("stored TOTP secret is not valid UTF-8".to_owned()))
}

async fn insert_recovery_codes(
    tx: &mut MySqlTx,
    user_id: &UserId,
    code_hashes: &[String],
    at: DateTime<Utc>,
) -> Result<(), AppError> {
    sqlx::query("DELETE FROM two_factor_recovery_codes WHERE user_id = ?")
        .bind(user_id.as_str())
        .execute(&mut **tx)
        .await?;
    if code_hashes.is_empty() {
        return Ok(());
    }
    let placeholders = vec!["(?, ?, ?, ?)"; code_hashes.len()].join(", ");
    let sql = format!(
        "INSERT INTO two_factor_recovery_codes (id, user_id, code_hash, created_at) VALUES {placeholders}"
    );
    let mut query = sqlx::query(AssertSqlSafe(sql));
    for hash in code_hashes {
        query = query
            .bind(Uuid::now_v7().to_string())
            .bind(user_id.as_str())
            .bind(hash)
            .bind(mapper::naive(at));
    }
    query.execute(&mut **tx).await?;
    Ok(())
}

#[async_trait]
impl TwoFactorRepository for MySqlTwoFactorRepository {
    async fn find(&self, user_id: &UserId) -> Result<Option<TwoFactorRecord>, AppError> {
        let row = sqlx::query(
            "SELECT key_id, nonce, ciphertext, enabled_at, last_used_step FROM user_two_factor WHERE user_id = ?",
        )
        .bind(user_id.as_str())
        .fetch_optional(&self.pool)
        .await?;
        let Some(row) = row else {
            return Ok(None);
        };
        let encrypted = EncryptedValue {
            key_id: row.try_get("key_id")?,
            nonce: row.try_get("nonce")?,
            ciphertext: row.try_get("ciphertext")?,
        };
        let secret = into_secret(
            self.keyring
                .decrypt(&encrypted, &context(user_id.as_str()))?,
        )?;
        let enabled_at: Option<NaiveDateTime> = row.try_get("enabled_at")?;
        Ok(Some(TwoFactorRecord {
            user_id: user_id.clone(),
            secret,
            enabled_at: enabled_at.map(mapper::utc),
            last_used_step: row.try_get("last_used_step")?,
        }))
    }

    async fn requirement(&self, user_id: &UserId) -> Result<TwoFactorRequirement, AppError> {
        let row = sqlx::query(
            "SELECT EXISTS(SELECT 1 FROM user_two_factor WHERE user_id = ? AND enabled_at IS NOT NULL) AS enabled, \
             (SELECT two_factor_policy FROM organization_settings WHERE id = 1) AS policy",
        )
        .bind(user_id.as_str())
        .fetch_one(&self.pool)
        .await?;
        let enabled: i64 = row.try_get("enabled")?;
        let policy: Option<String> = row.try_get("policy")?;
        Ok(TwoFactorRequirement {
            enabled: enabled != 0,
            policy: policy
                .as_deref()
                .and_then(TwoFactorPolicy::parse)
                .unwrap_or_default(),
        })
    }

    async fn save_pending(
        &self,
        user_id: &UserId,
        secret: &Secret,
        at: DateTime<Utc>,
    ) -> Result<bool, AppError> {
        let encrypted = self
            .keyring
            .encrypt(secret.expose().as_bytes(), &context(user_id.as_str()))?;
        let mut tx = self.pool.begin().await?;
        sqlx::query("DELETE FROM user_two_factor WHERE user_id = ? AND enabled_at IS NULL")
            .bind(user_id.as_str())
            .execute(&mut *tx)
            .await?;
        let inserted = sqlx::query(
            "INSERT IGNORE INTO user_two_factor (user_id, key_id, nonce, ciphertext, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(user_id.as_str())
        .bind(&encrypted.key_id)
        .bind(&encrypted.nonce)
        .bind(&encrypted.ciphertext)
        .bind(mapper::naive(at))
        .bind(mapper::naive(at))
        .execute(&mut *tx)
        .await?
        .rows_affected();
        tx.commit().await?;
        Ok(inserted == 1)
    }

    async fn enable(
        &self,
        user_id: &UserId,
        step: u64,
        recovery_code_hashes: &[String],
        at: DateTime<Utc>,
    ) -> Result<bool, AppError> {
        let mut tx = self.pool.begin().await?;
        let updated = sqlx::query(
            "UPDATE user_two_factor SET enabled_at = ?, last_used_step = ?, updated_at = ? WHERE user_id = ? AND enabled_at IS NULL",
        )
        .bind(mapper::naive(at))
        .bind(step)
        .bind(mapper::naive(at))
        .bind(user_id.as_str())
        .execute(&mut *tx)
        .await?
        .rows_affected();
        if updated != 1 {
            return Ok(false);
        }
        insert_recovery_codes(&mut tx, user_id, recovery_code_hashes, at).await?;
        tx.commit().await?;
        Ok(true)
    }

    async fn claim_step(&self, user_id: &UserId, step: u64) -> Result<bool, AppError> {
        let updated = sqlx::query(
            "UPDATE user_two_factor SET last_used_step = ? \
             WHERE user_id = ? AND enabled_at IS NOT NULL AND (last_used_step IS NULL OR last_used_step < ?)",
        )
        .bind(step)
        .bind(user_id.as_str())
        .bind(step)
        .execute(&self.pool)
        .await?
        .rows_affected();
        Ok(updated == 1)
    }

    async fn consume_recovery_code(
        &self,
        user_id: &UserId,
        code_hash: &str,
        at: DateTime<Utc>,
    ) -> Result<bool, AppError> {
        let updated = sqlx::query(
            "UPDATE two_factor_recovery_codes SET used_at = ? WHERE user_id = ? AND code_hash = ? AND used_at IS NULL",
        )
        .bind(mapper::naive(at))
        .bind(user_id.as_str())
        .bind(code_hash)
        .execute(&self.pool)
        .await?
        .rows_affected();
        Ok(updated == 1)
    }

    async fn replace_recovery_codes(
        &self,
        user_id: &UserId,
        code_hashes: &[String],
        at: DateTime<Utc>,
    ) -> Result<(), AppError> {
        let mut tx = self.pool.begin().await?;
        insert_recovery_codes(&mut tx, user_id, code_hashes, at).await?;
        tx.commit().await?;
        Ok(())
    }

    async fn remaining_recovery_codes(&self, user_id: &UserId) -> Result<u32, AppError> {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM two_factor_recovery_codes WHERE user_id = ? AND used_at IS NULL",
        )
        .bind(user_id.as_str())
        .fetch_one(&self.pool)
        .await?;
        Ok(u32::try_from(count).unwrap_or_default())
    }

    async fn disable(&self, user_id: &UserId) -> Result<(), AppError> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("DELETE FROM two_factor_recovery_codes WHERE user_id = ?")
            .bind(user_id.as_str())
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM user_two_factor WHERE user_id = ?")
            .bind(user_id.as_str())
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(())
    }
}

#[derive(Clone)]
pub struct MySqlTwoFactorPolicyRepository {
    pool: MySqlPool,
}

impl MySqlTwoFactorPolicyRepository {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl TwoFactorPolicyRepository for MySqlTwoFactorPolicyRepository {
    type Tx = MySqlTx;

    async fn find(&self) -> Result<TwoFactorPolicy, AppError> {
        let policy: Option<String> =
            sqlx::query_scalar("SELECT two_factor_policy FROM organization_settings WHERE id = 1")
                .fetch_optional(&self.pool)
                .await?;
        Ok(policy
            .as_deref()
            .and_then(TwoFactorPolicy::parse)
            .unwrap_or_default())
    }

    async fn save(
        &self,
        tx: &mut MySqlTx,
        _updated_by: &UserId,
        policy: TwoFactorPolicy,
        at: DateTime<Utc>,
    ) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE organization_settings SET two_factor_policy = ?, updated_at = ? WHERE id = 1",
        )
        .bind(policy.as_str())
        .bind(mapper::naive(at))
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    async fn members_without_two_factor(&self, policy: TwoFactorPolicy) -> Result<u64, AppError> {
        let role_filter = match policy {
            TwoFactorPolicy::None => return Ok(0),
            TwoFactorPolicy::Admins => "AND r.`key` = ?",
            TwoFactorPolicy::Everyone => "",
        };
        let sql = format!(
            "SELECT COUNT(*) FROM users u \
             JOIN roles r ON r.id = u.role_id \
             LEFT JOIN user_two_factor t ON t.user_id = u.id AND t.enabled_at IS NOT NULL \
             WHERE u.status = 'active' AND t.user_id IS NULL {role_filter}"
        );
        let mut query = sqlx::query_scalar::<_, i64>(AssertSqlSafe(sql));
        if policy == TwoFactorPolicy::Admins {
            query = query.bind(RoleKey::ADMIN);
        }
        let count = query.fetch_one(&self.pool).await?;
        Ok(u64::try_from(count).unwrap_or_default())
    }
}

pub async fn rotate_two_factor_secrets(
    pool: &MySqlPool,
    keyring: &EncryptionKeyring,
) -> Result<u64, AppError> {
    let current = keyring
        .current_key_id()
        .ok_or_else(crate::application::secrets::encryption_key_missing)?
        .to_owned();
    let mut rotated = 0;
    loop {
        let rows = sqlx::query(
            "SELECT user_id, key_id, nonce, ciphertext FROM user_two_factor WHERE key_id <> ? ORDER BY user_id LIMIT ?",
        )
        .bind(&current)
        .bind(ROTATION_BATCH)
        .fetch_all(pool)
        .await?;
        if rows.is_empty() {
            break;
        }
        for row in rows {
            let user_id: String = row.try_get("user_id")?;
            let aad = context(&user_id);
            let plaintext = keyring.decrypt(
                &EncryptedValue {
                    key_id: row.try_get("key_id")?,
                    nonce: row.try_get("nonce")?,
                    ciphertext: row.try_get("ciphertext")?,
                },
                &aad,
            )?;
            let encrypted = keyring.encrypt(&plaintext, &aad)?;
            sqlx::query(
                "UPDATE user_two_factor SET key_id = ?, nonce = ?, ciphertext = ? WHERE user_id = ?",
            )
            .bind(&encrypted.key_id)
            .bind(&encrypted.nonce)
            .bind(&encrypted.ciphertext)
            .bind(&user_id)
            .execute(pool)
            .await?;
            rotated += 1;
        }
    }
    Ok(rotated)
}
