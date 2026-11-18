use std::collections::BTreeMap;

use async_trait::async_trait;
use chrono::{DateTime, NaiveDateTime, Utc};
use sqlx::mysql::MySqlRow;
use sqlx::types::Json;
use sqlx::{MySql, MySqlPool, QueryBuilder, Row};

use crate::application::auth::{NewUserToken, UserTokenPurpose};
use crate::application::organization_admin::{
    ALREADY_MEMBER, InvitationContext, LAST_ADMIN, ListWindow, Listing, MemberRepository,
    MemberView, NewInvitation, RoleSummary, TeamReference,
};
use crate::database::transaction::Tx;
use crate::domain::access::RoleKey;
use crate::domain::users::UserStatus;
use crate::domain::{Email, RoleId, TeamId, UserId};
use crate::errors::AppError;
use crate::infrastructure::repositories::mapper;

macro_rules! member_select {
    () => {
        "SELECT u.id AS user_id, u.email, u.display_name, u.role_id, r.`key` AS role_key, r.name AS role_name, r.is_system, u.status, u.invited_by, u.joined_at, u.created_at, u.updated_at \
     FROM users u \
     JOIN roles r ON r.id = u.role_id"
    };
}

#[derive(Clone)]
pub struct MySqlMemberRepository {
    pool: MySqlPool,
}

impl MySqlMemberRepository {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    async fn attach_teams(&self, members: &mut [MemberView]) -> Result<(), AppError> {
        if members.is_empty() {
            return Ok(());
        }
        let mut builder = QueryBuilder::<MySql>::new(
            "SELECT tm.user_id, t.id, t.name FROM team_memberships tm JOIN teams t ON t.id = tm.team_id WHERE tm.user_id IN (",
        );
        let mut separated = builder.separated(", ");
        for member in members.iter() {
            separated.push_bind(member.user_id.as_str());
        }
        builder.push(") ORDER BY t.name");

        let mut teams: BTreeMap<String, Vec<TeamReference>> = BTreeMap::new();
        for row in builder.build().fetch_all(&self.pool).await? {
            teams
                .entry(row.try_get("user_id")?)
                .or_default()
                .push(TeamReference {
                    id: mapper::typed_id(row.try_get("id")?, TeamId::new)?,
                    name: row.try_get("name")?,
                });
        }
        for member in members.iter_mut() {
            member.teams = teams.remove(member.user_id.as_str()).unwrap_or_default();
        }
        Ok(())
    }
}

fn member_status(value: String) -> Result<UserStatus, AppError> {
    UserStatus::parse(&value).ok_or_else(|| mapper::invalid_column("users.status", &value))
}

fn member_from_row(row: &MySqlRow) -> Result<MemberView, AppError> {
    let invited_by: Option<String> = row.try_get("invited_by")?;
    let joined_at: Option<NaiveDateTime> = row.try_get("joined_at")?;
    Ok(MemberView {
        user_id: mapper::typed_id(row.try_get("user_id")?, UserId::new)?,
        email: row.try_get("email")?,
        display_name: row.try_get("display_name")?,
        role: RoleSummary {
            id: mapper::typed_id(row.try_get("role_id")?, RoleId::new)?,
            key: RoleKey::new(row.try_get::<String, _>("role_key")?)?,
            name: row.try_get("role_name")?,
            is_system: row.try_get("is_system")?,
        },
        status: member_status(row.try_get("status")?)?,
        invited_by: invited_by
            .map(|id| mapper::typed_id(id, UserId::new))
            .transpose()?,
        joined_at: joined_at.map(mapper::utc),
        created_at: mapper::utc(row.try_get("created_at")?),
        updated_at: mapper::utc(row.try_get("updated_at")?),
        teams: Vec::new(),
    })
}

fn is_unique_violation(error: &sqlx::Error) -> bool {
    matches!(error, sqlx::Error::Database(database_error) if database_error.is_unique_violation())
}

async fn insert_token(
    transaction: &mut sqlx::MySqlConnection,
    token: &NewUserToken,
) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO user_tokens (id, user_id, purpose, token_hash, payload, expires_at) VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(&token.id)
    .bind(token.user_id.as_str())
    .bind(token.purpose.as_str())
    .bind(&token.token_hash)
    .bind(token.payload.clone().map(Json))
    .bind(mapper::naive(token.expires_at))
    .execute(&mut *transaction)
    .await?;
    Ok(())
}

async fn invalidate_invitation_tokens(
    transaction: &mut sqlx::MySqlConnection,
    user_id: &UserId,
    at: DateTime<Utc>,
) -> Result<(), AppError> {
    sqlx::query(
        "UPDATE user_tokens SET used_at = ? WHERE purpose = ? AND used_at IS NULL AND user_id = ?",
    )
    .bind(mapper::naive(at))
    .bind(UserTokenPurpose::Invitation.as_str())
    .bind(user_id.as_str())
    .execute(&mut *transaction)
    .await?;
    Ok(())
}

async fn lock_status(
    transaction: &mut sqlx::MySqlConnection,
    user_id: &UserId,
) -> Result<Option<UserStatus>, AppError> {
    let status: Option<String> =
        sqlx::query_scalar("SELECT status FROM users WHERE id = ? FOR UPDATE")
            .bind(user_id.as_str())
            .fetch_optional(&mut *transaction)
            .await?;
    status.map(member_status).transpose()
}

async fn ensure_admin_remains(
    transaction: &mut sqlx::MySqlConnection,
    user_id: &UserId,
    keeps_admin: bool,
) -> Result<(), AppError> {
    let admins: Vec<String> = sqlx::query_scalar(
        "SELECT u.id FROM users u JOIN roles r ON r.id = u.role_id \
         WHERE u.status = ? AND r.is_system = TRUE AND r.`key` = ? ORDER BY u.id FOR UPDATE",
    )
    .bind(UserStatus::Active.as_str())
    .bind(RoleKey::ADMIN)
    .fetch_all(&mut *transaction)
    .await?;
    let is_admin = admins.iter().any(|id| id == user_id.as_str());
    if is_admin && !keeps_admin && admins.len() <= 1 {
        return Err(AppError::conflict_code(
            LAST_ADMIN,
            "the organization needs at least one active administrator",
        ));
    }
    Ok(())
}

async fn revoke_sessions(
    transaction: &mut Tx,
    pool: &MySqlPool,
    user_id: &UserId,
    at: DateTime<Utc>,
) -> Result<(), AppError> {
    sqlx::query("UPDATE sessions SET revoked_at = ? WHERE user_id = ? AND revoked_at IS NULL")
        .bind(mapper::naive(at))
        .bind(user_id.as_str())
        .execute(&mut **transaction)
        .await?;
    crate::application::security::session_revoked(transaction, pool, None, Some(user_id), None)
        .await?;
    Ok(())
}

fn member_missing() -> AppError {
    AppError::NotFound("member was not found".to_owned())
}

#[async_trait]
impl MemberRepository for MySqlMemberRepository {
    type Tx = Tx;

    async fn list(&self, window: ListWindow) -> Result<Listing<MemberView>, AppError> {
        let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
            .fetch_one(&self.pool)
            .await?;

        let rows = sqlx::query(concat!(
            member_select!(),
            " ORDER BY u.display_name, u.email, u.id LIMIT ? OFFSET ?"
        ))
        .bind(window.limit)
        .bind(window.offset)
        .fetch_all(&self.pool)
        .await?;
        let mut items = rows
            .iter()
            .map(member_from_row)
            .collect::<Result<Vec<_>, _>>()?;
        self.attach_teams(&mut items).await?;

        Ok(Listing {
            items,
            total: u64::try_from(total).unwrap_or_default(),
        })
    }

    async fn find(&self, user_id: &UserId) -> Result<Option<MemberView>, AppError> {
        let row = sqlx::query(concat!(member_select!(), " WHERE u.id = ?"))
            .bind(user_id.as_str())
            .fetch_optional(&self.pool)
            .await?;
        let Some(row) = row else {
            return Ok(None);
        };
        let mut members = vec![member_from_row(&row)?];
        self.attach_teams(&mut members).await?;
        Ok(members.pop())
    }

    async fn email_exists(&self, email: &Email) -> Result<bool, AppError> {
        let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM users WHERE email = ?)")
            .bind(email.as_str())
            .fetch_one(&self.pool)
            .await?;
        Ok(exists)
    }

    async fn invitation_context(
        &self,
        inviter_id: &UserId,
        invitee_email: &str,
    ) -> Result<InvitationContext, AppError> {
        let row = sqlx::query(
            "SELECT o.name AS organization_name, o.locale AS organization_locale, u.display_name, \
             (SELECT invitee.locale FROM users invitee WHERE invitee.email = ?) AS invitee_locale \
             FROM organization_settings o JOIN users u ON u.id = ? WHERE o.id = 1",
        )
        .bind(invitee_email.trim().to_ascii_lowercase())
        .bind(inviter_id.as_str())
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::NotFound("organization was not found".to_owned()))?;
        Ok(InvitationContext {
            organization_name: row.try_get("organization_name")?,
            inviter_display_name: row.try_get("display_name")?,
            organization_locale: row.try_get("organization_locale")?,
            invitee_locale: row.try_get("invitee_locale")?,
        })
    }

    async fn create_invitation(
        &self,
        tx: &mut Tx,
        invitation: NewInvitation,
    ) -> Result<(), AppError> {
        let user = &invitation.user;
        let inserted = sqlx::query(
            "INSERT INTO users (id, email, password_hash, display_name, status, role_id, invited_by, email_verified_at) \
             SELECT ?, ?, NULL, ?, ?, r.id, ?, CURRENT_TIMESTAMP(6) FROM roles r WHERE r.id = ?",
        )
        .bind(user.id.as_str())
        .bind(user.email.as_str())
        .bind(user.display_name.as_str())
        .bind(UserStatus::Invited.as_str())
        .bind(invitation.invited_by.as_str())
        .bind(invitation.role_id.as_str())
        .execute(&mut **tx)
        .await
        .map_err(|error| {
            if is_unique_violation(&error) {
                AppError::conflict_code(
                    ALREADY_MEMBER,
                    "this user already belongs to the organization",
                )
            } else {
                AppError::from(error)
            }
        })?;
        if inserted.rows_affected() != 1 {
            return Err(AppError::NotFound("role was not found".to_owned()));
        }

        insert_token(&mut *tx, &invitation.token).await?;
        Ok(())
    }

    async fn replace_invitation_token(
        &self,
        tx: &mut Tx,
        user_id: &UserId,
        token: NewUserToken,
        at: DateTime<Utc>,
    ) -> Result<(), AppError> {
        match lock_status(&mut *tx, user_id).await? {
            Some(UserStatus::Invited) if user_id == &token.user_id => {}
            _ => return Err(member_missing()),
        }
        invalidate_invitation_tokens(&mut *tx, user_id, at).await?;
        insert_token(&mut *tx, &token).await?;
        Ok(())
    }

    async fn cancel_invitation(
        &self,
        tx: &mut Tx,
        user_id: &UserId,
        at: DateTime<Utc>,
    ) -> Result<bool, AppError> {
        if lock_status(&mut *tx, user_id).await? != Some(UserStatus::Invited) {
            return Ok(false);
        }
        invalidate_invitation_tokens(&mut *tx, user_id, at).await?;
        sqlx::query("DELETE FROM users WHERE id = ? AND status = ?")
            .bind(user_id.as_str())
            .bind(UserStatus::Invited.as_str())
            .execute(&mut **tx)
            .await?;
        Ok(true)
    }

    async fn change_role(
        &self,
        tx: &mut Self::Tx,
        user_id: &UserId,
        role_id: &RoleId,
    ) -> Result<(), AppError> {
        let admin_role: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM roles WHERE id = ? AND is_system = TRUE AND `key` = ?)",
        )
        .bind(role_id.as_str())
        .bind(RoleKey::ADMIN)
        .fetch_one(&mut **tx)
        .await?;
        ensure_admin_remains(tx, user_id, admin_role).await?;
        if lock_status(tx, user_id).await?.is_none() {
            return Err(member_missing());
        }
        let role_exists: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM roles WHERE id = ?)")
                .bind(role_id.as_str())
                .fetch_one(&mut **tx)
                .await?;
        if !role_exists {
            return Err(AppError::NotFound("role was not found".to_owned()));
        }
        sqlx::query("UPDATE users SET role_id = ?, updated_at = UTC_TIMESTAMP(6) WHERE id = ?")
            .bind(role_id.as_str())
            .bind(user_id.as_str())
            .execute(&mut **tx)
            .await?;
        Ok(())
    }

    async fn deactivate(
        &self,
        tx: &mut Tx,
        user_id: &UserId,
        at: DateTime<Utc>,
    ) -> Result<(), AppError> {
        ensure_admin_remains(&mut *tx, user_id, false).await?;
        if lock_status(&mut *tx, user_id).await?.is_none() {
            return Err(member_missing());
        }
        sqlx::query("UPDATE users SET status = ?, updated_at = UTC_TIMESTAMP(6) WHERE id = ?")
            .bind(UserStatus::Deactivated.as_str())
            .bind(user_id.as_str())
            .execute(&mut **tx)
            .await?;
        revoke_sessions(tx, &self.pool, user_id, at).await?;
        Ok(())
    }

    async fn reactivate(&self, tx: &mut Tx, user_id: &UserId) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE users SET status = ?, updated_at = UTC_TIMESTAMP(6) WHERE id = ? AND status = ?",
        )
        .bind(UserStatus::Active.as_str())
        .bind(user_id.as_str())
        .bind(UserStatus::Deactivated.as_str())
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    async fn remove(
        &self,
        tx: &mut Tx,
        user_id: &UserId,
        at: DateTime<Utc>,
    ) -> Result<(), AppError> {
        ensure_admin_remains(&mut *tx, user_id, false).await?;
        let Some(status) = lock_status(&mut *tx, user_id).await? else {
            return Err(member_missing());
        };
        invalidate_invitation_tokens(&mut *tx, user_id, at).await?;
        if status == UserStatus::Invited {
            sqlx::query("DELETE FROM users WHERE id = ?")
                .bind(user_id.as_str())
                .execute(&mut **tx)
                .await?;
            return Ok(());
        }
        sqlx::query("DELETE FROM team_memberships WHERE user_id = ?")
            .bind(user_id.as_str())
            .execute(&mut **tx)
            .await?;
        sqlx::query("UPDATE users SET status = ?, updated_at = UTC_TIMESTAMP(6) WHERE id = ?")
            .bind(UserStatus::Deactivated.as_str())
            .bind(user_id.as_str())
            .execute(&mut **tx)
            .await?;
        revoke_sessions(tx, &self.pool, user_id, at).await?;
        Ok(())
    }

    async fn count_active_admins(&self) -> Result<u64, AppError> {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM users u JOIN roles r ON r.id = u.role_id \
             WHERE u.status = ? AND r.is_system = TRUE AND r.`key` = ?",
        )
        .bind(UserStatus::Active.as_str())
        .bind(RoleKey::ADMIN)
        .fetch_one(&self.pool)
        .await?;
        Ok(u64::try_from(count).unwrap_or_default())
    }
}
