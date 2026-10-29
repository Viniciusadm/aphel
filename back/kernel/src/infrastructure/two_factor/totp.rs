use chrono::{DateTime, Utc};
use qrcode::QrCode;
use qrcode::render::svg;
use totp_rs::{Builder, Totp};

use crate::application::auth::two_factor::{OneTimePasswords, RECOVERY_CODE_LENGTH, TOTP_DIGITS};
use crate::domain::Secret;
use crate::errors::AppError;

const SECRET_BYTES: usize = 20;
const STEP_SECONDS: u64 = 30;
const ALLOWED_SKEW: u16 = 1;
const QR_CODE_SIZE: u32 = 220;
const RECOVERY_ALPHABET: &[u8] = b"abcdefghjkmnpqrstuvwxyz23456789";

#[derive(Debug, Clone, Copy, Default)]
pub struct TotpPasswords;

impl TotpPasswords {
    fn totp(&self, secret: &Secret, account: Option<(&str, &str)>) -> Result<Totp, AppError> {
        let bytes = totp_rs::Secret::try_from_base32(secret.expose())
            .map_err(|_| AppError::Internal("stored TOTP secret is not valid base32".to_owned()))?;
        let mut builder = Builder::new()
            .with_secret(bytes)
            .with_digits(TOTP_DIGITS as u8)
            .with_skew(ALLOWED_SKEW)
            .with_step_duration(STEP_SECONDS);
        if let Some((account, issuer)) = account {
            builder = builder.with_account_name(account).with_issuer(Some(issuer));
        }
        builder
            .build()
            .map_err(|error| AppError::Internal(format!("invalid TOTP configuration: {error}")))
    }
}

fn random_bytes(length: usize) -> Result<Vec<u8>, AppError> {
    let mut bytes = vec![0u8; length];
    getrandom::fill(&mut bytes)
        .map_err(|error| AppError::Internal(format!("failed to generate random bytes: {error}")))?;
    Ok(bytes)
}

impl OneTimePasswords for TotpPasswords {
    fn generate_secret(&self) -> Result<Secret, AppError> {
        let secret = totp_rs::Secret::from(random_bytes(SECRET_BYTES)?);
        Ok(Secret::new(secret.to_base32()))
    }

    fn provisioning_uri(
        &self,
        secret: &Secret,
        account: &str,
        issuer: &str,
    ) -> Result<String, AppError> {
        let account = account.replace(':', "");
        self.totp(secret, Some((&account, issuer)))?
            .to_url()
            .map_err(|error| AppError::Internal(format!("failed to build otpauth URI: {error}")))
    }

    fn qr_code_svg(&self, uri: &str) -> Result<String, AppError> {
        let code = QrCode::new(uri.as_bytes())
            .map_err(|error| AppError::Internal(format!("failed to build QR code: {error}")))?;
        Ok(code
            .render::<svg::Color<'_>>()
            .min_dimensions(QR_CODE_SIZE, QR_CODE_SIZE)
            .quiet_zone(true)
            .build())
    }

    fn matching_step(
        &self,
        secret: &Secret,
        code: &str,
        at: DateTime<Utc>,
    ) -> Result<Option<u64>, AppError> {
        let timestamp = u64::try_from(at.timestamp()).unwrap_or_default();
        Ok(self.totp(secret, None)?.check(code, timestamp))
    }

    fn recovery_code(&self) -> Result<Secret, AppError> {
        let limit = (u8::MAX as usize / RECOVERY_ALPHABET.len()) * RECOVERY_ALPHABET.len();
        let mut characters = String::with_capacity(RECOVERY_CODE_LENGTH + 1);
        while characters.len() < RECOVERY_CODE_LENGTH {
            for byte in random_bytes(RECOVERY_CODE_LENGTH * 2)? {
                if usize::from(byte) >= limit || characters.len() == RECOVERY_CODE_LENGTH {
                    continue;
                }
                characters.push(char::from(
                    RECOVERY_ALPHABET[usize::from(byte) % RECOVERY_ALPHABET.len()],
                ));
            }
        }
        let (head, tail) = characters.split_at(RECOVERY_CODE_LENGTH / 2);
        Ok(Secret::new(format!("{head}-{tail}")))
    }
}
