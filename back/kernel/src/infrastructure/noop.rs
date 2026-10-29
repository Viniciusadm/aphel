use async_trait::async_trait;
use bytes::Bytes;

use crate::application::secrets::{SecretKey, SecretVault, encryption_key_missing};
use crate::application::shared::{
    AuditEntry, AuditLog, ByteStream, DomainEvent, EnqueueOutcome, EventPublisher, FileStorage,
    Job, JobQueue,
};
use crate::domain::Secret;
use crate::domain::access::AccessScope;
use crate::errors::AppError;

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct NoopEventPublisher;

#[async_trait]
impl<Tx: Send> EventPublisher<Tx> for NoopEventPublisher {
    async fn publish(&self, _tx: &mut Tx, _event: DomainEvent) -> Result<(), AppError> {
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct NoopAuditLog;

#[async_trait]
impl<Tx: Send> AuditLog<Tx> for NoopAuditLog {
    async fn record(&self, _tx: &mut Tx, _entry: AuditEntry) -> Result<(), AppError> {
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct NoopJobQueue;

#[async_trait]
impl<Tx: Send> JobQueue<Tx> for NoopJobQueue {
    async fn enqueue(&self, _tx: &mut Tx, _job: Job) -> Result<EnqueueOutcome, AppError> {
        Ok(EnqueueOutcome::Duplicate)
    }

    async fn enqueue_now(&self, _job: Job) -> Result<EnqueueOutcome, AppError> {
        Ok(EnqueueOutcome::Duplicate)
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct UnavailableFileStorage;

fn storage_unavailable() -> AppError {
    AppError::Configuration("file storage is not configured".to_owned())
}

#[async_trait]
impl FileStorage for UnavailableFileStorage {
    async fn put(&self, _key: &str, _content: Bytes) -> Result<(), AppError> {
        Err(storage_unavailable())
    }

    async fn get_stream(&self, _key: &str) -> Result<ByteStream, AppError> {
        Err(storage_unavailable())
    }

    async fn delete(&self, _key: &str) -> Result<(), AppError> {
        Err(storage_unavailable())
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct UnavailableSecretVault;

#[async_trait]
impl<Tx: Send> SecretVault<Tx> for UnavailableSecretVault {
    async fn store(
        &self,
        _tx: &mut Tx,
        _scope: &AccessScope,
        _key: &SecretKey,
        _value: &Secret,
    ) -> Result<(), AppError> {
        Err(encryption_key_missing())
    }

    async fn reveal(
        &self,
        _scope: &AccessScope,
        _key: &SecretKey,
    ) -> Result<Option<Secret>, AppError> {
        Err(encryption_key_missing())
    }

    async fn remove(
        &self,
        _tx: &mut Tx,
        _scope: &AccessScope,
        _key: &SecretKey,
    ) -> Result<(), AppError> {
        Ok(())
    }

    async fn remove_owner(
        &self,
        _tx: &mut Tx,
        _scope: &AccessScope,
        _owner_type: &str,
        _owner_id: &str,
    ) -> Result<(), AppError> {
        Ok(())
    }

    async fn configured(
        &self,
        _scope: &AccessScope,
        _owner_type: &str,
        _owner_ids: &[String],
    ) -> Result<Vec<SecretKey>, AppError> {
        Ok(Vec::new())
    }
}
