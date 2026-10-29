use argon2::Argon2;
use argon2::password_hash::phc::PasswordHash;
use argon2::password_hash::{PasswordHasher as ArgonPasswordHasher, PasswordVerifier};
use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};

use crate::application::security::{Clock, IssuedToken, PasswordHasher, SecureTokens};
use crate::domain::Secret;
use crate::errors::AppError;

const TOKEN_BYTES: usize = 32;

#[derive(Debug, Clone, Copy, Default)]
pub struct Argon2PasswordHasher;

impl PasswordHasher for Argon2PasswordHasher {
    fn hash_password(&self, password: &str) -> Result<String, AppError> {
        Argon2::default()
            .hash_password(password.as_bytes())
            .map(|hash| hash.to_string())
            .map_err(|error| AppError::Internal(format!("password hash failed: {error}")))
    }

    fn verify_password(&self, password: &str, password_hash: &str) -> Result<bool, AppError> {
        let Ok(parsed_hash) = PasswordHash::new(password_hash) else {
            return Ok(false);
        };
        Ok(Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_ok())
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Sha256SecureTokens;

impl SecureTokens for Sha256SecureTokens {
    fn issue(&self) -> Result<IssuedToken, AppError> {
        let mut bytes = [0u8; TOKEN_BYTES];
        getrandom::fill(&mut bytes)
            .map_err(|error| AppError::Internal(format!("random generator failed: {error}")))?;
        let token = to_hex(&bytes);
        let hash = self.digest(&token);
        Ok(IssuedToken {
            token: Secret::new(token),
            hash,
        })
    }

    fn digest(&self, token: &str) -> String {
        to_hex(&Sha256::digest(token.as_bytes()))
    }
}

fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }
}
