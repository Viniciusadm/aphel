use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, NaiveDateTime, Utc};
use serde_json::Value;
use sqlx::{MySqlPool, Row, mysql::MySqlRow, types::Json};

use crate::application::fields::RecordTypeRegistry;
use crate::application::organization_profile::{
    OrganizationExperiences, OrganizationExperiencesPorts, OrganizationExperiencesRepository,
    StoredOrganizationExperiences, validate_organization_experiences,
};
use crate::database::transaction::MySqlTx;
use crate::errors::AppError;
use crate::infrastructure::auth::SystemClock;
use crate::infrastructure::platform::{SharedAuditLog, SharedEventPublisher};
use crate::infrastructure::transactions::MySqlTransactionManager;

fn decode(row: MySqlRow) -> Result<StoredOrganizationExperiences, AppError> {
    let kind: Option<String> = row.try_get("settings_kind")?;
    if kind
        .as_deref()
        .is_some_and(|kind| !matches!(kind, "OBJECT" | "NULL"))
    {
        return Err(AppError::Internal(
            "organization settings must be a JSON object".to_owned(),
        ));
    }
    let value: Option<Json<Value>> = row.try_get("experiences")?;
    let configuration = match value {
        Some(Json(Value::Null)) | None => None,
        Some(Json(value)) => {
            let configuration: OrganizationExperiences =
                serde_json::from_value(value).map_err(|error| {
                    AppError::Internal(format!("invalid stored organization experiences: {error}"))
                })?;
            if !validate_organization_experiences(&configuration).is_empty() {
                return Err(AppError::Internal(
                    "invalid stored organization experiences".to_owned(),
                ));
            }
            Some(configuration)
        }
    };
    let updated_at: NaiveDateTime = row.try_get("updated_at")?;
    Ok(StoredOrganizationExperiences {
        configuration,
        updated_at: updated_at.and_utc(),
    })
}

pub struct MySqlOrganizationExperiencesRepository {
    pool: MySqlPool,
}

impl MySqlOrganizationExperiencesRepository {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl OrganizationExperiencesRepository for MySqlOrganizationExperiencesRepository {
    type Tx = MySqlTx;

    async fn find(&self) -> Result<StoredOrganizationExperiences, AppError> {
        let row = sqlx::query("SELECT JSON_TYPE(settings) AS settings_kind, JSON_EXTRACT(settings, '$.experiences') AS experiences, updated_at FROM organization_settings WHERE id = 1")
            .fetch_optional(&self.pool).await?
            .ok_or_else(|| AppError::NotFound("the organization is not installed".to_owned()))?;
        decode(row)
    }

    async fn lock(&self, tx: &mut MySqlTx) -> Result<StoredOrganizationExperiences, AppError> {
        let row = sqlx::query("SELECT JSON_TYPE(settings) AS settings_kind, JSON_EXTRACT(settings, '$.experiences') AS experiences, updated_at FROM organization_settings WHERE id = 1 FOR UPDATE")
            .fetch_optional(&mut **tx).await?
            .ok_or_else(|| AppError::NotFound("the organization is not installed".to_owned()))?;
        decode(row)
    }

    async fn lock_shared(
        &self,
        tx: &mut MySqlTx,
    ) -> Result<StoredOrganizationExperiences, AppError> {
        let row = sqlx::query("SELECT JSON_TYPE(settings) AS settings_kind, JSON_EXTRACT(settings, '$.experiences') AS experiences, updated_at FROM organization_settings WHERE id = 1 FOR SHARE")
            .fetch_optional(&mut **tx).await?
            .ok_or_else(|| AppError::NotFound("the organization is not installed".to_owned()))?;
        decode(row)
    }

    async fn save(
        &self,
        tx: &mut MySqlTx,
        configuration: &OrganizationExperiences,
    ) -> Result<DateTime<Utc>, AppError> {
        sqlx::query("UPDATE organization_settings SET settings = JSON_SET(CASE WHEN settings IS NULL OR JSON_TYPE(settings) = 'NULL' THEN JSON_OBJECT() ELSE settings END, '$.experiences', CAST(? AS JSON)), updated_at = GREATEST(CURRENT_TIMESTAMP(6), updated_at + INTERVAL 1 MICROSECOND) WHERE id = 1")
            .bind(Json(configuration)).execute(&mut **tx).await?;
        let updated_at: NaiveDateTime =
            sqlx::query_scalar("SELECT updated_at FROM organization_settings WHERE id = 1")
                .fetch_one(&mut **tx)
                .await?;
        Ok(updated_at.and_utc())
    }
}

pub struct MySqlOrganizationExperiencesPorts {
    transactions: MySqlTransactionManager,
    repository: MySqlOrganizationExperiencesRepository,
    events: SharedEventPublisher,
    audit: SharedAuditLog,
    registry: Arc<RecordTypeRegistry>,
}

impl MySqlOrganizationExperiencesPorts {
    pub(crate) fn new(
        pool: MySqlPool,
        registry: Arc<RecordTypeRegistry>,
        events: SharedEventPublisher,
        audit: SharedAuditLog,
    ) -> Self {
        Self {
            transactions: MySqlTransactionManager::new(pool.clone()),
            repository: MySqlOrganizationExperiencesRepository { pool },
            events,
            audit,
            registry,
        }
    }
}

impl OrganizationExperiencesPorts for MySqlOrganizationExperiencesPorts {
    type Transactions = MySqlTransactionManager;
    type Repository = MySqlOrganizationExperiencesRepository;
    type Events = SharedEventPublisher;
    type Audit = SharedAuditLog;
    type Clock = SystemClock;

    fn transactions(&self) -> &Self::Transactions {
        &self.transactions
    }
    fn repository(&self) -> &Self::Repository {
        &self.repository
    }
    fn events(&self) -> &Self::Events {
        &self.events
    }
    fn audit(&self) -> &Self::Audit {
        &self.audit
    }
    fn clock(&self) -> &Self::Clock {
        &SystemClock
    }
    fn registry(&self) -> &RecordTypeRegistry {
        &self.registry
    }
}
