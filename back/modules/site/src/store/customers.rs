use aphel_contract_site::SiteCustomer;
use aphel_sdk::platform::database::Db;
use aphel_sdk::platform::database::transaction::Tx;
use chrono::{DateTime, Utc};
use sqlx::Row;
use sqlx::mysql::MySqlRow;
use uuid::Uuid;

pub(crate) struct CustomerExport {
    pub id: String,
    pub email: String,
    pub name: String,
    pub email_verified_at: Option<DateTime<Utc>>,
    pub last_login_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

pub(crate) struct LiveSession {
    pub customer: SiteCustomer,
    pub session_id: String,
    pub stale: bool,
}

fn customer_of(row: &MySqlRow) -> Result<SiteCustomer, sqlx::Error> {
    Ok(SiteCustomer {
        id: row.try_get("id")?,
        email: row.try_get("email")?,
        name: row.try_get("name")?,
        email_verified: row
            .try_get::<Option<DateTime<Utc>>, _>("email_verified_at")?
            .is_some(),
    })
}

pub(crate) async fn by_email(pool: &Db, email: &str) -> Result<Option<SiteCustomer>, sqlx::Error> {
    let row = sqlx::query(
        "SELECT id, email, name, email_verified_at FROM site_customers WHERE email = ? AND deleted_at IS NULL",
    )
    .bind(email)
    .fetch_optional(pool)
    .await?;
    row.as_ref().map(customer_of).transpose()
}

pub(crate) async fn export(pool: &Db, id: &str) -> Result<Option<CustomerExport>, sqlx::Error> {
    let row = sqlx::query(
        "SELECT id, email, name, email_verified_at, last_login_at, created_at FROM site_customers WHERE id = ? AND deleted_at IS NULL",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    row.map(|row| {
        Ok(CustomerExport {
            id: row.try_get("id")?,
            email: row.try_get("email")?,
            name: row.try_get("name")?,
            email_verified_at: row.try_get("email_verified_at")?,
            last_login_at: row.try_get("last_login_at")?,
            created_at: row.try_get("created_at")?,
        })
    })
    .transpose()
}

pub(crate) async fn insert(
    tx: &mut Tx,
    id: &str,
    email: &str,
    name: &str,
    password_hash: Option<String>,
) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO site_customers (id, email, name, password_hash) VALUES (?, ?, ?, ?)")
        .bind(id)
        .bind(email)
        .bind(name)
        .bind(password_hash)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

pub(crate) async fn credentials(
    pool: &Db,
    email: &str,
) -> Result<Option<(SiteCustomer, Option<String>)>, sqlx::Error> {
    let row = sqlx::query(
        "SELECT id, email, name, email_verified_at, password_hash FROM site_customers WHERE email = ? AND deleted_at IS NULL",
    )
    .bind(email)
    .fetch_optional(pool)
    .await?;
    row.map(|row| Ok((customer_of(&row)?, row.try_get("password_hash")?)))
        .transpose()
}

pub(crate) async fn start_session(
    pool: &Db,
    customer_id: &str,
    token_hash: &str,
    ip: Option<String>,
    user_agent: Option<String>,
    days: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO site_customer_sessions (id, customer_id, token_hash, ip, user_agent, expires_at) \
         VALUES (?, ?, ?, ?, ?, NOW(6) + INTERVAL ? DAY)",
    )
    .bind(Uuid::now_v7().to_string())
    .bind(customer_id)
    .bind(token_hash)
    .bind(ip)
    .bind(user_agent)
    .bind(days)
    .execute(pool)
    .await?;
    sqlx::query("UPDATE site_customers SET last_login_at = NOW(6) WHERE id = ?")
        .bind(customer_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub(crate) async fn live_session(
    pool: &Db,
    token_hash: &str,
    stale_after_minutes: i64,
) -> Result<Option<LiveSession>, sqlx::Error> {
    let row = sqlx::query(
        "SELECT c.id, c.email, c.name, c.email_verified_at, s.id AS session_id, \
         s.last_seen_at < NOW(6) - INTERVAL ? MINUTE AS stale \
         FROM site_customer_sessions s JOIN site_customers c ON c.id = s.customer_id \
         WHERE s.token_hash = ? AND s.revoked_at IS NULL AND s.expires_at > NOW(6) AND c.deleted_at IS NULL",
    )
    .bind(stale_after_minutes)
    .bind(token_hash)
    .fetch_optional(pool)
    .await?;
    row.map(|row| {
        Ok(LiveSession {
            customer: customer_of(&row)?,
            session_id: row.try_get("session_id")?,
            stale: row.try_get::<i64, _>("stale")? != 0,
        })
    })
    .transpose()
}

pub(crate) async fn extend_session(
    pool: &Db,
    session_id: &str,
    days: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE site_customer_sessions SET last_seen_at = NOW(6), expires_at = NOW(6) + INTERVAL ? DAY WHERE id = ?",
    )
    .bind(days)
    .bind(session_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub(crate) async fn revoke_session(pool: &Db, token_hash: &str) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE site_customer_sessions SET revoked_at = NOW(6) WHERE token_hash = ?")
        .bind(token_hash)
        .execute(pool)
        .await?;
    Ok(())
}

pub(crate) async fn replace_token(
    pool: &Db,
    customer_id: &str,
    purpose: &str,
    token_hash: &str,
    lifetime_minutes: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE site_customer_tokens SET used_at = NOW(6) WHERE customer_id = ? AND purpose = ? AND used_at IS NULL",
    )
    .bind(customer_id)
    .bind(purpose)
    .execute(pool)
    .await?;
    sqlx::query(
        "INSERT INTO site_customer_tokens (id, customer_id, purpose, token_hash, expires_at) \
         VALUES (?, ?, ?, ?, NOW(6) + INTERVAL ? MINUTE)",
    )
    .bind(Uuid::now_v7().to_string())
    .bind(customer_id)
    .bind(purpose)
    .bind(token_hash)
    .bind(lifetime_minutes)
    .execute(pool)
    .await?;
    Ok(())
}

pub(crate) async fn token_customer(
    pool: &Db,
    token_hash: &str,
    purpose: &str,
) -> Result<Option<String>, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT customer_id FROM site_customer_tokens \
         WHERE token_hash = ? AND purpose = ? AND used_at IS NULL AND expires_at > NOW(6)",
    )
    .bind(token_hash)
    .bind(purpose)
    .fetch_optional(pool)
    .await
}

pub(crate) async fn redeem_token(
    pool: &Db,
    token_hash: &str,
    purpose: &str,
) -> Result<Option<String>, sqlx::Error> {
    let mut tx = pool.begin().await?;
    let customer: Option<String> = sqlx::query_scalar(
        "SELECT customer_id FROM site_customer_tokens \
         WHERE token_hash = ? AND purpose = ? AND used_at IS NULL AND expires_at > NOW(6) FOR UPDATE",
    )
    .bind(token_hash)
    .bind(purpose)
    .fetch_optional(&mut *tx)
    .await?;
    if customer.is_some() {
        sqlx::query("UPDATE site_customer_tokens SET used_at = NOW(6) WHERE token_hash = ?")
            .bind(token_hash)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    Ok(customer)
}

pub(crate) async fn mark_verified(pool: &Db, customer_id: &str) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE site_customers SET email_verified_at = COALESCE(email_verified_at, NOW(6)), updated_at = NOW(6) WHERE id = ?",
    )
    .bind(customer_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub(crate) async fn replace_password(
    tx: &mut Tx,
    customer_id: &str,
    password_hash: String,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE site_customers SET password_hash = ?, updated_at = NOW(6) WHERE id = ?")
        .bind(password_hash)
        .bind(customer_id)
        .execute(&mut **tx)
        .await?;
    sqlx::query(
        "UPDATE site_customer_sessions SET revoked_at = NOW(6) WHERE customer_id = ? AND revoked_at IS NULL",
    )
    .bind(customer_id)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

pub(crate) async fn rename(tx: &mut Tx, customer_id: &str, name: &str) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE site_customers SET name = ?, updated_at = NOW(6) WHERE id = ?")
        .bind(name)
        .bind(customer_id)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

pub(crate) async fn close(
    tx: &mut Tx,
    customer_id: &str,
    closed_email: &str,
) -> Result<Option<(String, String)>, sqlx::Error> {
    let Some(row) = sqlx::query(
        "SELECT email, name FROM site_customers WHERE id = ? AND deleted_at IS NULL FOR UPDATE",
    )
    .bind(customer_id)
    .fetch_optional(&mut **tx)
    .await?
    else {
        return Ok(None);
    };
    let previous = (row.try_get("email")?, row.try_get("name")?);
    sqlx::query(
        "UPDATE site_customers SET email = ?, name = '', password_hash = NULL, deleted_at = NOW(6), updated_at = NOW(6) WHERE id = ?",
    )
    .bind(closed_email)
    .bind(customer_id)
    .execute(&mut **tx)
    .await?;
    sqlx::query("UPDATE site_customer_sessions SET revoked_at = NOW(6) WHERE customer_id = ? AND revoked_at IS NULL")
        .bind(customer_id)
        .execute(&mut **tx)
        .await?;
    sqlx::query("DELETE FROM site_customer_tokens WHERE customer_id = ?")
        .bind(customer_id)
        .execute(&mut **tx)
        .await?;
    Ok(Some(previous))
}

pub(crate) async fn verified_customer(
    pool: &Db,
    customer_id: &str,
) -> Result<Option<(String, String, Option<String>)>, sqlx::Error> {
    sqlx::query_as(
        "SELECT email, name, person_id FROM site_customers WHERE id = ? AND deleted_at IS NULL \
         AND email_verified_at IS NOT NULL",
    )
    .bind(customer_id)
    .fetch_optional(pool)
    .await
}

pub(crate) async fn link_person(
    pool: &Db,
    customer_id: &str,
    person_id: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE site_customers SET person_id = ? WHERE id = ? AND person_id IS NULL")
        .bind(person_id)
        .bind(customer_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub(crate) async fn link_person_in(
    tx: &mut Tx,
    customer_id: &str,
    person_id: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE site_customers SET person_id = ? WHERE id = ? AND person_id IS NULL")
        .bind(person_id)
        .bind(customer_id)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

pub(crate) async fn move_person(tx: &mut Tx, from: &str, to: &str) -> Result<u64, sqlx::Error> {
    Ok(
        sqlx::query("UPDATE site_customers SET person_id = ? WHERE person_id = ?")
            .bind(to)
            .bind(from)
            .execute(&mut **tx)
            .await?
            .rows_affected(),
    )
}

pub(crate) async fn forget_person(pool: &Db, person_id: &str) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE site_customers SET person_id = NULL WHERE person_id = ?")
        .bind(person_id)
        .execute(pool)
        .await?;
    Ok(())
}
