use std::sync::Arc;

use async_trait::async_trait;

use crate::domain::Secret;
use crate::domain::access::AccessScope;
use crate::errors::AppError;

pub const MAX_SECRET_BYTES: usize = 8 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SecretKey {
    pub owner_type: String,
    pub owner_id: String,
    pub name: String,
}

impl SecretKey {
    pub const ORGANIZATION_OWNER: &'static str = "organization";

    pub fn new(
        owner_type: impl Into<String>,
        owner_id: impl Into<String>,
        name: impl Into<String>,
    ) -> Self {
        Self {
            owner_type: owner_type.into(),
            owner_id: owner_id.into(),
            name: name.into(),
        }
    }

    pub fn organization(name: impl Into<String>) -> Self {
        Self::new(
            Self::ORGANIZATION_OWNER,
            crate::domain::organizations::ORGANIZATION_RECORD_ID,
            name,
        )
    }
}

#[async_trait]
pub trait SecretVault<Tx: Send>: Send + Sync {
    async fn store(
        &self,
        tx: &mut Tx,
        scope: &AccessScope,
        key: &SecretKey,
        value: &Secret,
    ) -> Result<(), AppError>;

    async fn reveal(
        &self,
        scope: &AccessScope,
        key: &SecretKey,
    ) -> Result<Option<Secret>, AppError>;

    async fn remove(
        &self,
        tx: &mut Tx,
        scope: &AccessScope,
        key: &SecretKey,
    ) -> Result<(), AppError>;

    async fn remove_owner(
        &self,
        tx: &mut Tx,
        scope: &AccessScope,
        owner_type: &str,
        owner_id: &str,
    ) -> Result<(), AppError>;

    async fn configured(
        &self,
        scope: &AccessScope,
        owner_type: &str,
        owner_ids: &[String],
    ) -> Result<Vec<SecretKey>, AppError>;

    async fn reveal_installation(&self, _key: &SecretKey) -> Result<Option<Secret>, AppError> {
        Err(encryption_key_missing())
    }

    async fn store_installation_if_absent(
        &self,
        _key: &SecretKey,
        _value: &Secret,
    ) -> Result<bool, AppError> {
        Err(encryption_key_missing())
    }

    async fn store_organization_if_absent(
        &self,
        _scope: &AccessScope,
        _key: &SecretKey,
        _value: &Secret,
    ) -> Result<bool, AppError> {
        Err(encryption_key_missing())
    }
}

#[async_trait]
impl<Tx, V> SecretVault<Tx> for Arc<V>
where
    Tx: Send,
    V: SecretVault<Tx> + ?Sized,
{
    async fn store(
        &self,
        tx: &mut Tx,
        scope: &AccessScope,
        key: &SecretKey,
        value: &Secret,
    ) -> Result<(), AppError> {
        (**self).store(tx, scope, key, value).await
    }

    async fn reveal(
        &self,
        scope: &AccessScope,
        key: &SecretKey,
    ) -> Result<Option<Secret>, AppError> {
        (**self).reveal(scope, key).await
    }

    async fn remove(
        &self,
        tx: &mut Tx,
        scope: &AccessScope,
        key: &SecretKey,
    ) -> Result<(), AppError> {
        (**self).remove(tx, scope, key).await
    }

    async fn remove_owner(
        &self,
        tx: &mut Tx,
        scope: &AccessScope,
        owner_type: &str,
        owner_id: &str,
    ) -> Result<(), AppError> {
        (**self).remove_owner(tx, scope, owner_type, owner_id).await
    }

    async fn configured(
        &self,
        scope: &AccessScope,
        owner_type: &str,
        owner_ids: &[String],
    ) -> Result<Vec<SecretKey>, AppError> {
        (**self).configured(scope, owner_type, owner_ids).await
    }

    async fn reveal_installation(&self, key: &SecretKey) -> Result<Option<Secret>, AppError> {
        (**self).reveal_installation(key).await
    }

    async fn store_installation_if_absent(
        &self,
        key: &SecretKey,
        value: &Secret,
    ) -> Result<bool, AppError> {
        (**self).store_installation_if_absent(key, value).await
    }

    async fn store_organization_if_absent(
        &self,
        scope: &AccessScope,
        key: &SecretKey,
        value: &Secret,
    ) -> Result<bool, AppError> {
        (**self)
            .store_organization_if_absent(scope, key, value)
            .await
    }
}

pub fn encryption_key_missing() -> AppError {
    AppError::Configuration("APP_ENCRYPTION_KEY is not configured".to_owned())
}
