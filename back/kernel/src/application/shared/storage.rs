use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use bytes::Bytes;
use futures::Stream;

use crate::errors::AppError;

pub type ByteStream = Pin<Box<dyn Stream<Item = Result<Bytes, AppError>> + Send>>;

#[async_trait]
pub trait FileStorage: Send + Sync {
    async fn put(&self, key: &str, content: Bytes) -> Result<(), AppError>;
    async fn get_stream(&self, key: &str) -> Result<ByteStream, AppError>;
    async fn delete(&self, key: &str) -> Result<(), AppError>;

    async fn put_stream(&self, key: &str, mut content: ByteStream) -> Result<(), AppError> {
        use futures::StreamExt;

        let mut buffer = Vec::new();
        while let Some(chunk) = content.next().await {
            buffer.extend_from_slice(&chunk?);
        }
        self.put(key, Bytes::from(buffer)).await
    }

    async fn presigned_get(&self, _key: &str, _ttl: Duration) -> Result<Option<String>, AppError> {
        Ok(None)
    }
}

pub type SharedFileStorage = Arc<dyn FileStorage>;
