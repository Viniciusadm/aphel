use std::sync::Arc;

use async_trait::async_trait;
use chrono_tz::Tz;
use sqlx::MySqlPool;
use uuid::Uuid;

use crate::application::fields::RecordTypeRegistry;
use crate::application::organization_admin::Listing;
use crate::application::query::{
    FilterSource, FilterSourceResolver, QueryPlan, RecordQueryExecutor, RecordQueryPorts,
};
use crate::domain::access::AccessScope;
use crate::errors::AppError;
use crate::infrastructure::auth::SystemClock;
use crate::infrastructure::fields::MySqlFieldDefinitionRepository;
use crate::infrastructure::query::aggregate::MySqlAggregateExecutor;
use crate::infrastructure::query::compiler::{Projection, build_query};
use crate::records::constrain_query;

#[derive(Clone)]
pub struct MySqlRecordQueryExecutor {
    pool: MySqlPool,
}

impl MySqlRecordQueryExecutor {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl RecordQueryExecutor for MySqlRecordQueryExecutor {
    async fn timezone(&self) -> Result<Tz, AppError> {
        let timezone: Option<String> =
            sqlx::query_scalar("SELECT timezone FROM organization_settings WHERE id = 1")
                .fetch_optional(&self.pool)
                .await?;
        Ok(timezone
            .and_then(|timezone| timezone.parse::<Tz>().ok())
            .unwrap_or(chrono_tz::UTC))
    }

    async fn execute(
        &self,
        scope: &AccessScope,
        plan: &QueryPlan,
    ) -> Result<Listing<Uuid>, AppError> {
        let constrained = constrain_query(&self.pool, scope, plan).await?;
        let plan = constrained.as_ref().unwrap_or(plan);
        let mut count = build_query(scope, plan, Projection::Count)?;
        let total: i64 = count.build_query_scalar().fetch_one(&self.pool).await?;
        if total == 0 {
            return Ok(Listing {
                items: Vec::new(),
                total: 0,
            });
        }
        let mut ids = build_query(scope, plan, Projection::Ids)?;
        let rows: Vec<String> = ids.build_query_scalar().fetch_all(&self.pool).await?;
        Ok(Listing {
            items: rows
                .into_iter()
                .map(|id| {
                    Uuid::parse_str(&id).map_err(|_| {
                        AppError::Persistence(format!("record id {id:?} is not a UUID"))
                    })
                })
                .collect::<Result<_, _>>()?,
            total: u64::try_from(total).unwrap_or_default(),
        })
    }

    async fn filter_source(&self, filter_id: Uuid) -> Result<Option<FilterSource>, AppError> {
        let Some(resolver) = crate::modules::app()
            .services()
            .get::<dyn FilterSourceResolver>()
        else {
            return Ok(None);
        };
        resolver.source(&self.pool, filter_id).await
    }
}

pub struct MySqlRecordQueryPorts {
    registry: Arc<RecordTypeRegistry>,
    definitions: MySqlFieldDefinitionRepository,
    executor: MySqlRecordQueryExecutor,
    aggregates: MySqlAggregateExecutor,
}

impl MySqlRecordQueryPorts {
    pub fn new(pool: MySqlPool, registry: Arc<RecordTypeRegistry>) -> Self {
        Self {
            registry,
            definitions: MySqlFieldDefinitionRepository::new(pool.clone()),
            executor: MySqlRecordQueryExecutor::new(pool.clone()),
            aggregates: MySqlAggregateExecutor::new(pool),
        }
    }

    pub(crate) fn aggregate_executor(&self) -> &MySqlAggregateExecutor {
        &self.aggregates
    }
}

impl RecordQueryPorts for MySqlRecordQueryPorts {
    type Definitions = MySqlFieldDefinitionRepository;
    type Executor = MySqlRecordQueryExecutor;
    type Clock = SystemClock;

    fn registry(&self) -> &RecordTypeRegistry {
        &self.registry
    }

    fn definitions(&self) -> &Self::Definitions {
        &self.definitions
    }

    fn executor(&self) -> &Self::Executor {
        &self.executor
    }

    fn clock(&self) -> &Self::Clock {
        &SystemClock
    }
}
