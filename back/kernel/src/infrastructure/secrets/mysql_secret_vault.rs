use async_trait::async_trait;
use chrono::Utc;
use sqlx::{AssertSqlSafe, MySqlPool, Row};
use uuid::Uuid;

use crate::application::secrets::{MAX_SECRET_BYTES, SecretKey, SecretVault};
use crate::database::transaction::MySqlTx;
use crate::domain::Secret;
use crate::domain::access::AccessScope;
use crate::errors::AppError;
use crate::infrastructure::repositories::mapper;
use crate::infrastructure::secrets::keyring::{EncryptedValue, EncryptionKeyring};

const ROTATION_BATCH: i64 = 200;

#[derive(Clone)]
pub struct MySqlSecretVault {
    pool: MySqlPool,
    keyring: EncryptionKeyring,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RotationReport {
    pub rotated: u64,
    pub current: u64,
}

impl MySqlSecretVault {
    pub(crate) fn new(pool: MySqlPool, keyring: EncryptionKeyring) -> Self {
        Self { pool, keyring }
    }
}

fn context(key: &SecretKey) -> Vec<u8> {
    format!(
        "aphel:secret:{}:{}:{}",
        key.owner_type, key.owner_id, key.name
    )
    .into_bytes()
}

fn into_secret(bytes: Vec<u8>) -> Result<Secret, AppError> {
    String::from_utf8(bytes)
        .map(Secret::new)
        .map_err(|_| AppError::Internal("stored secret is not valid UTF-8".to_owned()))
}

#[async_trait]
impl SecretVault<MySqlTx> for MySqlSecretVault {
    async fn store(
        &self,
        tx: &mut MySqlTx,
        _scope: &AccessScope,
        key: &SecretKey,
        value: &Secret,
    ) -> Result<(), AppError> {
        if value.expose().len() > MAX_SECRET_BYTES {
            return Err(AppError::Validation(format!(
                "secret must have at most {MAX_SECRET_BYTES} bytes"
            )));
        }
        let encrypted = self
            .keyring
            .encrypt(value.expose().as_bytes(), &context(key))?;
        let now = mapper::naive(Utc::now());
        sqlx::query(
            "INSERT INTO secrets (id, owner_type, owner_id, name, key_id, nonce, ciphertext, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?) \
             ON DUPLICATE KEY UPDATE key_id = VALUES(key_id), nonce = VALUES(nonce), ciphertext = VALUES(ciphertext), updated_at = VALUES(updated_at)",
        )
        .bind(Uuid::now_v7().to_string())
        .bind(&key.owner_type)
        .bind(&key.owner_id)
        .bind(&key.name)
        .bind(&encrypted.key_id)
        .bind(&encrypted.nonce)
        .bind(&encrypted.ciphertext)
        .bind(now)
        .bind(now)
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    async fn reveal(
        &self,
        _scope: &AccessScope,
        key: &SecretKey,
    ) -> Result<Option<Secret>, AppError> {
        let row = sqlx::query(
            "SELECT key_id, nonce, ciphertext FROM secrets \
             WHERE owner_type = ? AND owner_id = ? AND name = ?",
        )
        .bind(&key.owner_type)
        .bind(&key.owner_id)
        .bind(&key.name)
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
        let plaintext = self.keyring.decrypt(&encrypted, &context(key))?;
        into_secret(plaintext).map(Some)
    }

    async fn remove(
        &self,
        tx: &mut MySqlTx,
        _scope: &AccessScope,
        key: &SecretKey,
    ) -> Result<(), AppError> {
        sqlx::query("DELETE FROM secrets WHERE owner_type = ? AND owner_id = ? AND name = ?")
            .bind(&key.owner_type)
            .bind(&key.owner_id)
            .bind(&key.name)
            .execute(&mut **tx)
            .await?;
        Ok(())
    }

    async fn remove_owner(
        &self,
        tx: &mut MySqlTx,
        _scope: &AccessScope,
        owner_type: &str,
        owner_id: &str,
    ) -> Result<(), AppError> {
        sqlx::query("DELETE FROM secrets WHERE owner_type = ? AND owner_id = ?")
            .bind(owner_type)
            .bind(owner_id)
            .execute(&mut **tx)
            .await?;
        Ok(())
    }

    async fn configured(
        &self,
        _scope: &AccessScope,
        owner_type: &str,
        owner_ids: &[String],
    ) -> Result<Vec<SecretKey>, AppError> {
        if owner_ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = vec!["?"; owner_ids.len()].join(", ");
        let sql = format!(
            "SELECT owner_id, name FROM secrets WHERE owner_type = ? AND owner_id IN ({placeholders})"
        );
        let mut query = sqlx::query(AssertSqlSafe(sql)).bind(owner_type);
        for owner_id in owner_ids {
            query = query.bind(owner_id);
        }
        query
            .fetch_all(&self.pool)
            .await?
            .iter()
            .map(|row| {
                Ok(SecretKey::new(
                    owner_type,
                    row.try_get::<String, _>("owner_id")?,
                    row.try_get::<String, _>("name")?,
                ))
            })
            .collect()
    }

    async fn reveal_installation(&self, key: &SecretKey) -> Result<Option<Secret>, AppError> {
        let row = sqlx::query(
            "SELECT key_id, nonce, ciphertext FROM installation_secrets \
             WHERE owner_type = ? AND owner_id = ? AND name = ?",
        )
        .bind(&key.owner_type)
        .bind(&key.owner_id)
        .bind(&key.name)
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
        let plaintext = self
            .keyring
            .decrypt(&encrypted, &installation_context(key))?;
        into_secret(plaintext).map(Some)
    }

    async fn store_installation_if_absent(
        &self,
        key: &SecretKey,
        value: &Secret,
    ) -> Result<bool, AppError> {
        if value.expose().len() > MAX_SECRET_BYTES {
            return Err(AppError::Validation(format!(
                "secret must have at most {MAX_SECRET_BYTES} bytes"
            )));
        }
        let encrypted = self
            .keyring
            .encrypt(value.expose().as_bytes(), &installation_context(key))?;
        let now = mapper::naive(Utc::now());
        let result = sqlx::query(
            "INSERT IGNORE INTO installation_secrets (id, owner_type, owner_id, name, key_id, nonce, ciphertext, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(Uuid::now_v7().to_string())
        .bind(&key.owner_type)
        .bind(&key.owner_id)
        .bind(&key.name)
        .bind(&encrypted.key_id)
        .bind(&encrypted.nonce)
        .bind(&encrypted.ciphertext)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn store_organization_if_absent(
        &self,
        _scope: &AccessScope,
        key: &SecretKey,
        value: &Secret,
    ) -> Result<bool, AppError> {
        if value.expose().len() > MAX_SECRET_BYTES {
            return Err(AppError::Validation(format!(
                "secret must have at most {MAX_SECRET_BYTES} bytes"
            )));
        }
        let encrypted = self
            .keyring
            .encrypt(value.expose().as_bytes(), &context(key))?;
        let now = mapper::naive(Utc::now());
        let result = sqlx::query(
            "INSERT IGNORE INTO secrets (id, owner_type, owner_id, name, key_id, nonce, ciphertext, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(Uuid::now_v7().to_string())
        .bind(&key.owner_type)
        .bind(&key.owner_id)
        .bind(&key.name)
        .bind(&encrypted.key_id)
        .bind(&encrypted.nonce)
        .bind(&encrypted.ciphertext)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }
}

fn installation_context(key: &SecretKey) -> Vec<u8> {
    format!(
        "aphel:installation-secret:{}:{}:{}",
        key.owner_type, key.owner_id, key.name
    )
    .into_bytes()
}

async fn rotate_installation_secrets(
    pool: &MySqlPool,
    keyring: &EncryptionKeyring,
    current: &str,
) -> Result<u64, AppError> {
    let rows = sqlx::query(
        "SELECT id, owner_type, owner_id, name, key_id, nonce, ciphertext \
         FROM installation_secrets WHERE key_id <> ? ORDER BY id",
    )
    .bind(current)
    .fetch_all(pool)
    .await?;
    let mut rotated = 0;
    for row in rows {
        let id: String = row.try_get("id")?;
        let key = SecretKey::new(
            row.try_get::<String, _>("owner_type")?,
            row.try_get::<String, _>("owner_id")?,
            row.try_get::<String, _>("name")?,
        );
        let aad = installation_context(&key);
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
            "UPDATE installation_secrets SET key_id = ?, nonce = ?, ciphertext = ? WHERE id = ?",
        )
        .bind(&encrypted.key_id)
        .bind(&encrypted.nonce)
        .bind(&encrypted.ciphertext)
        .bind(&id)
        .execute(pool)
        .await?;
        rotated += 1;
    }
    Ok(rotated)
}

pub async fn rotate_secrets(
    pool: &MySqlPool,
    keyring: &EncryptionKeyring,
) -> Result<RotationReport, AppError> {
    let current = keyring
        .current_key_id()
        .ok_or_else(crate::application::secrets::encryption_key_missing)?
        .to_owned();
    let mut report = RotationReport::default();
    loop {
        let rows = sqlx::query(
            "SELECT id, owner_type, owner_id, name, key_id, nonce, ciphertext \
             FROM secrets WHERE key_id <> ? ORDER BY id LIMIT ?",
        )
        .bind(&current)
        .bind(ROTATION_BATCH)
        .fetch_all(pool)
        .await?;
        if rows.is_empty() {
            break;
        }
        for row in rows {
            let id: String = row.try_get("id")?;
            let key = SecretKey::new(
                row.try_get::<String, _>("owner_type")?,
                row.try_get::<String, _>("owner_id")?,
                row.try_get::<String, _>("name")?,
            );
            let aad = context(&key);
            let plaintext = keyring.decrypt(
                &EncryptedValue {
                    key_id: row.try_get("key_id")?,
                    nonce: row.try_get("nonce")?,
                    ciphertext: row.try_get("ciphertext")?,
                },
                &aad,
            )?;
            let encrypted = keyring.encrypt(&plaintext, &aad)?;
            sqlx::query("UPDATE secrets SET key_id = ?, nonce = ?, ciphertext = ? WHERE id = ?")
                .bind(&encrypted.key_id)
                .bind(&encrypted.nonce)
                .bind(&encrypted.ciphertext)
                .bind(&id)
                .execute(pool)
                .await?;
            report.rotated += 1;
        }
    }
    report.rotated += rotate_installation_secrets(pool, keyring, &current).await?;
    let total: i64 = sqlx::query_scalar(
        "SELECT (SELECT COUNT(*) FROM secrets) + (SELECT COUNT(*) FROM installation_secrets)",
    )
    .fetch_one(pool)
    .await?;
    report.current = u64::try_from(total).unwrap_or_default();
    Ok(report)
}
