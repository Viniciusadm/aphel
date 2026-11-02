use crate::modules::points::DigestSource;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use sqlx::{AssertSqlSafe, MySqlPool, Row};

use crate::application::access::{AccessRepository, build_access_scope};
use crate::application::fields::{RecordTypeDefinition, RecordTypeRegistry};
use crate::application::files::RecordAccessChecker;
use crate::application::notifications::{
    DueTask, NotificationDirectory, Recipient, RecordFollowerSource, RecordSummary,
};
use crate::application::shared::RecordRef;
use crate::domain::UserId;
use crate::domain::users::UserStatus;
use crate::errors::AppError;
use crate::infrastructure::records::MySqlRecordAccessChecker;
use crate::infrastructure::repositories::mapper;
use crate::infrastructure::repositories::mysql_access::MySqlAccessRepository;

pub(crate) const MAX_DUE_TASKS_PER_RUN: u32 = 5000;

#[derive(Clone)]
pub struct MySqlNotificationDirectory {
    pool: MySqlPool,
    access: MySqlAccessRepository,
    registry: Arc<RecordTypeRegistry>,
}

impl MySqlNotificationDirectory {
    pub(crate) fn new(pool: MySqlPool, registry: Arc<RecordTypeRegistry>) -> Self {
        Self {
            access: MySqlAccessRepository::new(pool.clone()),
            registry,
            pool,
        }
    }

    async fn current_registry(&self) -> Arc<RecordTypeRegistry> {
        match crate::state::runtime_state() {
            Ok(state) => state
                .registry()
                .await
                .unwrap_or_else(|_| self.registry.clone()),
            Err(_) => self.registry.clone(),
        }
    }
}

fn summary_query(definition: &RecordTypeDefinition) -> Option<String> {
    if !definition.capabilities.notifies {
        return None;
    }
    let title = definition.title_column()?;
    let owner = definition
        .owner_column
        .as_deref()
        .map_or_else(|| "NULL".to_owned(), str::to_owned);
    let deleted = definition
        .deleted_column
        .as_deref()
        .map(|column| format!(" AND {column} IS NULL"))
        .unwrap_or_default();
    Some(format!(
        "SELECT {title} AS title, {owner} AS owner_id FROM {} WHERE id = ?{deleted}",
        definition.table
    ))
}

#[async_trait]
impl NotificationDirectory for MySqlNotificationDirectory {
    async fn recipient(&self, user_id: &UserId) -> Result<Option<Recipient>, AppError> {
        let row = sqlx::query(
            "SELECT u.id, u.display_name, u.email, COALESCE(u.locale, o.locale) AS locale FROM users u \
             LEFT JOIN organization_settings o ON o.id = 1 \
             WHERE u.id = ? AND u.status = ?",
        )
        .bind(user_id.as_str())
        .bind(UserStatus::Active.as_str())
        .fetch_optional(&self.pool)
        .await?;
        row.map(|row| {
            Ok(Recipient {
                user_id: mapper::typed_id(row.try_get("id")?, UserId::new)?,
                display_name: row.try_get("display_name")?,
                email: row.try_get("email")?,
                locale: row.try_get("locale")?,
            })
        })
        .transpose()
    }

    async fn record_summary(&self, record: &RecordRef) -> Result<Option<RecordSummary>, AppError> {
        let registry = self.current_registry().await;
        let Some(statement) = registry.get(&record.record_type).and_then(summary_query) else {
            return Ok(None);
        };
        let row = sqlx::query(AssertSqlSafe(statement))
            .bind(record.record_id.to_string())
            .fetch_optional(&self.pool)
            .await?;
        row.map(|row| {
            let title: Option<String> = row.try_get("title")?;
            let owner_id: Option<String> = row.try_get("owner_id")?;
            Ok(RecordSummary {
                title: title.unwrap_or_default(),
                owner_id: owner_id
                    .map(|id| mapper::typed_id(id, UserId::new))
                    .transpose()?,
            })
        })
        .transpose()
    }

    async fn can_read(&self, user_id: &UserId, record: &RecordRef) -> Result<bool, AppError> {
        let Some(access) = self.access.load_access(user_id).await? else {
            return Ok(false);
        };
        let scope = build_access_scope(user_id.clone(), access);
        let records =
            MySqlRecordAccessChecker::new(self.pool.clone(), self.current_registry().await);
        match records.can_read(&scope, record).await {
            Ok(readable) => Ok(readable),
            Err(AppError::Coded { .. }) => Ok(false),
            Err(error) => Err(error),
        }
    }

    async fn followers(&self, record: &RecordRef) -> Result<Vec<UserId>, AppError> {
        match crate::modules::app()
            .services()
            .get::<dyn RecordFollowerSource>()
        {
            Some(source) => source.followers(&self.pool, record).await,
            None => Ok(Vec::new()),
        }
    }

    async fn user_timezone(&self, user_id: &UserId) -> Result<Tz, AppError> {
        let stored: Option<Option<String>> =
            sqlx::query_scalar("SELECT timezone FROM users WHERE id = ?")
                .bind(user_id.as_str())
                .fetch_optional(&self.pool)
                .await?;
        match stored.flatten().and_then(|name| name.parse::<Tz>().ok()) {
            Some(timezone) => Ok(timezone),
            None => self.organization_timezone().await,
        }
    }

    async fn api_token_owner(&self, token_id: &str) -> Result<Option<UserId>, AppError> {
        let owner: Option<String> =
            sqlx::query_scalar("SELECT created_by FROM api_tokens WHERE id = ?")
                .bind(token_id)
                .fetch_optional(&self.pool)
                .await?;
        Ok(owner.and_then(|owner| UserId::new(owner).ok()))
    }

    async fn organization_timezone(&self) -> Result<Tz, AppError> {
        let timezone: Option<String> =
            sqlx::query_scalar("SELECT timezone FROM organization_settings WHERE id = 1")
                .fetch_optional(&self.pool)
                .await?;
        Ok(timezone
            .and_then(|timezone| timezone.parse::<Tz>().ok())
            .unwrap_or(chrono_tz::UTC))
    }

    async fn open_tasks_due(
        &self,
        owner_id: Option<&UserId>,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<Vec<DueTask>, AppError> {
        let mut tasks = Vec::new();
        for source in crate::modules::app().contributions::<dyn DigestSource>() {
            tasks.extend(
                source
                    .open_tasks_due(
                        &self.pool,
                        owner_id,
                        from,
                        to,
                        u64::from(MAX_DUE_TASKS_PER_RUN),
                    )
                    .await?,
            );
        }
        tasks.sort_by_key(|task| (task.due_at, task.id));
        tasks.truncate(MAX_DUE_TASKS_PER_RUN as usize);
        Ok(tasks)
    }
}
