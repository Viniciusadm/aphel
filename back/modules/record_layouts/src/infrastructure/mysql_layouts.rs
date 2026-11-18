use std::sync::Arc;

use async_trait::async_trait;
use chrono::NaiveDateTime;
use serde_json::Value;
use sqlx::mysql::MySqlRow;
use sqlx::types::Json;
use sqlx::{AssertSqlSafe, MySqlPool, QueryBuilder, Row};

use crate::application::{
    FormLayout, FormLayoutRepository, LayoutContext, LayoutPorts, RulesSource, StoredFormLayout,
};
use aphel_kernel::domain::UserId;
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::application::fields::RecordTypeRegistry;
use aphel_sdk::platform::application::shared::{AuditLog, EventPublisher, RecordType};
use aphel_sdk::platform::database::transaction::Tx;
use aphel_sdk::platform::infrastructure::auth::SystemClock;
use aphel_sdk::platform::infrastructure::fields::MySqlFieldDefinitionRepository;
use aphel_sdk::platform::infrastructure::fields::mysql_definitions::uuid_column;
use aphel_sdk::platform::infrastructure::repositories::mapper;
use aphel_sdk::platform::infrastructure::transactions::MySqlTransactionManager;

const LAYOUT_COLUMNS: &str = "id, record_type, context, layout, updated_by, created_at, updated_at";

#[derive(Clone)]
pub struct MySqlFormLayoutRepository {
    pool: MySqlPool,
}

impl MySqlFormLayoutRepository {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

fn parse_layout(value: Value) -> Result<FormLayout, AppError> {
    serde_json::from_value::<FormLayout>(value.clone())
        .map_err(|_| mapper::invalid_column("layout", &value.to_string()))
}

fn layout_from_row(row: &MySqlRow) -> Result<StoredFormLayout, AppError> {
    let record_type: String = row.try_get("record_type")?;
    let context: String = row.try_get("context")?;
    let Json(layout): Json<Value> = row.try_get("layout")?;
    let created_at: NaiveDateTime = row.try_get("created_at")?;
    let updated_at: NaiveDateTime = row.try_get("updated_at")?;
    Ok(StoredFormLayout {
        id: uuid_column(row, "id")?,
        record_type: RecordType::parse(record_type.clone())
            .map_err(|_| mapper::invalid_column("record_type", &record_type))?,
        context: LayoutContext::parse(&context)
            .ok_or_else(|| mapper::invalid_column("context", &context))?,
        layout: parse_layout(layout)?,
        updated_by: row
            .try_get::<Option<String>, _>("updated_by")?
            .map(|id| mapper::typed_id(id, UserId::new))
            .transpose()?,
        created_at: mapper::utc(created_at),
        updated_at: mapper::utc(updated_at),
    })
}

#[async_trait]
impl FormLayoutRepository for MySqlFormLayoutRepository {
    type Tx = Tx;

    async fn find(
        &self,
        record_type: &RecordType,
        context: LayoutContext,
    ) -> Result<Option<StoredFormLayout>, AppError> {
        sqlx::query(AssertSqlSafe(format!(
            "SELECT {LAYOUT_COLUMNS} FROM form_layouts \
             WHERE record_type = ? AND context = ?"
        )))
        .bind(record_type.as_str())
        .bind(context.as_str())
        .fetch_optional(&self.pool)
        .await?
        .as_ref()
        .map(layout_from_row)
        .transpose()
    }

    async fn rules_source(
        &self,
        record_type: &RecordType,
        contexts: &[LayoutContext],
    ) -> Result<RulesSource, AppError> {
        if contexts.is_empty() {
            return Ok(RulesSource::default());
        }
        let mut builder = QueryBuilder::new(
            "SELECT o.timezone AS timezone, l.layout AS layout FROM organization_settings o \
             LEFT JOIN form_layouts l ON l.record_type = ",
        );
        builder.push_bind(record_type.as_str().to_owned());
        builder.push(" AND l.context IN (");
        let mut separated = builder.separated(", ");
        for context in contexts {
            separated.push_bind(context.as_str());
        }
        builder.push(") WHERE o.id = 1");
        builder.push(" ORDER BY FIELD(l.context");
        for context in contexts {
            builder.push(", ").push_bind(context.as_str());
        }
        builder.push(") LIMIT 1");
        let Some(row) = builder.build().fetch_optional(&self.pool).await? else {
            return Ok(RulesSource::default());
        };
        let layout: Option<Json<Value>> = row.try_get("layout")?;
        Ok(RulesSource {
            layout: layout.map(|Json(value)| parse_layout(value)).transpose()?,
            timezone: row.try_get("timezone")?,
        })
    }

    async fn upsert(&self, tx: &mut Tx, layout: &StoredFormLayout) -> Result<(), AppError> {
        let document = serde_json::to_value(&layout.layout)
            .map_err(|error| AppError::Internal(format!("cannot serialize layout: {error}")))?;
        sqlx::query(
            "INSERT INTO form_layouts (id, record_type, context, layout, updated_by, \
             created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?) AS new \
             ON DUPLICATE KEY UPDATE layout = new.layout, updated_by = new.updated_by, \
             updated_at = new.updated_at",
        )
        .bind(layout.id.to_string())
        .bind(layout.record_type.as_str())
        .bind(layout.context.as_str())
        .bind(Json(document))
        .bind(layout.updated_by.as_ref().map(UserId::as_str))
        .bind(mapper::naive(layout.created_at))
        .bind(mapper::naive(layout.updated_at))
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    async fn delete(
        &self,
        tx: &mut Tx,
        record_type: &RecordType,
        context: LayoutContext,
    ) -> Result<(), AppError> {
        sqlx::query("DELETE FROM form_layouts WHERE record_type = ? AND context = ?")
            .bind(record_type.as_str())
            .bind(context.as_str())
            .execute(&mut **tx)
            .await?;
        Ok(())
    }
}

pub struct MySqlLayoutPorts {
    registry: Arc<RecordTypeRegistry>,
    transactions: MySqlTransactionManager,
    layouts: MySqlFormLayoutRepository,
    definitions: MySqlFieldDefinitionRepository,
    events: Arc<dyn EventPublisher<Tx>>,
    audit: Arc<dyn AuditLog<Tx>>,
}

impl MySqlLayoutPorts {
    pub fn new(
        pool: MySqlPool,
        registry: Arc<RecordTypeRegistry>,
        events: Arc<dyn EventPublisher<Tx>>,
        audit: Arc<dyn AuditLog<Tx>>,
    ) -> Self {
        Self {
            registry,
            transactions: MySqlTransactionManager::new(pool.clone()),
            layouts: MySqlFormLayoutRepository::new(pool.clone()),
            definitions: MySqlFieldDefinitionRepository::new(pool),
            events,
            audit,
        }
    }
}

impl LayoutPorts for MySqlLayoutPorts {
    type Transactions = MySqlTransactionManager;
    type Layouts = MySqlFormLayoutRepository;
    type Definitions = MySqlFieldDefinitionRepository;
    type Events = Arc<dyn EventPublisher<Tx>>;
    type Audit = Arc<dyn AuditLog<Tx>>;
    type Clock = SystemClock;

    fn registry(&self) -> &RecordTypeRegistry {
        &self.registry
    }

    fn transactions(&self) -> &Self::Transactions {
        &self.transactions
    }

    fn layouts(&self) -> &Self::Layouts {
        &self.layouts
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
