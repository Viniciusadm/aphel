use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::{Map, Value};
use sqlx::MySqlPool;

use crate::Ctx;
use crate::application::files::GeneratedFileKind;
use crate::application::system::FailureEntry;
use crate::errors::AppError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsTestOutcome {
    pub ok: bool,
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetupHint {
    pub key: &'static str,
    pub value: String,
}

#[async_trait]
pub trait ModuleSettingsTester: Send + Sync {
    fn module(&self) -> &'static str;

    fn can_test(&self) -> bool {
        true
    }

    fn has_hints(&self) -> bool {
        false
    }

    async fn test(
        &self,
        _ctx: &Ctx<'_>,
        _settings: &Map<String, Value>,
    ) -> Result<SettingsTestOutcome, AppError> {
        Err(AppError::not_found_code(
            "module_test_unsupported",
            "this module has no settings test",
        ))
    }

    async fn hints(&self, _ctx: &Ctx<'_>) -> Result<Vec<SetupHint>, AppError> {
        Ok(Vec::new())
    }
}

#[async_trait]
pub trait FileReferenceSource: Send + Sync {
    async fn reference_kind(
        &self,
        pool: &MySqlPool,
        file_id: &str,
    ) -> Result<Option<GeneratedFileKind>, AppError>;
}

#[async_trait]
pub trait HealthSource: Send + Sync {
    async fn recent_failures(
        &self,
        pool: &MySqlPool,
        since: DateTime<Utc>,
        limit: u64,
    ) -> Result<Vec<FailureEntry>, AppError>;
}

#[async_trait]
pub trait ReferenceProvider: Send + Sync {
    fn kind(&self) -> aphel_contract::ReferenceKind;

    async fn exists(&self, ctx: &crate::Ctx<'_>, id: &str) -> Result<bool, AppError>;

    async fn stable_key(&self, ctx: &crate::Ctx<'_>, id: &str) -> Result<Option<String>, AppError> {
        if !self.exists(ctx, id).await? {
            return Ok(None);
        }
        for source in crate::modules::app().contributions::<dyn ReferenceKeySource>() {
            if let Some(key) = source.stable_key(ctx, self.kind(), id).await? {
                return Ok(Some(key));
            }
        }
        Ok(None)
    }

    async fn resolve_key(
        &self,
        ctx: &crate::Ctx<'_>,
        key: &str,
    ) -> Result<Option<String>, AppError> {
        for source in crate::modules::app().contributions::<dyn ReferenceKeySource>() {
            if let Some(id) = source.resolve_key(ctx, self.kind(), key).await?
                && self.exists(ctx, &id).await?
            {
                return Ok(Some(id));
            }
        }
        Ok(None)
    }
}

pub fn lookup_exists<T>(result: Result<T, AppError>) -> Result<bool, AppError> {
    use crate::errors::CodedKind;
    match result {
        Ok(_) => Ok(true),
        Err(
            AppError::NotFound(_)
            | AppError::Forbidden(_)
            | AppError::Coded {
                kind: CodedKind::NotFound | CodedKind::Forbidden,
                ..
            },
        ) => Ok(false),
        Err(error) => Err(error),
    }
}

pub trait HttpRequestPolicy: Send + Sync {
    fn check(&self, method: &axum::http::Method, path: &str) -> Result<(), AppError>;
}

#[async_trait]
pub trait ReferenceKeySource: Send + Sync {
    async fn stable_key(
        &self,
        ctx: &crate::Ctx<'_>,
        kind: aphel_contract::ReferenceKind,
        id: &str,
    ) -> Result<Option<String>, AppError>;
    async fn resolve_key(
        &self,
        ctx: &crate::Ctx<'_>,
        kind: aphel_contract::ReferenceKind,
        key: &str,
    ) -> Result<Option<String>, AppError>;
}
