use std::fs;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use bytes::Bytes;
use futures::{StreamExt, TryStreamExt, stream};
use object_store::aws::AmazonS3Builder;
use object_store::local::LocalFileSystem;
use object_store::path::Path;
use object_store::signer::{Method, Signer};
use object_store::{
    GetOptions, ObjectStore, PutMultipartOptions, PutOptions, PutPayload, WriteMultipart,
};

use crate::application::organization_profile::{ObjectStoreSettings, StorageSettings};
use crate::application::shared::{ByteStream, FileStorage, SharedFileStorage};
use crate::domain::Secret;
use crate::errors::AppError;

pub struct ObjectFileStorage {
    store: Arc<dyn ObjectStore>,
    signer: Option<Arc<dyn Signer>>,
}

impl ObjectFileStorage {
    pub(crate) fn new(store: Arc<dyn ObjectStore>, signer: Option<Arc<dyn Signer>>) -> Self {
        Self { store, signer }
    }

    pub(crate) fn local(path: &str) -> Result<Self, AppError> {
        fs::create_dir_all(path).map_err(|error| {
            AppError::Configuration(format!("cannot create storage directory {path}: {error}"))
        })?;
        let store = LocalFileSystem::new_with_prefix(path)
            .map_err(|error| {
                AppError::Configuration(format!("invalid storage directory {path}: {error}"))
            })?
            .with_automatic_cleanup(true);
        Ok(Self::new(Arc::new(store), None))
    }

    pub(crate) fn s3(
        config: &ObjectStoreSettings,
        secret: Option<&Secret>,
    ) -> Result<Self, AppError> {
        let mut builder = AmazonS3Builder::new()
            .with_bucket_name(&config.bucket)
            .with_region(&config.region)
            .with_allow_http(config.allow_http)
            .with_virtual_hosted_style_request(!config.force_path_style);
        if let Some(endpoint) = &config.endpoint {
            builder = builder.with_endpoint(endpoint);
        }
        if let Some(access_key_id) = &config.access_key_id {
            builder = builder.with_access_key_id(access_key_id);
        }
        if let Some(secret_access_key) = secret {
            builder = builder.with_secret_access_key(secret_access_key.expose());
        }
        let store = Arc::new(builder.build().map_err(|error| {
            AppError::Configuration(format!("invalid S3 storage configuration: {error}"))
        })?);
        Ok(Self::new(store.clone(), Some(store)))
    }
}

pub fn build_file_storage(
    settings: &StorageSettings,
    secret: Option<&Secret>,
) -> Result<SharedFileStorage, AppError> {
    let storage = match settings {
        StorageSettings::Local { path } => ObjectFileStorage::local(path)?,
        StorageSettings::S3(s3) => ObjectFileStorage::s3(s3, secret)?,
    };
    Ok(Arc::new(storage))
}

fn location(key: &str) -> Result<Path, AppError> {
    Path::parse(key)
        .ok()
        .filter(|path| !key.is_empty() && path.as_ref() == key)
        .ok_or_else(|| AppError::Internal("invalid storage key".to_owned()))
}

const MULTIPART_CHUNK_BYTES: usize = 8 * 1024 * 1024;
const MULTIPART_CONCURRENCY: usize = 4;

async fn abort_upload(writer: WriteMultipart) {
    if let Err(error) = writer.abort().await {
        tracing::warn!(error = %error, "failed to abort a multipart upload");
    }
}

fn storage_error(error: object_store::Error) -> AppError {
    match error {
        object_store::Error::NotFound { .. } => {
            AppError::NotFound("file content was not found".to_owned())
        }
        other => AppError::Internal(format!("file storage failed: {other}")),
    }
}

#[async_trait]
impl FileStorage for ObjectFileStorage {
    async fn put(&self, key: &str, content: Bytes) -> Result<(), AppError> {
        self.store
            .put_opts(
                &location(key)?,
                PutPayload::from(content),
                PutOptions::default(),
            )
            .await
            .map_err(storage_error)?;
        Ok(())
    }

    async fn delete_prefix(&self, prefix: &str) -> Result<u64, AppError> {
        let prefix = location(prefix)?;
        let listed = self
            .store
            .list(Some(&prefix))
            .map_ok(|meta| meta.location)
            .boxed();
        let mut deleted = self.store.delete_stream(listed);
        let mut count = 0;
        while let Some(result) = deleted.next().await {
            result.map_err(storage_error)?;
            count += 1;
        }
        Ok(count)
    }

    async fn put_stream(&self, key: &str, mut content: ByteStream) -> Result<(), AppError> {
        let upload = self
            .store
            .put_multipart_opts(&location(key)?, PutMultipartOptions::default())
            .await
            .map_err(storage_error)?;
        let mut writer = WriteMultipart::new_with_chunk_size(upload, MULTIPART_CHUNK_BYTES);
        while let Some(chunk) = content.next().await {
            let chunk = match chunk {
                Ok(chunk) => chunk,
                Err(error) => {
                    abort_upload(writer).await;
                    return Err(error);
                }
            };
            if let Err(error) = writer.wait_for_capacity(MULTIPART_CONCURRENCY).await {
                abort_upload(writer).await;
                return Err(storage_error(error));
            }
            writer.put(chunk);
        }
        writer.finish().await.map_err(storage_error)?;
        Ok(())
    }

    async fn get_stream(&self, key: &str) -> Result<ByteStream, AppError> {
        let result = self
            .store
            .get_opts(&location(key)?, GetOptions::default())
            .await
            .map_err(storage_error)?;
        Ok(Box::pin(result.into_stream().map_err(storage_error)))
    }

    async fn delete(&self, key: &str) -> Result<(), AppError> {
        let path = location(key)?;
        let mut deleted = self
            .store
            .delete_stream(stream::once(async move { Ok(path) }).boxed());
        while let Some(result) = deleted.next().await {
            match result {
                Ok(_) | Err(object_store::Error::NotFound { .. }) => {}
                Err(error) => return Err(storage_error(error)),
            }
        }
        Ok(())
    }

    async fn presigned_get(&self, key: &str, ttl: Duration) -> Result<Option<String>, AppError> {
        let Some(signer) = &self.signer else {
            return Ok(None);
        };
        let url = signer
            .signed_url(Method::GET, &location(key)?, ttl)
            .await
            .map_err(storage_error)?;
        Ok(Some(url.to_string()))
    }
}
