use std::sync::Arc;

use async_trait::async_trait;
use chrono::NaiveDateTime;
use serde_json::{Map, Value};
use sqlx::mysql::MySqlRow;
use sqlx::types::Json;
use sqlx::{MySqlPool, QueryBuilder, Row};
use uuid::Uuid;

use crate::application::{SavedView, SavedViewRepository, ViewPorts, ViewType, ViewVisibility};
use aphel_kernel::domain::{TeamId, UserId};
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::application::fields::RecordTypeRegistry;
use aphel_sdk::platform::application::query::{Filter, SortSpec};
use aphel_sdk::platform::application::shared::{AuditLog, EventPublisher, RecordType};
use aphel_sdk::platform::database::transaction::MySqlTx;
use aphel_sdk::platform::domain::access::AccessScope;
use aphel_sdk::platform::infrastructure::auth::SystemClock;
use aphel_sdk::platform::infrastructure::fields::MySqlFieldDefinitionRepository;
use aphel_sdk::platform::infrastructure::fields::mysql_definitions::uuid_column;
use aphel_sdk::platform::infrastructure::repositories::mapper;
use aphel_sdk::platform::infrastructure::transactions::MySqlTransactionManager;

const VIEW_COLUMNS: &str = "id, record_type, name, owner_id, visibility, team_id, view_type, filter, sort, \
     columns, settings, is_default, position, created_at, updated_at";

#[derive(Clone)]
pub struct MySqlSavedViewRepository {
    pool: MySqlPool,
}

impl MySqlSavedViewRepository {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

fn json_column<T: serde::de::DeserializeOwned>(
    row: &MySqlRow,
    column: &str,
) -> Result<T, AppError> {
    let Json(value): Json<Value> = row.try_get(column)?;
    serde_json::from_value(value.clone())
        .map_err(|_| mapper::invalid_column(column, &value.to_string()))
}

fn view_from_row(row: &MySqlRow) -> Result<SavedView, AppError> {
    let record_type: String = row.try_get("record_type")?;
    let visibility: String = row.try_get("visibility")?;
    let view_type: String = row.try_get("view_type")?;
    let filter: Option<Json<Value>> = row.try_get("filter")?;
    let created_at: NaiveDateTime = row.try_get("created_at")?;
    let updated_at: NaiveDateTime = row.try_get("updated_at")?;
    Ok(SavedView {
        id: uuid_column(row, "id")?,
        record_type: RecordType::parse(record_type.clone())
            .map_err(|_| mapper::invalid_column("record_type", &record_type))?,
        name: row.try_get("name")?,
        owner_id: row
            .try_get::<Option<String>, _>("owner_id")?
            .map(|id| mapper::typed_id(id, UserId::new))
            .transpose()?,
        visibility: ViewVisibility::parse(&visibility)
            .ok_or_else(|| mapper::invalid_column("visibility", &visibility))?,
        team_id: row
            .try_get::<Option<String>, _>("team_id")?
            .map(|id| mapper::typed_id(id, TeamId::new))
            .transpose()?,
        view_type: ViewType::parse(&view_type)
            .ok_or_else(|| mapper::invalid_column("view_type", &view_type))?,
        filter: filter
            .map(|Json(value)| {
                serde_json::from_value::<Filter>(value.clone())
                    .map_err(|_| mapper::invalid_column("filter", &value.to_string()))
            })
            .transpose()?,
        sort: json_column::<Vec<SortSpec>>(row, "sort")?,
        columns: json_column::<Vec<String>>(row, "columns")?,
        settings: json_column::<Map<String, Value>>(row, "settings")?,
        is_default: row.try_get("is_default")?,
        position: row.try_get("position")?,
        created_at: mapper::utc(created_at),
        updated_at: mapper::utc(updated_at),
    })
}

fn to_json<T: serde::Serialize>(value: &T) -> Result<Json<Value>, AppError> {
    serde_json::to_value(value)
        .map(Json)
        .map_err(|error| AppError::Internal(format!("cannot serialize view: {error}")))
}

#[async_trait]
impl SavedViewRepository for MySqlSavedViewRepository {
    type Tx = MySqlTx;

    async fn list_visible(
        &self,
        scope: &AccessScope,
        record_type: &RecordType,
    ) -> Result<Vec<SavedView>, AppError> {
        let mut builder =
            QueryBuilder::new(format!("SELECT {VIEW_COLUMNS} FROM saved_views WHERE "));
        builder
            .push("record_type = ")
            .push_bind(record_type.as_str().to_owned())
            .push(" AND (visibility = 'organization' OR owner_id = ")
            .push_bind(scope.user_id().as_str().to_owned());
        if !scope.team_ids().is_empty() {
            builder.push(" OR (visibility = 'team' AND team_id IN (");
            let mut teams = builder.separated(", ");
            for team in scope.team_ids() {
                teams.push_bind(team.as_str().to_owned());
            }
            builder.push("))");
        }
        builder.push(") ORDER BY position, name, id");
        let rows = builder.build().fetch_all(&self.pool).await?;
        rows.iter().map(view_from_row).collect()
    }

    async fn find(&self, view_id: Uuid) -> Result<Option<SavedView>, AppError> {
        let row = sqlx::query(sqlx::AssertSqlSafe(format!(
            "SELECT {VIEW_COLUMNS} FROM saved_views WHERE id = ?"
        )))
        .bind(view_id.to_string())
        .fetch_optional(&self.pool)
        .await?;
        row.as_ref().map(view_from_row).transpose()
    }

    async fn next_position(&self, record_type: &RecordType) -> Result<i32, AppError> {
        let position: Option<i32> =
            sqlx::query_scalar("SELECT MAX(position) FROM saved_views WHERE record_type = ?")
                .bind(record_type.as_str())
                .fetch_one(&self.pool)
                .await?;
        Ok(position.map_or(0, |position| position + 1))
    }

    async fn team_exists(&self, team_id: &TeamId) -> Result<bool, AppError> {
        Ok(
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM teams WHERE id = ?)")
                .bind(team_id.as_str())
                .fetch_one(&self.pool)
                .await?,
        )
    }

    async fn user_default(
        &self,
        scope: &AccessScope,
        record_type: &RecordType,
    ) -> Result<Option<Uuid>, AppError> {
        let id: Option<String> = sqlx::query_scalar(
            "SELECT view_id FROM saved_view_user_defaults \
             WHERE user_id = ? AND record_type = ?",
        )
        .bind(scope.user_id().as_str())
        .bind(record_type.as_str())
        .fetch_optional(&self.pool)
        .await?;
        Ok(id.and_then(|id| Uuid::parse_str(&id).ok()))
    }

    async fn insert(&self, tx: &mut MySqlTx, view: &SavedView) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO saved_views (id, record_type, name, owner_id, visibility, team_id, \
             view_type, filter, sort, columns, settings, is_default, position, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(view.id.to_string())
        .bind(view.record_type.as_str())
        .bind(&view.name)
        .bind(view.owner_id.as_ref().map(UserId::as_str))
        .bind(view.visibility.as_str())
        .bind(view.team_id.as_ref().map(TeamId::as_str))
        .bind(view.view_type.as_str())
        .bind(view.filter.as_ref().map(to_json).transpose()?)
        .bind(to_json(&view.sort)?)
        .bind(to_json(&view.columns)?)
        .bind(to_json(&view.settings)?)
        .bind(view.is_default)
        .bind(view.position)
        .bind(mapper::naive(view.created_at))
        .bind(mapper::naive(view.updated_at))
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    async fn update(&self, tx: &mut MySqlTx, view: &SavedView) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE saved_views SET name = ?, visibility = ?, team_id = ?, view_type = ?, filter = ?, \
             sort = ?, columns = ?, settings = ?, is_default = ?, updated_at = ? \
             WHERE id = ?",
        )
        .bind(&view.name)
        .bind(view.visibility.as_str())
        .bind(view.team_id.as_ref().map(TeamId::as_str))
        .bind(view.view_type.as_str())
        .bind(view.filter.as_ref().map(to_json).transpose()?)
        .bind(to_json(&view.sort)?)
        .bind(to_json(&view.columns)?)
        .bind(to_json(&view.settings)?)
        .bind(view.is_default)
        .bind(mapper::naive(view.updated_at))
        .bind(view.id.to_string())
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    async fn delete(&self, tx: &mut MySqlTx, view_id: Uuid) -> Result<(), AppError> {
        sqlx::query("DELETE FROM saved_views WHERE id = ?")
            .bind(view_id.to_string())
            .execute(&mut **tx)
            .await?;
        aphel_sdk::reference_cleanup::removed(tx, "saved_views.view", view_id).await?;
        Ok(())
    }

    async fn clear_organization_default(
        &self,
        tx: &mut MySqlTx,
        record_type: &RecordType,
        except: Uuid,
    ) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE saved_views SET is_default = FALSE \
             WHERE record_type = ? AND id <> ? AND is_default",
        )
        .bind(record_type.as_str())
        .bind(except.to_string())
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    async fn set_user_default(
        &self,
        scope: &AccessScope,
        record_type: &RecordType,
        view_id: Option<Uuid>,
    ) -> Result<(), AppError> {
        match view_id {
            Some(view_id) => {
                sqlx::query(
                    "INSERT INTO saved_view_user_defaults (user_id, record_type, view_id) \
                     VALUES (?, ?, ?) AS new ON DUPLICATE KEY UPDATE view_id = new.view_id",
                )
                .bind(scope.user_id().as_str())
                .bind(record_type.as_str())
                .bind(view_id.to_string())
                .execute(&self.pool)
                .await?;
            }
            None => {
                sqlx::query(
                    "DELETE FROM saved_view_user_defaults \
                     WHERE user_id = ? AND record_type = ?",
                )
                .bind(scope.user_id().as_str())
                .bind(record_type.as_str())
                .execute(&self.pool)
                .await?;
            }
        }
        Ok(())
    }
}

pub struct MySqlViewPorts {
    registry: Arc<RecordTypeRegistry>,
    transactions: MySqlTransactionManager,
    views: MySqlSavedViewRepository,
    definitions: MySqlFieldDefinitionRepository,
    events: Arc<dyn EventPublisher<MySqlTx>>,
    audit: Arc<dyn AuditLog<MySqlTx>>,
}

impl MySqlViewPorts {
    pub fn new(
        pool: MySqlPool,
        registry: Arc<RecordTypeRegistry>,
        events: Arc<dyn EventPublisher<MySqlTx>>,
        audit: Arc<dyn AuditLog<MySqlTx>>,
    ) -> Self {
        Self {
            registry,
            transactions: MySqlTransactionManager::new(pool.clone()),
            views: MySqlSavedViewRepository::new(pool.clone()),
            definitions: MySqlFieldDefinitionRepository::new(pool),
            events,
            audit,
        }
    }
}

impl ViewPorts for MySqlViewPorts {
    type Transactions = MySqlTransactionManager;
    type Views = MySqlSavedViewRepository;
    type Definitions = MySqlFieldDefinitionRepository;
    type Events = Arc<dyn EventPublisher<MySqlTx>>;
    type Audit = Arc<dyn AuditLog<MySqlTx>>;
    type Clock = SystemClock;

    fn registry(&self) -> &RecordTypeRegistry {
        &self.registry
    }

    fn transactions(&self) -> &Self::Transactions {
        &self.transactions
    }

    fn views(&self) -> &Self::Views {
        &self.views
    }

    fn definitions(&self) -> &Self::Definitions {
        &self.definitions
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
}
