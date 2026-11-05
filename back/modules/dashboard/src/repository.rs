use std::collections::HashMap;

use async_trait::async_trait;
use sqlx::mysql::MySqlRow;
use sqlx::{MySqlPool, Row};
use uuid::Uuid;

use crate::engine::{
    DashboardRepository, SubjectDirectory, SubjectTeam, SubjectUser, WidgetRegistry,
};
use aphel_kernel::domain::{TeamId, UserId};
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::application::files::UserReference;
use aphel_sdk::platform::application::query::QueryPlan;
use aphel_sdk::platform::domain::access::AccessScope;
use aphel_sdk::platform::infrastructure::query::compiler::{Projection, build_query};
use aphel_sdk::platform::infrastructure::repositories::mapper::invalid_column;
use aphel_sdk::platform::records::constrain_query;

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

pub use aphel_sdk::dashboard::RecentEventExclusion;

impl MySqlDashboardRepository {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
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

pub use aphel_sdk::dashboard::WidgetPorts as MySqlDashboardPorts;

impl WidgetRegistry<MySqlDashboardPorts> {
    pub fn builtin() -> Self {
        aphel_sdk::platform::modules::app()
            .contributions::<dyn crate::engine::WidgetProvider<MySqlDashboardPorts>>()
            .into_iter()
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
