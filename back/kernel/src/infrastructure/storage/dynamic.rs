use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use bytes::Bytes;
use futures::StreamExt;
use uuid::Uuid;

use crate::application::organization_profile::{
    OrganizationConfig, OrganizationConfigCatalog, StorageProbe, StorageSettings,
    storage_secret_key,
};
use crate::application::shared::{ByteStream, FileStorage, SharedFileStorage};
use crate::domain::Secret;
use crate::domain::access::AccessScope;
use crate::errors::AppError;
use crate::infrastructure::platform::SharedSecretVault;
use crate::infrastructure::storage::build_file_storage;

const PROBE_TIMEOUT: Duration = Duration::from_secs(20);

pub struct DynamicFileStorage {
    catalog: Arc<OrganizationConfigCatalog>,
    vault: SharedSecretVault,
    cache: Mutex<Option<(Arc<OrganizationConfig>, SharedFileStorage)>>,
}

impl DynamicFileStorage {
    pub(crate) fn new(catalog: Arc<OrganizationConfigCatalog>, vault: SharedSecretVault) -> Self {
        Self {
            catalog,
            vault,
            cache: Mutex::new(None),
        }
    }

    pub fn shared(
        catalog: Arc<OrganizationConfigCatalog>,
        vault: SharedSecretVault,
    ) -> SharedFileStorage {
        Arc::new(Self::new(catalog, vault))
    }

    async fn current(&self) -> Result<SharedFileStorage, AppError> {
        let config = self.catalog.current().await?;
        let cached = self.cache.lock().ok().and_then(|cache| cache.clone());
        if let Some((cached_config, storage)) = cached
            && (Arc::ptr_eq(&cached_config, &config) || *cached_config == *config)
        {
            return Ok(storage);
        }
        let secret = match &config.storage {
            StorageSettings::S3(_) => {
                self.vault
                    .reveal(&AccessScope::system(), &storage_secret_key())
                    .await?
            }
            StorageSettings::Local { .. } => None,
        };
        let storage = build_file_storage(&config.storage, secret.as_ref())?;
        if let Ok(mut cache) = self.cache.lock() {
            *cache = Some((config, storage.clone()));
        }
        Ok(storage)
    }
}

#[async_trait]
impl FileStorage for DynamicFileStorage {
    async fn put(&self, key: &str, content: Bytes) -> Result<(), AppError> {
        self.current().await?.put(key, content).await
    }

    async fn get_stream(&self, key: &str) -> Result<ByteStream, AppError> {
        self.current().await?.get_stream(key).await
    }

    async fn delete(&self, key: &str) -> Result<(), AppError> {
        self.current().await?.delete(key).await
    }

    async fn put_stream(&self, key: &str, content: ByteStream) -> Result<(), AppError> {
        self.current().await?.put_stream(key, content).await
    }

    async fn presigned_get(&self, key: &str, ttl: Duration) -> Result<Option<String>, AppError> {
        self.current().await?.presigned_get(key, ttl).await
    }
}

pub async fn probe_file_storage(storage: &SharedFileStorage) -> Result<(), AppError> {
    let key = format!("system/probe/{}", Uuid::now_v7().simple());
    let payload = Bytes::from_static(b"aphel storage probe");
    storage.put(&key, payload.clone()).await?;
    let mut stream = storage.get_stream(&key).await?;
    let mut read = Vec::new();
    while let Some(chunk) = stream.next().await {
        read.extend_from_slice(&chunk?);
    }
    storage.delete(&key).await?;
    if read != payload {
        return Err(AppError::Internal(
            "the stored object did not match what was written".to_owned(),
        ));
    }
    Ok(())
}

#[derive(Clone, Default)]
pub struct ObjectStorageProbe;

#[async_trait]
impl StorageProbe for ObjectStorageProbe {
    async fn probe(
        &self,
        settings: &StorageSettings,
        secret: Option<&Secret>,
    ) -> Result<(), String> {
        let storage = build_file_storage(settings, secret)
            .map_err(|error| error.client_message().to_owned())?;
        match tokio::time::timeout(PROBE_TIMEOUT, probe_file_storage(&storage)).await {
            Ok(Ok(())) => Ok(()),
            Ok(Err(error)) => Err(error.client_message().to_owned()),
            Err(_) => Err("the storage did not answer in time".to_owned()),
        }
    }
}
