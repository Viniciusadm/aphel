use chrono::{DateTime, NaiveDateTime, Utc};
use sqlx::mysql::MySqlRow;
use sqlx::types::Json;
use sqlx::{MySqlPool, Row};
use uuid::Uuid;

use crate::outgoing::{DisabledReason, EventSubscription, WebhookEndpoint};
use aphel_kernel::Ctx;
use aphel_kernel::database::transaction::MySqlTx;
use aphel_kernel::domain::UserId;
use aphel_kernel::domain::access::Permission;
use aphel_kernel::errors::AppError;
use aphel_kernel::infrastructure::repositories::mapper;

const ENDPOINT_COLUMNS: &str = "e.id, e.url, e.description, e.events, e.is_active, \
     e.disabled_reason, e.disabled_at, e.consecutive_failures, e.last_delivery_at, e.last_success_at, \
     e.created_by, u.display_name AS created_by_name, e.created_at, e.updated_at";

#[derive(Clone)]
pub(crate) struct WebhookEndpointStore {
    pool: MySqlPool,
}

fn endpoint_from_row(row: &MySqlRow) -> Result<WebhookEndpoint, AppError> {
    let id: String = row.try_get("id")?;
    let Json(events): Json<Vec<String>> = row.try_get("events")?;
    let optional = |column: &str| -> Result<Option<DateTime<Utc>>, AppError> {
        Ok(row
            .try_get::<Option<NaiveDateTime>, _>(column)?
            .map(mapper::utc))
    };
    let disabled_reason = row
        .try_get::<Option<String>, _>("disabled_reason")?
        .map(|value| {
            DisabledReason::parse(&value)
                .ok_or_else(|| mapper::invalid_column("webhook_endpoints.disabled_reason", &value))
        })
        .transpose()?;
    let created_by = row
        .try_get::<Option<String>, _>("created_by")?
        .map(|value| mapper::typed_id(value, UserId::new))
        .transpose()?;
    Ok(WebhookEndpoint {
        id: Uuid::parse_str(&id)
            .map_err(|_| mapper::invalid_column("webhook_endpoints.id", &id))?,
        url: row.try_get("url")?,
        description: row.try_get("description")?,
        events: EventSubscription::from_strings(events),
        is_active: row.try_get("is_active")?,
        disabled_reason,
        disabled_at: optional("disabled_at")?,
        consecutive_failures: row.try_get("consecutive_failures")?,
        last_delivery_at: optional("last_delivery_at")?,
        last_success_at: optional("last_success_at")?,
        created_by,
        created_by_name: row.try_get("created_by_name")?,
        created_at: mapper::utc(row.try_get("created_at")?),
        updated_at: mapper::utc(row.try_get("updated_at")?),
    })
}

impl WebhookEndpointStore {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    pub(crate) fn of(ctx: &Ctx<'_>) -> Self {
        Self::new(ctx.pool().clone())
    }

    async fn select(
        &self,
        condition: &str,
        id: Option<Uuid>,
    ) -> Result<Vec<WebhookEndpoint>, AppError> {
        let sql = format!(
            "SELECT {ENDPOINT_COLUMNS} FROM webhook_endpoints e LEFT JOIN users u ON u.id = e.created_by \
             {condition} ORDER BY e.created_at DESC, e.id DESC"
        );
        let mut query = sqlx::query(sqlx::AssertSqlSafe(sql));
        if let Some(id) = id {
            query = query.bind(id.to_string());
        }
        query
            .fetch_all(&self.pool)
            .await?
            .iter()
            .map(endpoint_from_row)
            .collect()
    }

    pub(crate) async fn insert(
        &self,
        tx: &mut MySqlTx,
        endpoint: &WebhookEndpoint,
    ) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO webhook_endpoints (id, url, description, events, is_active, \
             disabled_reason, disabled_at, consecutive_failures, created_by, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(endpoint.id.to_string())
        .bind(&endpoint.url)
        .bind(&endpoint.description)
        .bind(Json(endpoint.events.to_strings()))
        .bind(endpoint.is_active)
        .bind(endpoint.disabled_reason.map(|reason| reason.as_str()))
        .bind(endpoint.disabled_at.map(mapper::naive))
        .bind(endpoint.consecutive_failures)
        .bind(endpoint.created_by.as_ref().map(UserId::as_str))
        .bind(mapper::naive(endpoint.created_at))
        .bind(mapper::naive(endpoint.updated_at))
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub(crate) async fn update(
        &self,
        tx: &mut MySqlTx,
        endpoint: &WebhookEndpoint,
    ) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE webhook_endpoints SET url = ?, description = ?, events = ?, is_active = ?, \
             disabled_reason = ?, disabled_at = ?, consecutive_failures = ?, updated_at = ? \
             WHERE id = ?",
        )
        .bind(&endpoint.url)
        .bind(&endpoint.description)
        .bind(Json(endpoint.events.to_strings()))
        .bind(endpoint.is_active)
        .bind(endpoint.disabled_reason.map(|reason| reason.as_str()))
        .bind(endpoint.disabled_at.map(mapper::naive))
        .bind(endpoint.consecutive_failures)
        .bind(mapper::naive(endpoint.updated_at))
        .bind(endpoint.id.to_string())
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub(crate) async fn delete(&self, tx: &mut MySqlTx, id: Uuid) -> Result<(), AppError> {
        sqlx::query("DELETE FROM webhook_endpoints WHERE id = ?")
            .bind(id.to_string())
            .execute(&mut **tx)
            .await?;
        Ok(())
    }

    pub(crate) async fn list(&self) -> Result<Vec<WebhookEndpoint>, AppError> {
        self.select("", None).await
    }

    pub(crate) async fn find(&self, id: Uuid) -> Result<Option<WebhookEndpoint>, AppError> {
        Ok(self
            .select("WHERE e.id = ?", Some(id))
            .await?
            .into_iter()
            .next())
    }

    pub(crate) async fn count(&self) -> Result<u64, AppError> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM webhook_endpoints")
            .fetch_one(&self.pool)
            .await?;
        Ok(count.max(0) as u64)
    }

    pub(crate) async fn active(&self) -> Result<Vec<WebhookEndpoint>, AppError> {
        self.select("WHERE e.is_active = TRUE", None).await
    }

    pub(crate) async fn record_success(
        &self,
        tx: &mut MySqlTx,
        id: Uuid,
        at: DateTime<Utc>,
    ) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE webhook_endpoints SET consecutive_failures = 0, last_delivery_at = ?, last_success_at = ? \
             WHERE id = ?",
        )
        .bind(mapper::naive(at))
        .bind(mapper::naive(at))
        .bind(id.to_string())
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub(crate) async fn record_failure(
        &self,
        tx: &mut MySqlTx,
        id: Uuid,
        at: DateTime<Utc>,
        counts: bool,
    ) -> Result<u32, AppError> {
        sqlx::query(
            "UPDATE webhook_endpoints SET consecutive_failures = consecutive_failures + ?, last_delivery_at = ? \
             WHERE id = ?",
        )
        .bind(u32::from(counts))
        .bind(mapper::naive(at))
        .bind(id.to_string())
        .execute(&mut **tx)
        .await?;
        let failures: Option<u32> =
            sqlx::query_scalar("SELECT consecutive_failures FROM webhook_endpoints WHERE id = ?")
                .bind(id.to_string())
                .fetch_optional(&mut **tx)
                .await?;
        Ok(failures.unwrap_or(0))
    }

    pub(crate) async fn disable(
        &self,
        tx: &mut MySqlTx,
        id: Uuid,
        reason: DisabledReason,
        at: DateTime<Utc>,
    ) -> Result<bool, AppError> {
        let result = sqlx::query(
            "UPDATE webhook_endpoints SET is_active = FALSE, disabled_reason = ?, disabled_at = ?, updated_at = ? \
             WHERE id = ? AND is_active = TRUE",
        )
        .bind(reason.as_str())
        .bind(mapper::naive(at))
        .bind(mapper::naive(at))
        .bind(id.to_string())
        .execute(&mut **tx)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    pub(crate) async fn settings_managers(&self) -> Result<Vec<UserId>, AppError> {
        let ids: Vec<String> = sqlx::query_scalar(
            "SELECT DISTINCT u.id FROM users u \
             JOIN roles r ON r.id = u.role_id \
             LEFT JOIN role_permissions rp ON rp.role_id = r.id AND rp.permission = ? \
             WHERE u.status = 'active' \
             AND ((r.is_system = TRUE AND r.`key` = 'admin') OR rp.role_id IS NOT NULL)",
        )
        .bind(Permission::SETTINGS_MANAGE.as_str())
        .fetch_all(&self.pool)
        .await?;
        ids.into_iter()
            .map(|id| mapper::typed_id(id, UserId::new))
            .collect()
    }
}
