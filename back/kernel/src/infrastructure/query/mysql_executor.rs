use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::{Arc, LazyLock, Mutex};
use std::time::{Duration, Instant};

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

impl MySqlRecordQueryExecutor {
    async fn count(&self, scope: &AccessScope, plan: &QueryPlan) -> Result<u64, AppError> {
        let mut count = build_query(scope, plan, Projection::Count)?;
        let key = count_key(count.sql().as_str(), scope, plan);
        if let Some(total) = cached_count(key) {
            return Ok(total);
        }
        let total: i64 = count.build_query_scalar().fetch_one(&self.pool).await?;
        let total = u64::try_from(total).unwrap_or_default();
        store_count(key, total);
        Ok(total)
    }
}

const COUNT_CACHE_TTL: Duration = Duration::from_secs(3);
const COUNT_CACHE_CAPACITY: usize = 512;

struct CachedCount {
    total: u64,
    stored_at: Instant,
}

static COUNTS: LazyLock<Mutex<HashMap<u128, CachedCount>>> = LazyLock::new(Mutex::default);

fn count_key(sql: &str, scope: &AccessScope, plan: &QueryPlan) -> u128 {
    let mut first = DefaultHasher::new();
    let mut second = DefaultHasher::new();
    second.write_u8(1);
    for hasher in [&mut first, &mut second] {
        sql.hash(hasher);
        format!("{scope:?}").hash(hasher);
        format!(
            "{:?}{:?}{:?}{:?}{:?}{:?}",
            plan.record_type, plan.visibility, plan.filter, plan.search, plan.sort, plan.archived
        )
        .hash(hasher);
    }
    (u128::from(first.finish()) << 64) | u128::from(second.finish())
}

fn cached_count(key: u128) -> Option<u64> {
    let counts = COUNTS.lock().ok()?;
    let cached = counts.get(&key)?;
    (cached.stored_at.elapsed() < COUNT_CACHE_TTL).then_some(cached.total)
}

fn store_count(key: u128, total: u64) {
    let Ok(mut counts) = COUNTS.lock() else {
        return;
    };
    if counts.len() >= COUNT_CACHE_CAPACITY {
        counts.retain(|_, cached| cached.stored_at.elapsed() < COUNT_CACHE_TTL);
        if counts.len() >= COUNT_CACHE_CAPACITY {
            counts.clear();
        }
    }
    counts.insert(
        key,
        CachedCount {
            total,
            stored_at: Instant::now(),
        },
    );
}

pub fn invalidate_counts() {
    if let Ok(mut counts) = COUNTS.lock() {
        counts.clear();
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
        let mut ids = build_query(scope, plan, Projection::Ids)?;
        let rows: Vec<String> = ids.build_query_scalar().fetch_all(&self.pool).await?;
        let fetched = u64::try_from(rows.len()).unwrap_or(u64::MAX);
        let total = if plan.window.offset == 0 && fetched < plan.window.limit {
            fetched
        } else if rows.is_empty() && plan.window.offset > 0 {
            self.count(scope, plan).await?
        } else {
            self.count(scope, plan)
                .await?
                .max(plan.window.offset + fetched)
        };
        Ok(Listing {
            items: rows
                .into_iter()
                .map(|id| {
                    Uuid::parse_str(&id).map_err(|_| {
                        AppError::Persistence(format!("record id {id:?} is not a UUID"))
                    })
                })
                .collect::<Result<_, _>>()?,
            total,
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
