use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use sqlx::mysql::{MySqlArguments, MySqlRow};
use sqlx::{Execute, MySql, MySqlPool, QueryBuilder, Row};
use uuid::Uuid;

use crate::application::dashboard::{
    DashboardPorts, DashboardRepository, SubjectDirectory, SubjectTeam, SubjectUser, WidgetRegistry,
};
use crate::application::fields::RecordTypeRegistry;
use crate::application::files::UserReference;
use crate::application::query::QueryPlan;
use crate::domain::access::AccessScope;
use crate::domain::{TeamId, UserId};
use crate::errors::AppError;
use crate::infrastructure::query::MySqlRecordQueryPorts;
use crate::infrastructure::query::compiler::{Projection, build_query, push_where, quoted};
use crate::infrastructure::repositories::mapper::invalid_column;
use crate::records::constrain_query;

#[derive(Clone)]
pub struct MySqlDashboardRepository {
    pool: MySqlPool,
}

pub fn uuid_column(value: &str, column: &str) -> Result<Uuid, AppError> {
    Uuid::parse_str(value).map_err(|_| invalid_column(column, value))
}

pub fn user_column(
    row: &MySqlRow,
    id_column: &str,
    name_column: &str,
) -> Result<Option<UserReference>, AppError> {
    let id: Option<String> = row.try_get(id_column)?;
    let name: Option<String> = row.try_get(name_column)?;
    id.map(|id| {
        Ok(UserReference {
            id: UserId::new(id.clone()).map_err(|_| invalid_column(id_column, &id))?,
            display_name: name.unwrap_or_default(),
        })
    })
    .transpose()
}

pub fn link_column(
    row: &MySqlRow,
    type_column: &str,
    id_column: &str,
) -> Result<Option<(String, Uuid)>, AppError> {
    let record_type: Option<String> = row.try_get(type_column)?;
    let record_id: Option<String> = row.try_get(id_column)?;
    match (record_type, record_id) {
        (Some(record_type), Some(record_id)) => {
            Ok(Some((record_type, uuid_column(&record_id, id_column)?)))
        }
        _ => Ok(None),
    }
}

pub fn aggregate_query(
    scope: &AccessScope,
    plan: &QueryPlan,
    select: &str,
    suffix: &str,
) -> Result<(String, MySqlArguments), AppError> {
    let mut count = build_query(scope, plan, Projection::Count)?;
    let table = quoted(&plan.record_type.table);
    let prefix = format!("SELECT COUNT(*) FROM {table} r");
    let sql = count.sql().as_str().to_owned();
    let Some(conditions) = sql.strip_prefix(&prefix) else {
        return Err(AppError::Internal(
            "unexpected shape of the record count query".to_owned(),
        ));
    };
    let statement = format!("SELECT {select} FROM {table} r{conditions}{suffix}");
    let arguments = count
        .build()
        .take_arguments()
        .map_err(|error| AppError::Internal(error.to_string()))?
        .unwrap_or_default();
    Ok((statement, arguments))
}

pub fn push_ids(builder: &mut QueryBuilder<MySql>, ids: &[Uuid]) {
    builder.push(" IN (");
    let mut separated = builder.separated(", ");
    for id in ids {
        separated.push_bind(id.to_string());
    }
    builder.push(")");
}

pub fn in_order<T>(ids: &[Uuid], mut items: HashMap<Uuid, T>) -> Vec<T> {
    ids.iter().filter_map(|id| items.remove(id)).collect()
}

pub async fn record_titles(
    pool: &MySqlPool,
    registry: &RecordTypeRegistry,
    links: &[(String, Uuid)],
) -> Result<HashMap<(String, Uuid), String>, AppError> {
    let mut titles = HashMap::new();
    let mut record_types: Vec<&str> = links.iter().map(|(kind, _)| kind.as_str()).collect();
    record_types.sort_unstable();
    record_types.dedup();
    for record_type in record_types {
        let Ok(definition) = registry.require(record_type) else {
            continue;
        };
        let Some(column) = definition.title_column() else {
            continue;
        };
        let ids: Vec<Uuid> = links
            .iter()
            .filter(|(kind, _)| kind == record_type)
            .map(|(_, id)| *id)
            .collect();
        let mut builder = QueryBuilder::<MySql>::new(format!(
            "SELECT id, {} FROM {} WHERE id",
            quoted(column),
            quoted(&definition.table)
        ));
        push_ids(&mut builder, &ids);
        let rows: Vec<(String, Option<String>)> = builder.build_query_as().fetch_all(pool).await?;
        for (id, title) in rows {
            if let Some(title) = title {
                titles.insert((record_type.to_owned(), uuid_column(&id, "id")?), title);
            }
        }
    }
    Ok(titles)
}

pub trait RecentEventExclusion: Send + Sync {
    fn exclude(&self, builder: &mut QueryBuilder<MySql>);
}

pub fn push_readable(
    builder: &mut QueryBuilder<MySql>,
    scope: &AccessScope,
    type_column: &str,
    id_column: &str,
    plan: &QueryPlan,
) -> Result<(), AppError> {
    let definition = &plan.record_type;
    builder
        .push(format!(" OR ({type_column} = "))
        .push_bind(definition.record_type.as_str().to_owned())
        .push(format!(
            " AND {id_column} IN (SELECT r.id FROM {} r",
            quoted(&definition.table)
        ));
    push_where(builder, scope, plan)?;
    builder.push("))");
    Ok(())
}

impl MySqlDashboardRepository {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

pub async fn record_ids(
    pool: &MySqlPool,
    scope: &AccessScope,
    plan: &QueryPlan,
) -> Result<Vec<Uuid>, AppError> {
    let constrained = constrain_query(pool, scope, plan).await?;
    let plan = constrained.as_ref().unwrap_or(plan);
    let mut builder = build_query(scope, plan, Projection::Ids)?;
    let rows: Vec<String> = builder.build_query_scalar().fetch_all(pool).await?;
    rows.iter().map(|id| uuid_column(id, "id")).collect()
}

#[async_trait]
impl DashboardRepository for MySqlDashboardRepository {
    async fn currency(&self) -> Result<String, AppError> {
        let currency: Option<String> =
            sqlx::query_scalar("SELECT currency FROM organization_settings WHERE id = 1")
                .fetch_optional(&self.pool)
                .await?;
        Ok(currency.unwrap_or_else(|| "BRL".to_owned()))
    }

    async fn count(&self, scope: &AccessScope, plan: &QueryPlan) -> Result<u64, AppError> {
        let constrained = constrain_query(&self.pool, scope, plan).await?;
        let plan = constrained.as_ref().unwrap_or(plan);
        let mut builder = build_query(scope, plan, Projection::Count)?;
        let total: i64 = builder.build_query_scalar().fetch_one(&self.pool).await?;
        Ok(u64::try_from(total).unwrap_or_default())
    }

    async fn subject_directory(&self) -> Result<SubjectDirectory, AppError> {
        let users: Vec<(String, String)> = sqlx::query_as(
            "SELECT u.id, u.display_name FROM users u \
             WHERE u.status IN ('active', 'deactivated') \
             ORDER BY u.display_name, u.id",
        )
        .fetch_all(&self.pool)
        .await?;
        let teams: Vec<(String, String)> =
            sqlx::query_as("SELECT id, name FROM teams ORDER BY name, id")
                .fetch_all(&self.pool)
                .await?;
        let memberships: Vec<(String, String)> = sqlx::query_as(
            "SELECT tm.team_id, tm.user_id FROM team_memberships tm \
             JOIN teams t ON t.id = tm.team_id \
             JOIN users m ON m.id = tm.user_id \
             WHERE m.status IN ('active', 'deactivated')",
        )
        .fetch_all(&self.pool)
        .await?;
        let mut members: HashMap<String, Vec<UserId>> = HashMap::new();
        for (team_id, user_id) in memberships {
            let user = UserId::new(user_id.clone())
                .map_err(|_| invalid_column("team_memberships.user_id", &user_id))?;
            members.entry(team_id).or_default().push(user);
        }
        Ok(SubjectDirectory {
            users: users
                .into_iter()
                .map(|(id, display_name)| {
                    Ok(SubjectUser {
                        id: UserId::new(id.clone()).map_err(|_| invalid_column("users.id", &id))?,
                        display_name,
                    })
                })
                .collect::<Result<_, AppError>>()?,
            teams: teams
                .into_iter()
                .map(|(id, name)| {
                    Ok(SubjectTeam {
                        member_ids: members.remove(&id).unwrap_or_default(),
                        id: TeamId::new(id.clone()).map_err(|_| invalid_column("teams.id", &id))?,
                        name,
                    })
                })
                .collect::<Result<_, AppError>>()?,
        })
    }
}

pub struct MySqlDashboardPorts {
    query: MySqlRecordQueryPorts,
    repository: MySqlDashboardRepository,
}

impl MySqlDashboardPorts {
    pub fn new(pool: MySqlPool, registry: Arc<RecordTypeRegistry>) -> Self {
        Self {
            query: MySqlRecordQueryPorts::new(pool.clone(), registry),
            repository: MySqlDashboardRepository::new(pool),
        }
    }
}

impl DashboardPorts for MySqlDashboardPorts {
    type Query = MySqlRecordQueryPorts;
    type Dashboard = MySqlDashboardRepository;

    fn query(&self) -> &Self::Query {
        &self.query
    }

    fn dashboard(&self) -> &Self::Dashboard {
        &self.repository
    }
}

impl WidgetRegistry<MySqlDashboardPorts> {
    pub fn builtin() -> Self {
        crate::modules::app()
            .enabled_widgets()
            .cloned()
            .chain(crate::modules::runtime_extensions::widgets())
            .fold(Self::empty(), |registry, provider| {
                let key = provider.key().to_owned();
                registry
                    .clone()
                    .register(provider.clone())
                    .unwrap_or_else(|error| {
                        tracing::error!(widget = key, %error, "dashboard widget skipped");
                        registry
                    })
            })
    }
}
