use std::sync::Arc;

use async_trait::async_trait;
use base64::Engine;
use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use ring::rand::SystemRandom;
use ring::signature::{ECDSA_P256_SHA256_FIXED_SIGNING, EcdsaKeyPair, KeyPair};
use tokio::sync::OnceCell;

use crate::application::notifications::push::VapidKeys;
use crate::application::secrets::{SecretKey, SecretVault};
use crate::domain::Secret;
use crate::errors::AppError;
use crate::infrastructure::platform::SharedSecretVault;

pub(crate) const VAPID_OWNER_TYPE: &str = "web_push";
pub(crate) const VAPID_OWNER_ID: &str = "vapid";
pub(crate) const VAPID_PRIVATE_KEY_NAME: &str = "private_key_pkcs8";

static VAPID_KEY_PAIR: OnceCell<Arc<VapidKeyPair>> = OnceCell::const_new();

pub(crate) struct VapidKeyPair {
    pair: EcdsaKeyPair,
    public_key: String,
}

impl VapidKeyPair {
    fn from_pkcs8(encoded: &str) -> Result<Self, AppError> {
        let pkcs8 = STANDARD
            .decode(encoded.trim())
            .map_err(|_| AppError::Internal("stored VAPID key is not valid base64".to_owned()))?;
        let pair = EcdsaKeyPair::from_pkcs8(
            &ECDSA_P256_SHA256_FIXED_SIGNING,
            &pkcs8,
            &SystemRandom::new(),
        )
        .map_err(|_| AppError::Internal("stored VAPID key is invalid".to_owned()))?;
        let public_key = URL_SAFE_NO_PAD.encode(pair.public_key().as_ref());
        Ok(Self { pair, public_key })
    }

    pub(crate) fn public_key(&self) -> &str {
        &self.public_key
    }

    pub(crate) fn sign(&self, message: &[u8]) -> Result<Vec<u8>, AppError> {
        self.pair
            .sign(&SystemRandom::new(), message)
            .map(|signature| signature.as_ref().to_vec())
            .map_err(|_| AppError::Internal("VAPID signature failed".to_owned()))
    }
}

fn vapid_secret_key() -> SecretKey {
    SecretKey::new(VAPID_OWNER_TYPE, VAPID_OWNER_ID, VAPID_PRIVATE_KEY_NAME)
}

fn generate_pkcs8() -> Result<String, AppError> {
    let document =
        EcdsaKeyPair::generate_pkcs8(&ECDSA_P256_SHA256_FIXED_SIGNING, &SystemRandom::new())
            .map_err(|_| AppError::Internal("VAPID key generation failed".to_owned()))?;
    Ok(STANDARD.encode(document.as_ref()))
}

async fn load_or_create(secrets: &SharedSecretVault) -> Result<Arc<VapidKeyPair>, AppError> {
    let key = vapid_secret_key();
    if let Some(stored) = secrets.reveal_installation(&key).await? {
        return VapidKeyPair::from_pkcs8(stored.expose()).map(Arc::new);
    }
    let generated = Secret::new(generate_pkcs8()?);
    if secrets
        .store_installation_if_absent(&key, &generated)
        .await?
    {
        tracing::info!("generated the installation VAPID key pair for web push");
        return VapidKeyPair::from_pkcs8(generated.expose()).map(Arc::new);
    }
    let stored = secrets
        .reveal_installation(&key)
        .await?
        .ok_or_else(|| AppError::Internal("VAPID key could not be stored".to_owned()))?;
    VapidKeyPair::from_pkcs8(stored.expose()).map(Arc::new)
}

#[derive(Clone)]
pub(crate) struct VaultVapidKeys {
    secrets: SharedSecretVault,
}

impl VaultVapidKeys {
    pub(crate) fn new(secrets: SharedSecretVault) -> Self {
        Self { secrets }
    }

    pub(crate) async fn key_pair(&self) -> Result<Arc<VapidKeyPair>, AppError> {
        VAPID_KEY_PAIR
            .get_or_try_init(|| load_or_create(&self.secrets))
            .await
            .cloned()
    }
}

#[async_trait]
impl VapidKeys for VaultVapidKeys {
    async fn public_key(&self) -> Result<String, AppError> {
        Ok(self.key_pair().await?.public_key().to_owned())
    }
}
