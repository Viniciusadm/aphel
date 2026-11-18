use chrono::{DateTime, NaiveDateTime, Utc};
use rust_decimal::Decimal;
use sqlx::mysql::MySqlRow;
use sqlx::{MySql, MySqlPool, QueryBuilder, Row};
use uuid::Uuid;

use crate::team::domain::{
    CreditTotals, CreditedUsers, DealMember, DealMemberRole, DealMemberRoleRef, NewDealMember,
};
use aphel_kernel::Ctx;
use aphel_kernel::domain::UserId;
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::application::files::UserReference;
use aphel_sdk::platform::application::query::QueryPlan;
use aphel_sdk::platform::database::transaction::Tx;
use aphel_sdk::platform::domain::access::AccessScope;
use aphel_sdk::platform::domain::users::UserStatus;
use aphel_sdk::platform::infrastructure::fields::mysql_definitions::uuid_column;
use aphel_sdk::platform::infrastructure::query::compiler::{push_where, quoted};
use aphel_sdk::platform::infrastructure::repositories::mapper;

const MEMBER_SELECT: &str = "SELECT m.id, m.deal_id, m.user_id, u.display_name AS user_name, \
     CASE WHEN u.status = ? THEN 1 ELSE 0 END AS user_active, m.role_id, r.`key` AS role_key, \
     r.name AS role_name, m.credit_percent, m.added_by, m.created_at, m.updated_at \
     FROM deal_members m JOIN users u ON u.id = m.user_id \
     LEFT JOIN deal_member_roles r ON r.id = m.role_id \
     WHERE m.deal_id = ? ORDER BY m.created_at, m.id";

const ROLE_COLUMNS: &str = "id, `key`, name, is_active, position, created_at, updated_at";

fn member_from_row(row: &MySqlRow) -> Result<DealMember, AppError> {
    let created_at: NaiveDateTime = row.try_get("created_at")?;
    let updated_at: NaiveDateTime = row.try_get("updated_at")?;
    let user_id: String = row.try_get("user_id")?;
    let added_by: Option<String> = row.try_get("added_by")?;
    let role_id: Option<String> = row.try_get("role_id")?;
    let role_key: Option<String> = row.try_get("role_key")?;
    let role_name: Option<String> = row.try_get("role_name")?;
    let role = match (role_id, role_key, role_name) {
        (Some(id), Some(key), Some(name)) => Some(DealMemberRoleRef {
            id: Uuid::parse_str(&id).map_err(|_| mapper::invalid_column("role_id", &id))?,
            key,
            name,
        }),
        _ => None,
    };
    let user_active: i64 = row.try_get("user_active")?;
    Ok(DealMember {
        id: uuid_column(row, "id")?,
        deal_id: uuid_column(row, "deal_id")?,
        user: UserReference {
            id: UserId::new(user_id)?,
            display_name: row.try_get("user_name")?,
        },
        user_active: user_active != 0,
        role,
        credit_percent: row.try_get("credit_percent")?,
        added_by: added_by.map(UserId::new).transpose()?,
        created_at: mapper::utc(created_at),
        updated_at: mapper::utc(updated_at),
    })
}

fn role_from_row(row: &MySqlRow) -> Result<DealMemberRole, AppError> {
    let created_at: NaiveDateTime = row.try_get("created_at")?;
    let updated_at: NaiveDateTime = row.try_get("updated_at")?;
    Ok(DealMemberRole {
        id: uuid_column(row, "id")?,
        key: row.try_get("key")?,
        name: row.try_get("name")?,
        is_active: row.try_get("is_active")?,
        position: row.try_get("position")?,
        created_at: mapper::utc(created_at),
        updated_at: mapper::utc(updated_at),
    })
}

#[derive(Clone)]
pub(crate) struct DealMemberStore {
    pool: MySqlPool,
}

impl DealMemberStore {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    pub(crate) fn of(ctx: &Ctx<'_>) -> Self {
        Self::new(ctx.pool().clone())
    }

    pub(crate) async fn list(&self, deal_id: Uuid) -> Result<Vec<DealMember>, AppError> {
        let rows = sqlx::query(MEMBER_SELECT)
            .bind(UserStatus::Active.as_str())
            .bind(deal_id.to_string())
            .fetch_all(&self.pool)
            .await?;
        rows.iter().map(member_from_row).collect()
    }

    pub(crate) async fn lock_deal(&self, tx: &mut Tx, deal_id: Uuid) -> Result<bool, AppError> {
        let found: Option<String> = sqlx::query_scalar(
            "SELECT id FROM deals WHERE id = ? AND deleted_at IS NULL FOR UPDATE",
        )
        .bind(deal_id.to_string())
        .fetch_optional(&mut **tx)
        .await?;
        Ok(found.is_some())
    }

    pub(crate) async fn list_locked(
        &self,
        tx: &mut Tx,
        deal_id: Uuid,
    ) -> Result<Vec<DealMember>, AppError> {
        let rows = sqlx::query(MEMBER_SELECT)
            .bind(UserStatus::Active.as_str())
            .bind(deal_id.to_string())
            .fetch_all(&mut **tx)
            .await?;
        rows.iter().map(member_from_row).collect()
    }

    pub(crate) async fn insert(&self, tx: &mut Tx, member: &NewDealMember) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO deal_members (id, deal_id, user_id, role_id, credit_percent, \
             added_by, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(member.id.to_string())
        .bind(member.deal_id.to_string())
        .bind(member.user_id.as_str())
        .bind(member.role_id.map(|id| id.to_string()))
        .bind(member.credit_percent)
        .bind(member.added_by.as_ref().map(UserId::as_str))
        .bind(mapper::naive(member.at))
        .bind(mapper::naive(member.at))
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub(crate) async fn update(
        &self,
        tx: &mut Tx,
        member_id: Uuid,
        role_id: Option<Uuid>,
        credit_percent: Decimal,
        at: DateTime<Utc>,
    ) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE deal_members SET role_id = ?, credit_percent = ?, updated_at = ? \
             WHERE id = ?",
        )
        .bind(role_id.map(|id| id.to_string()))
        .bind(credit_percent)
        .bind(mapper::naive(at))
        .bind(member_id.to_string())
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub(crate) async fn delete(
        &self,
        tx: &mut Tx,
        deal_id: Uuid,
        member_id: Uuid,
    ) -> Result<(), AppError> {
        sqlx::query("DELETE FROM deal_members WHERE deal_id = ? AND id = ?")
            .bind(deal_id.to_string())
            .bind(member_id.to_string())
            .execute(&mut **tx)
            .await?;
        Ok(())
    }

    pub(crate) async fn active_user_name(
        &self,
        user_id: &UserId,
    ) -> Result<Option<String>, AppError> {
        Ok(
            sqlx::query_scalar("SELECT display_name FROM users WHERE id = ? AND status = ?")
                .bind(user_id.as_str())
                .bind(UserStatus::Active.as_str())
                .fetch_optional(&self.pool)
                .await?,
        )
    }
}

#[derive(Clone)]
pub(crate) struct DealMemberRoleStore {
    pool: MySqlPool,
}

impl DealMemberRoleStore {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    pub(crate) fn of(ctx: &Ctx<'_>) -> Self {
        Self::new(ctx.pool().clone())
    }

    pub(crate) async fn list(
        &self,
        include_inactive: bool,
    ) -> Result<Vec<DealMemberRole>, AppError> {
        let active = if include_inactive {
            ""
        } else {
            " AND is_active = TRUE"
        };
        let rows = sqlx::query(sqlx::AssertSqlSafe(format!(
            "SELECT {ROLE_COLUMNS} FROM deal_member_roles WHERE TRUE{active} \
             ORDER BY position, created_at"
        )))
        .fetch_all(&self.pool)
        .await?;
        rows.iter().map(role_from_row).collect()
    }

    pub(crate) async fn find(&self, id: Uuid) -> Result<Option<DealMemberRole>, AppError> {
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "SELECT {ROLE_COLUMNS} FROM deal_member_roles WHERE id = ?"
        )))
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await?
        .as_ref()
        .map(role_from_row)
        .transpose()
    }

    pub(crate) async fn insert(&self, tx: &mut Tx, role: &DealMemberRole) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO deal_member_roles (id, `key`, name, is_active, position, \
             created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(role.id.to_string())
        .bind(&role.key)
        .bind(&role.name)
        .bind(role.is_active)
        .bind(role.position)
        .bind(mapper::naive(role.created_at))
        .bind(mapper::naive(role.updated_at))
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub(crate) async fn update(&self, tx: &mut Tx, role: &DealMemberRole) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE deal_member_roles SET name = ?, is_active = ?, updated_at = ? \
             WHERE id = ?",
        )
        .bind(&role.name)
        .bind(role.is_active)
        .bind(mapper::naive(role.updated_at))
        .bind(role.id.to_string())
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub(crate) async fn delete(&self, tx: &mut Tx, id: Uuid) -> Result<(), AppError> {
        sqlx::query("DELETE FROM deal_member_roles WHERE id = ?")
            .bind(id.to_string())
            .execute(&mut **tx)
            .await?;
        Ok(())
    }

    pub(crate) async fn reorder(
        &self,
        tx: &mut Tx,
        ordered_ids: &[Uuid],
        now: DateTime<Utc>,
    ) -> Result<(), AppError> {
        for (position, id) in ordered_ids.iter().enumerate() {
            sqlx::query(
                "UPDATE deal_member_roles SET position = ?, updated_at = ? \
                 WHERE id = ?",
            )
            .bind(position as i32)
            .bind(mapper::naive(now))
            .bind(id.to_string())
            .execute(&mut **tx)
            .await?;
        }
        Ok(())
    }
}

fn push_credited_users(builder: &mut QueryBuilder<MySql>, credited: &CreditedUsers) {
    match credited {
        CreditedUsers::User(user_id) => {
            builder.push_bind(user_id.as_str().to_owned());
        }
        CreditedUsers::Team(team_id) => {
            builder
                .push("SELECT ctm.user_id FROM team_memberships ctm WHERE ctm.team_id = ")
                .push_bind(team_id.as_str().to_owned());
        }
    }
}

pub fn push_credit_share(builder: &mut QueryBuilder<MySql>, credited: &CreditedUsers, alias: &str) {
    builder
        .push(
            "((COALESCE((SELECT SUM(csm.credit_percent) FROM deal_members csm \
             WHERE ",
        )
        .push(format!("csm.deal_id = {alias}.id AND csm.user_id IN ("));
    push_credited_users(builder, credited);
    builder.push(format!(")), 0) + CASE WHEN {alias}.owner_id IN ("));
    push_credited_users(builder, credited);
    builder
        .push(
            ") THEN 100 - COALESCE((SELECT SUM(cso.credit_percent) FROM deal_members cso \
             WHERE ",
        )
        .push(format!("cso.deal_id = {alias}.id), 0) ELSE 0 END) / 100)"));
}

pub fn push_credited_filter(
    builder: &mut QueryBuilder<MySql>,
    credited: &CreditedUsers,
    alias: &str,
) {
    builder.push(format!(" AND ({alias}.owner_id IN ("));
    push_credited_users(builder, credited);
    builder
        .push(format!(
            ") OR {alias}.id IN (SELECT cfm.deal_id FROM deal_members cfm WHERE "
        ))
        .push("cfm.user_id IN (");
    push_credited_users(builder, credited);
    builder.push(")))");
}

#[derive(Clone)]
pub(crate) struct DealCreditReader {
    pool: MySqlPool,
}

impl DealCreditReader {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    pub(crate) fn of(ctx: &Ctx<'_>) -> Self {
        Self::new(ctx.pool().clone())
    }

    pub(crate) async fn credited_total(
        &self,
        scope: &AccessScope,
        plan: &QueryPlan,
        amount_column: &str,
        credited: &CreditedUsers,
    ) -> Result<CreditTotals, AppError> {
        let mut builder = QueryBuilder::<MySql>::new("SELECT COUNT(*) AS deals, SUM(r.");
        builder.push(quoted(amount_column)).push(" * ");
        push_credit_share(&mut builder, credited, "r");
        builder.push(format!(
            ") AS amount FROM {} r",
            quoted(&plan.record_type.table)
        ));
        push_where(&mut builder, scope, plan)?;
        builder.push(" AND ");
        push_credit_share(&mut builder, credited, "r");
        builder.push(" > 0");
        let row = builder.build().fetch_one(&self.pool).await?;
        let deals: i64 = row.try_get("deals")?;
        let amount: Option<Decimal> = row.try_get("amount")?;
        Ok(CreditTotals {
            count: u64::try_from(deals).unwrap_or_default(),
            amount: amount.unwrap_or_default().round_dp(2),
        })
    }
}
