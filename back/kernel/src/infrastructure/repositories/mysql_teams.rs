use std::collections::BTreeMap;

use async_trait::async_trait;
use sqlx::mysql::MySqlRow;
use sqlx::{MySql, MySqlPool, QueryBuilder, Row};

use crate::application::organization_admin::{
    ListWindow, Listing, TeamDraft, TeamMemberView, TeamRepository, TeamView,
};
use crate::database::transaction::Tx;
use crate::domain::{TeamId, UserId};
use crate::errors::AppError;
use crate::infrastructure::repositories::mapper;

const MAX_TEAM_DEPTH: u32 = 1000;

#[derive(Clone)]
pub struct MySqlTeamRepository {
    pool: MySqlPool,
}

impl MySqlTeamRepository {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    async fn with_members(&self, rows: Vec<MySqlRow>) -> Result<Vec<TeamView>, AppError> {
        let mut teams = rows
            .iter()
            .map(team_from_row)
            .collect::<Result<Vec<_>, _>>()?;
        if teams.is_empty() {
            return Ok(teams);
        }

        let mut builder = QueryBuilder::<MySql>::new(
            "SELECT tm.team_id, tm.user_id, u.email, u.display_name, tm.is_manager \
             FROM team_memberships tm JOIN users u ON u.id = tm.user_id WHERE tm.team_id IN (",
        );
        let mut separated = builder.separated(", ");
        for team in &teams {
            separated.push_bind(team.id.as_str());
        }
        builder.push(") ORDER BY tm.is_manager DESC, u.display_name, u.email");

        let mut members: BTreeMap<String, Vec<TeamMemberView>> = BTreeMap::new();
        for row in builder.build().fetch_all(&self.pool).await? {
            members
                .entry(row.try_get("team_id")?)
                .or_default()
                .push(TeamMemberView {
                    user_id: mapper::typed_id(row.try_get("user_id")?, UserId::new)?,
                    email: row.try_get("email")?,
                    display_name: row.try_get("display_name")?,
                    is_manager: row.try_get("is_manager")?,
                });
        }
        for team in &mut teams {
            team.members = members.remove(team.id.as_str()).unwrap_or_default();
        }
        Ok(teams)
    }
}

fn team_from_row(row: &MySqlRow) -> Result<TeamView, AppError> {
    let parent_team_id: Option<String> = row.try_get("parent_team_id")?;
    Ok(TeamView {
        id: mapper::typed_id(row.try_get("id")?, TeamId::new)?,
        name: row.try_get("name")?,
        parent_team_id: parent_team_id
            .map(|id| mapper::typed_id(id, TeamId::new))
            .transpose()?,
        members: Vec::new(),
        updated_at: mapper::utc(row.try_get("updated_at")?),
    })
}

async fn ensure_parent(
    transaction: &mut sqlx::MySqlConnection,
    draft: &TeamDraft,
) -> Result<(), AppError> {
    let Some(parent) = &draft.parent_team_id else {
        return Ok(());
    };
    let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM teams WHERE id = ?)")
        .bind(parent.as_str())
        .fetch_one(&mut *transaction)
        .await?;
    if !exists {
        return Err(AppError::NotFound("parent team was not found".to_owned()));
    }
    Ok(())
}

async fn insert_members(
    transaction: &mut sqlx::MySqlConnection,
    team_id: &TeamId,
    draft: &TeamDraft,
) -> Result<(), AppError> {
    for member in &draft.members {
        let inserted = sqlx::query(
            "INSERT INTO team_memberships (team_id, user_id, is_manager) \
             SELECT ?, u.id, ? FROM users u WHERE u.id = ?",
        )
        .bind(team_id.as_str())
        .bind(member.is_manager)
        .bind(member.user_id.as_str())
        .execute(&mut *transaction)
        .await?;
        if inserted.rows_affected() != 1 {
            return Err(AppError::NotFound("team member was not found".to_owned()));
        }
    }
    Ok(())
}

#[async_trait]
impl TeamRepository for MySqlTeamRepository {
    type Tx = Tx;

    async fn list(&self, window: ListWindow) -> Result<Listing<TeamView>, AppError> {
        let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM teams")
            .fetch_one(&self.pool)
            .await?;
        let rows = sqlx::query(
            "SELECT id, name, parent_team_id, updated_at FROM teams ORDER BY name, id LIMIT ? OFFSET ?",
        )
        .bind(window.limit)
        .bind(window.offset)
        .fetch_all(&self.pool)
        .await?;
        Ok(Listing {
            items: self.with_members(rows).await?,
            total: u64::try_from(total).unwrap_or_default(),
        })
    }

    async fn find(&self, team_id: &TeamId) -> Result<Option<TeamView>, AppError> {
        let rows =
            sqlx::query("SELECT id, name, parent_team_id, updated_at FROM teams WHERE id = ?")
                .bind(team_id.as_str())
                .fetch_all(&self.pool)
                .await?;
        Ok(self.with_members(rows).await?.pop())
    }

    async fn ancestors(&self, team_id: &TeamId) -> Result<Vec<TeamId>, AppError> {
        sqlx::query_scalar::<_, String>(
            "WITH RECURSIVE chain (id, parent_team_id, depth) AS ( \
               SELECT id, parent_team_id, 0 FROM teams WHERE id = ? \
               UNION ALL \
               SELECT t.id, t.parent_team_id, c.depth + 1 FROM teams t JOIN chain c ON t.id = c.parent_team_id WHERE c.depth < ? \
             ) SELECT id FROM chain ORDER BY depth",
        )
        .bind(team_id.as_str())
        .bind(MAX_TEAM_DEPTH)
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(|id| mapper::typed_id(id, TeamId::new))
        .collect()
    }

    async fn members_of_organization(&self, user_ids: &[UserId]) -> Result<Vec<UserId>, AppError> {
        if user_ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut builder = QueryBuilder::<MySql>::new(
            "SELECT id FROM users WHERE status <> 'invited' AND id IN (",
        );
        let mut separated = builder.separated(", ");
        for user_id in user_ids {
            separated.push_bind(user_id.as_str());
        }
        builder.push(")");
        builder
            .build_query_scalar::<String>()
            .fetch_all(&self.pool)
            .await?
            .into_iter()
            .map(|id| mapper::typed_id(id, UserId::new))
            .collect()
    }

    async fn create(
        &self,
        tx: &mut Tx,
        team_id: &TeamId,
        draft: TeamDraft,
    ) -> Result<(), AppError> {
        ensure_parent(&mut *tx, &draft).await?;
        sqlx::query("INSERT INTO teams (id, parent_team_id, name) VALUES (?, ?, ?)")
            .bind(team_id.as_str())
            .bind(draft.parent_team_id.as_ref().map(TeamId::as_str))
            .bind(draft.name.as_str())
            .execute(&mut **tx)
            .await?;
        insert_members(&mut *tx, team_id, &draft).await?;
        Ok(())
    }

    async fn update(
        &self,
        tx: &mut Tx,
        team_id: &TeamId,
        draft: TeamDraft,
    ) -> Result<(), AppError> {
        let exists: Option<String> =
            sqlx::query_scalar("SELECT id FROM teams WHERE id = ? FOR UPDATE")
                .bind(team_id.as_str())
                .fetch_optional(&mut **tx)
                .await?;
        if exists.is_none() {
            return Err(AppError::NotFound("team was not found".to_owned()));
        }
        ensure_parent(&mut *tx, &draft).await?;
        sqlx::query(
            "UPDATE teams SET name = ?, parent_team_id = ?, updated_at = UTC_TIMESTAMP(6) WHERE id = ?",
        )
        .bind(draft.name.as_str())
        .bind(draft.parent_team_id.as_ref().map(TeamId::as_str))
        .bind(team_id.as_str())
        .execute(&mut **tx)
        .await?;
        sqlx::query("DELETE FROM team_memberships WHERE team_id = ?")
            .bind(team_id.as_str())
            .execute(&mut **tx)
            .await?;
        insert_members(&mut *tx, team_id, &draft).await?;
        Ok(())
    }

    async fn delete(&self, tx: &mut Tx, team_id: &TeamId) -> Result<(), AppError> {
        let deleted = sqlx::query("DELETE FROM teams WHERE id = ?")
            .bind(team_id.as_str())
            .execute(&mut **tx)
            .await?;
        if deleted.rows_affected() != 1 {
            return Err(AppError::NotFound("team was not found".to_owned()));
        }
        Ok(())
    }
}

/// `(id, name)` of the given teams, ordered by name; unknown ids are skipped.
pub(crate) async fn team_names(
    pool: &MySqlPool,
    ids: &[String],
) -> Result<Vec<(String, String)>, sqlx::Error> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let mut builder = QueryBuilder::<MySql>::new("SELECT id, name FROM teams WHERE id IN (");
    let mut separated = builder.separated(", ");
    for id in ids {
        separated.push_bind(id.clone());
    }
    builder.push(") ORDER BY name");
    builder.build_query_as().fetch_all(pool).await
}
