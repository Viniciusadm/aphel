use chrono::{NaiveDate, NaiveDateTime};
use rust_decimal::Decimal;
use sqlx::mysql::MySqlRow;
use sqlx::{MySql, MySqlPool, QueryBuilder, Row};
use uuid::Uuid;

use crate::domain::{
    Goal, GoalCreditMode, GoalListFilter, GoalMetric, GoalOwner, GoalOwnerOption, GoalPeriod,
    GoalVisibility,
};
use aphel_kernel::Ctx;
use aphel_kernel::application::organization_admin::Listing;
use aphel_kernel::database::transaction::MySqlTx;
use aphel_kernel::domain::{TeamId, UserId};
use aphel_kernel::errors::AppError;
use aphel_kernel::infrastructure::repositories::mapper;

const GOAL_SELECT: &str = "SELECT g.id, g.owner_type, g.user_id, g.team_id, g.metric, g.period, \
     g.starts_on, g.ends_on, g.target, g.credit_mode, g.created_by, g.created_at, g.updated_at, \
     COALESCE(u.display_name, t.name) AS owner_name \
     FROM goals g \
     LEFT JOIN users u ON u.id = g.user_id \
     LEFT JOIN teams t ON t.id = g.team_id \
     WHERE 1 = 1";

const LIVE_OWNER: &str = " AND NOT (g.owner_type = 'user' AND g.user_id IS NULL) \
     AND NOT (g.owner_type = 'team' AND g.team_id IS NULL)";

pub(crate) struct OrphanGoal {
    pub(crate) id: Uuid,
    pub(crate) owner_type: String,
    pub(crate) metric: String,
    pub(crate) period: String,
    pub(crate) starts_on: NaiveDate,
    pub(crate) target: Decimal,
}

fn push_list<T: ToString>(builder: &mut QueryBuilder<MySql>, values: &[T]) {
    builder.push(" IN (");
    let mut separated = builder.separated(", ");
    for value in values {
        separated.push_bind(value.to_string());
    }
    builder.push(")");
}

fn push_filter(builder: &mut QueryBuilder<MySql>, filter: &GoalListFilter) {
    builder.push(LIVE_OWNER);
    if let GoalVisibility::Limited { user_ids, team_ids } = &filter.visibility {
        builder.push(" AND (g.owner_type = 'organization'");
        if !user_ids.is_empty() {
            builder.push(" OR g.user_id");
            push_list(builder, user_ids);
        }
        if !team_ids.is_empty() {
            builder.push(" OR g.team_id");
            push_list(builder, team_ids);
        }
        builder.push(")");
    }
    if let Some(owner_type) = filter.owner_type {
        builder
            .push(" AND g.owner_type = ")
            .push_bind(owner_type.to_owned());
    }
    if let Some(user_id) = &filter.user_id {
        builder
            .push(" AND g.user_id = ")
            .push_bind(user_id.as_str().to_owned());
    }
    if let Some(team_id) = &filter.team_id {
        builder
            .push(" AND g.team_id = ")
            .push_bind(team_id.as_str().to_owned());
    }
    if let Some(metric) = filter.metric {
        builder
            .push(" AND g.metric = ")
            .push_bind(metric.as_str().to_owned());
    }
    if let Some(period) = filter.period {
        builder
            .push(" AND g.period = ")
            .push_bind(period.as_str().to_owned());
    }
    if let Some(day) = filter.active_on {
        builder
            .push(" AND g.starts_on <= ")
            .push_bind(day)
            .push(" AND g.ends_on >= ")
            .push_bind(day);
    }
}

fn goal_from_row(row: &MySqlRow) -> Result<Goal, AppError> {
    let id: String = row.try_get("id")?;
    let owner_type: String = row.try_get("owner_type")?;
    let user_id: Option<String> = row.try_get("user_id")?;
    let team_id: Option<String> = row.try_get("team_id")?;
    let owner = match (owner_type.as_str(), user_id, team_id) {
        ("organization", _, _) => GoalOwner::Organization,
        ("user", Some(user_id), _) => GoalOwner::User(mapper::typed_id(user_id, UserId::new)?),
        ("team", _, Some(team_id)) => GoalOwner::Team(mapper::typed_id(team_id, TeamId::new)?),
        _ => return Err(mapper::invalid_column("owner_type", &owner_type)),
    };
    let metric: String = row.try_get("metric")?;
    let period: String = row.try_get("period")?;
    let starts_on: NaiveDate = row.try_get("starts_on")?;
    let ends_on: NaiveDate = row.try_get("ends_on")?;
    let target: Decimal = row.try_get("target")?;
    let credit_mode: String = row.try_get("credit_mode")?;
    let created_at: NaiveDateTime = row.try_get("created_at")?;
    let updated_at: NaiveDateTime = row.try_get("updated_at")?;
    Ok(Goal {
        id: Uuid::parse_str(&id).map_err(|_| mapper::invalid_column("id", &id))?,
        owner,
        owner_name: row.try_get("owner_name")?,
        metric: GoalMetric::parse(&metric)
            .ok_or_else(|| mapper::invalid_column("metric", &metric))?,
        period: GoalPeriod::parse(&period)
            .ok_or_else(|| mapper::invalid_column("period", &period))?,
        starts_on,
        ends_on,
        target,
        credit_mode: GoalCreditMode::parse(&credit_mode)
            .ok_or_else(|| mapper::invalid_column("credit_mode", &credit_mode))?,
        created_by: row
            .try_get::<Option<String>, _>("created_by")?
            .map(|id| mapper::typed_id(id, UserId::new))
            .transpose()?,
        created_at: mapper::utc(created_at),
        updated_at: mapper::utc(updated_at),
    })
}

fn option_from_row(row: &MySqlRow) -> Result<GoalOwnerOption, AppError> {
    Ok(GoalOwnerOption {
        id: row.try_get("id")?,
        name: row.try_get("name")?,
    })
}

#[derive(Clone)]
pub(crate) struct GoalStore {
    pool: MySqlPool,
}

impl GoalStore {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    pub(crate) fn of(ctx: &Ctx<'_>) -> Self {
        Self::new(ctx.pool().clone())
    }

    pub(crate) async fn currency(&self) -> Result<String, AppError> {
        let currency: Option<String> =
            sqlx::query_scalar("SELECT currency FROM organization_settings WHERE id = 1")
                .fetch_optional(&self.pool)
                .await?;
        Ok(currency.unwrap_or_else(|| "BRL".to_owned()))
    }

    pub(crate) async fn list(&self, filter: &GoalListFilter) -> Result<Listing<Goal>, AppError> {
        let mut count = QueryBuilder::<MySql>::new("SELECT COUNT(*) FROM goals g WHERE 1 = 1");
        push_filter(&mut count, filter);
        let total: i64 = count.build_query_scalar().fetch_one(&self.pool).await?;
        if total == 0 {
            return Ok(Listing {
                items: Vec::new(),
                total: 0,
            });
        }
        let mut builder = QueryBuilder::<MySql>::new(GOAL_SELECT);
        push_filter(&mut builder, filter);
        builder
            .push(
                " ORDER BY g.starts_on DESC, FIELD(g.owner_type, 'organization', 'team', 'user'), \
                 owner_name, g.metric, g.id LIMIT ",
            )
            .push_bind(filter.window.limit)
            .push(" OFFSET ")
            .push_bind(filter.window.offset);
        let rows = builder.build().fetch_all(&self.pool).await?;
        Ok(Listing {
            items: rows.iter().map(goal_from_row).collect::<Result<_, _>>()?,
            total: u64::try_from(total).unwrap_or_default(),
        })
    }

    pub(crate) async fn find(&self, goal_id: Uuid) -> Result<Option<Goal>, AppError> {
        let mut builder = QueryBuilder::<MySql>::new(GOAL_SELECT);
        builder.push(LIVE_OWNER);
        builder.push(" AND g.id = ").push_bind(goal_id.to_string());
        builder
            .build()
            .fetch_optional(&self.pool)
            .await?
            .as_ref()
            .map(goal_from_row)
            .transpose()
    }

    pub(crate) async fn find_duplicate(&self, goal: &Goal) -> Result<Option<Uuid>, AppError> {
        let id: Option<String> = sqlx::query_scalar(
            "SELECT id FROM goals WHERE owner_type = ? \
             AND user_id <=> ? AND team_id <=> ? AND metric = ? AND period = ? AND starts_on = ? \
             LIMIT 1",
        )
        .bind(goal.owner.kind().as_str())
        .bind(goal.owner.user_id().map(UserId::as_str))
        .bind(goal.owner.team_id().map(TeamId::as_str))
        .bind(goal.metric.as_str())
        .bind(goal.period.as_str())
        .bind(goal.starts_on)
        .fetch_optional(&self.pool)
        .await?;
        id.map(|id| Uuid::parse_str(&id).map_err(|_| mapper::invalid_column("id", &id)))
            .transpose()
    }

    pub(crate) async fn insert(&self, tx: &mut MySqlTx, goal: &Goal) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO goals (id, owner_type, user_id, team_id, metric, period, \
             starts_on, ends_on, target, credit_mode, created_by, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(goal.id.to_string())
        .bind(goal.owner.kind().as_str())
        .bind(goal.owner.user_id().map(UserId::as_str))
        .bind(goal.owner.team_id().map(TeamId::as_str))
        .bind(goal.metric.as_str())
        .bind(goal.period.as_str())
        .bind(goal.starts_on)
        .bind(goal.ends_on)
        .bind(goal.target)
        .bind(goal.credit_mode.as_str())
        .bind(goal.created_by.as_ref().map(UserId::as_str))
        .bind(mapper::naive(goal.created_at))
        .bind(mapper::naive(goal.updated_at))
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub(crate) async fn update(&self, tx: &mut MySqlTx, goal: &Goal) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE goals SET owner_type = ?, user_id = ?, team_id = ?, metric = ?, period = ?, \
             starts_on = ?, ends_on = ?, target = ?, credit_mode = ?, updated_at = ? \
             WHERE id = ?",
        )
        .bind(goal.owner.kind().as_str())
        .bind(goal.owner.user_id().map(UserId::as_str))
        .bind(goal.owner.team_id().map(TeamId::as_str))
        .bind(goal.metric.as_str())
        .bind(goal.period.as_str())
        .bind(goal.starts_on)
        .bind(goal.ends_on)
        .bind(goal.target)
        .bind(goal.credit_mode.as_str())
        .bind(mapper::naive(goal.updated_at))
        .bind(goal.id.to_string())
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub(crate) async fn lock_orphans(&self, tx: &mut MySqlTx) -> Result<Vec<OrphanGoal>, AppError> {
        let rows = sqlx::query(
            "SELECT g.id, g.owner_type, g.metric, g.period, g.starts_on, g.target FROM goals g \
             WHERE (g.owner_type = 'user' AND g.user_id IS NULL) \
             OR (g.owner_type = 'team' AND g.team_id IS NULL) FOR UPDATE",
        )
        .fetch_all(&mut **tx)
        .await?;
        rows.iter()
            .map(|row| {
                let id: String = row.try_get("id")?;
                Ok(OrphanGoal {
                    id: Uuid::parse_str(&id).map_err(|_| mapper::invalid_column("id", &id))?,
                    owner_type: row.try_get("owner_type")?,
                    metric: row.try_get("metric")?,
                    period: row.try_get("period")?,
                    starts_on: row.try_get("starts_on")?,
                    target: row.try_get("target")?,
                })
            })
            .collect()
    }

    pub(crate) async fn delete(&self, tx: &mut MySqlTx, goal_id: Uuid) -> Result<(), AppError> {
        sqlx::query("DELETE FROM goals WHERE id = ?")
            .bind(goal_id.to_string())
            .execute(&mut **tx)
            .await?;
        Ok(())
    }

    pub(crate) async fn team_members(&self, team_ids: &[TeamId]) -> Result<Vec<UserId>, AppError> {
        if team_ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut builder = QueryBuilder::<MySql>::new(
            "SELECT DISTINCT tm.user_id FROM team_memberships tm WHERE tm.team_id",
        );
        push_list(&mut builder, team_ids);
        let rows: Vec<String> = builder.build_query_scalar().fetch_all(&self.pool).await?;
        rows.into_iter()
            .map(|id| mapper::typed_id(id, UserId::new))
            .collect()
    }

    pub(crate) async fn users(
        &self,
        only: Option<&[UserId]>,
    ) -> Result<Vec<GoalOwnerOption>, AppError> {
        if only.is_some_and(<[UserId]>::is_empty) {
            return Ok(Vec::new());
        }
        let mut builder = QueryBuilder::<MySql>::new(
            "SELECT u.id, u.display_name AS name FROM users u WHERE u.status = 'active'",
        );
        if let Some(ids) = only {
            builder.push(" AND u.id");
            push_list(&mut builder, ids);
        }
        builder.push(" ORDER BY u.display_name, u.id");
        let rows = builder.build().fetch_all(&self.pool).await?;
        rows.iter().map(option_from_row).collect()
    }

    pub(crate) async fn teams(
        &self,
        only: Option<&[TeamId]>,
    ) -> Result<Vec<GoalOwnerOption>, AppError> {
        if only.is_some_and(<[TeamId]>::is_empty) {
            return Ok(Vec::new());
        }
        let mut builder =
            QueryBuilder::<MySql>::new("SELECT t.id, t.name FROM teams t WHERE 1 = 1");
        if let Some(ids) = only {
            builder.push(" AND t.id");
            push_list(&mut builder, ids);
        }
        builder.push(" ORDER BY t.name, t.id");
        let rows = builder.build().fetch_all(&self.pool).await?;
        rows.iter().map(option_from_row).collect()
    }
}
