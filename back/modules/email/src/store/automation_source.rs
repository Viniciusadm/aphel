use aphel_kernel::domain::UserId;
use aphel_sdk::platform::database::Db;
use aphel_sdk::platform::database::transaction::Tx;
use aphel_sdk::{AppError, email::EmailAutomationReader};
use sqlx::{MySql, QueryBuilder, Row};
use std::collections::HashMap;
use uuid::Uuid;

pub(crate) struct Reader;

fn uuid(value: String, column: &str) -> Result<Uuid, AppError> {
    aphel_sdk::platform::infrastructure::repositories::mapper::parse_uuid(&value, column)
}
fn opt_uuid(value: Option<String>, column: &str) -> Result<Option<Uuid>, AppError> {
    value.map(|value| uuid(value, column)).transpose()
}

#[async_trait::async_trait]
impl EmailAutomationReader for Reader {
    async fn organization_template_keys(
        &self,
        pool: &Db,
    ) -> Result<HashMap<Uuid, String>, AppError> {
        let rows =
            sqlx::query("SELECT id, `key` FROM email_templates WHERE scope = 'organization'")
                .fetch_all(pool)
                .await?;
        let mut keys = HashMap::new();
        for row in rows {
            keys.insert(uuid(row.try_get("id")?, "id")?, row.try_get("key")?);
        }
        Ok(keys)
    }

    async fn organization_template_ids_in(
        &self,
        tx: &mut Tx,
    ) -> Result<HashMap<String, Uuid>, AppError> {
        let rows =
            sqlx::query("SELECT id, `key` FROM email_templates WHERE scope = 'organization'")
                .fetch_all(&mut **tx)
                .await?;
        let mut ids = HashMap::new();
        for row in rows {
            ids.insert(row.try_get("key")?, uuid(row.try_get("id")?, "id")?);
        }
        Ok(ids)
    }

    async fn objection_hash(
        &self,
        pool: &Db,
        objection_id: Uuid,
    ) -> Result<Option<String>, AppError> {
        Ok(sqlx::query_scalar(
            "SELECT address_hash FROM email_tracking_objections \
             WHERE id = ?",
        )
        .bind(objection_id.to_string())
        .fetch_optional(pool)
        .await?)
    }

    async fn has_objection(&self, pool: &Db, address_hash: &str) -> Result<bool, AppError> {
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM email_tracking_objections \
             WHERE address_hash = ?)",
        )
        .bind(address_hash)
        .fetch_one(pool)
        .await?;
        Ok(exists)
    }

    async fn sendable_mailbox(
        &self,
        pool: &Db,
        owner: &UserId,
        mailbox_id: Option<Uuid>,
    ) -> Result<Option<Uuid>, AppError> {
        let mut builder = QueryBuilder::<MySql>::new("SELECT id FROM mailboxes WHERE user_id = ");
        builder
            .push_bind(owner.as_str().to_owned())
            .push(" AND status <> 'disconnected'");
        if let Some(mailbox_id) = mailbox_id {
            builder.push(" AND id = ").push_bind(mailbox_id.to_string());
        }
        builder.push(" ORDER BY (status = 'active') DESC, created_at LIMIT 1");
        let id: Option<String> = builder.build_query_scalar().fetch_optional(pool).await?;
        opt_uuid(id, "id")
    }
}
