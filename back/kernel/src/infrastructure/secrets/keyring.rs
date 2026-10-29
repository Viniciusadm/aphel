use aes_gcm::Aes256Gcm;
use aes_gcm::aead::{Aead, KeyInit, Payload};
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use sha2::{Digest, Sha256};

use crate::application::secrets::encryption_key_missing;
use crate::config::EncryptionConfig;
use crate::domain::Secret;
use crate::errors::AppError;

const KEY_BYTES: usize = 32;
const NONCE_BYTES: usize = 12;
const KEY_ID_LENGTH: usize = 16;

#[derive(Clone)]
struct EncryptionKey {
    id: String,
    cipher: Aes256Gcm,
}

#[derive(Clone, Default)]
pub struct EncryptionKeyring {
    current: Option<EncryptionKey>,
    previous: Vec<EncryptionKey>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncryptedValue {
    pub(crate) key_id: String,
    pub(crate) nonce: Vec<u8>,
    pub(crate) ciphertext: Vec<u8>,
}

impl EncryptionKeyring {
    pub fn from_config(config: &EncryptionConfig) -> Result<Self, AppError> {
        let current = config
            .key
            .as_ref()
            .map(|key| parse_key("APP_ENCRYPTION_KEY", key))
            .transpose()?;
        let previous = config
            .previous_keys
            .iter()
            .map(|key| parse_key("APP_ENCRYPTION_PREVIOUS_KEYS", key))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self { current, previous })
    }

    pub fn is_configured(&self) -> bool {
        self.current.is_some()
    }

    pub(crate) fn current_key_id(&self) -> Option<&str> {
        self.current.as_ref().map(|key| key.id.as_str())
    }

    pub(crate) fn encrypt(
        &self,
        plaintext: &[u8],
        context: &[u8],
    ) -> Result<EncryptedValue, AppError> {
        let key = self.current.as_ref().ok_or_else(encryption_key_missing)?;
        let mut nonce = [0u8; NONCE_BYTES];
        getrandom::fill(&mut nonce)
            .map_err(|error| AppError::Internal(format!("failed to generate nonce: {error}")))?;
        let ciphertext = key
            .cipher
            .encrypt(
                &nonce.into(),
                Payload {
                    msg: plaintext,
                    aad: context,
                },
            )
            .map_err(|_| AppError::Internal("failed to encrypt secret".to_owned()))?;
        Ok(EncryptedValue {
            key_id: key.id.clone(),
            nonce: nonce.to_vec(),
            ciphertext,
        })
    }

    pub(crate) fn decrypt(
        &self,
        value: &EncryptedValue,
        context: &[u8],
    ) -> Result<Vec<u8>, AppError> {
        let key = self
            .current
            .iter()
            .chain(self.previous.iter())
            .find(|key| key.id == value.key_id)
            .ok_or_else(|| {
                AppError::Configuration(format!(
                    "secret was encrypted with key {} that is not in APP_ENCRYPTION_KEY or APP_ENCRYPTION_PREVIOUS_KEYS",
                    value.key_id
                ))
            })?;
        let nonce: [u8; NONCE_BYTES] = value
            .nonce
            .as_slice()
            .try_into()
            .map_err(|_| AppError::Internal("stored secret has an invalid nonce".to_owned()))?;
        key.cipher
            .decrypt(
                &nonce.into(),
                Payload {
                    msg: &value.ciphertext,
                    aad: context,
                },
            )
            .map_err(|_| AppError::Internal("failed to decrypt secret".to_owned()))
    }
}

fn parse_key(variable: &str, value: &Secret) -> Result<EncryptionKey, AppError> {
    let bytes = STANDARD.decode(value.expose().trim()).map_err(|_| {
        AppError::Configuration(format!("{variable} must be a base64 encoded 32-byte key"))
    })?;
    let invalid =
        || AppError::Configuration(format!("{variable} must be a base64 encoded 32-byte key"));
    if bytes.len() != KEY_BYTES {
        return Err(invalid());
    }
    let digest = Sha256::digest(&bytes);
    let id: String = digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>()
        .chars()
        .take(KEY_ID_LENGTH)
        .collect();
    Ok(EncryptionKey {
        id,
        cipher: Aes256Gcm::new_from_slice(&bytes).map_err(|_| invalid())?,
    })
}
