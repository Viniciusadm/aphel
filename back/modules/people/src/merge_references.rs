use aphel_sdk::platform::database::transaction::Tx;
use aphel_sdk::{AppError, Ctx};
use std::collections::BTreeMap;
use uuid::Uuid;
pub(crate) struct References;
#[async_trait::async_trait]
impl aphel_sdk::merges::MergeReferences for References {
    async fn move_references(
        &self,
        _ctx: &Ctx<'_>,
        tx: &mut Tx,
        record_type: &str,
        from: Uuid,
        to: Uuid,
    ) -> Result<BTreeMap<String, u64>, AppError> {
        let mut moved = BTreeMap::new();
        let tables = &["people", "people_organizations"];
        let present = aphel_sdk::records::existing_tables(tx, tables).await?;
        if present.is_empty() {
            return Ok(moved);
        }
        if present.len() != tables.len() {
            return Err(AppError::Internal(
                "partial merge reference owner schema".into(),
            ));
        }
        if record_type == aphel_sdk::people::RECORD_TYPE {
            let count = sqlx::query(
                "UPDATE IGNORE people_organizations SET contact_id = ? WHERE contact_id = ?",
            )
            .bind(to.to_string())
            .bind(from.to_string())
            .execute(&mut **tx)
            .await?
            .rows_affected();
            *moved.entry("organizations".to_owned()).or_default() += count;
        }
        if record_type == aphel_sdk::people::RECORD_TYPE {
            let count = sqlx::query("DELETE FROM people_organizations WHERE contact_id = ?")
                .bind(from.to_string())
                .execute(&mut **tx)
                .await?
                .rows_affected();
            let _ = count;
        }
        if record_type == aphel_sdk::people::RECORD_TYPE {
            let count = sqlx::query("DELETE l FROM people_organizations l JOIN people c ON c.id = l.contact_id WHERE l.contact_id = ? AND l.company_id = c.company_id").bind(to.to_string()).execute(&mut **tx).await?.rows_affected();
            let _ = count;
        }
        if record_type == aphel_sdk::organizations::RECORD_TYPE {
            let count = sqlx::query("UPDATE people SET company_id = ? WHERE company_id = ?")
                .bind(to.to_string())
                .bind(from.to_string())
                .execute(&mut **tx)
                .await?
                .rows_affected();
            *moved.entry("people".to_owned()).or_default() += count;
        }
        if record_type == aphel_sdk::organizations::RECORD_TYPE {
            let count = sqlx::query(
                "UPDATE IGNORE people_organizations SET company_id = ? WHERE company_id = ?",
            )
            .bind(to.to_string())
            .bind(from.to_string())
            .execute(&mut **tx)
            .await?
            .rows_affected();
            *moved.entry("people".to_owned()).or_default() += count;
        }
        if record_type == aphel_sdk::organizations::RECORD_TYPE {
            let count = sqlx::query("DELETE FROM people_organizations WHERE company_id = ?")
                .bind(from.to_string())
                .execute(&mut **tx)
                .await?
                .rows_affected();
            let _ = count;
        }
        if record_type == aphel_sdk::organizations::RECORD_TYPE {
            let count = sqlx::query("DELETE l FROM people_organizations l JOIN people c ON c.id = l.contact_id WHERE l.company_id = ? AND c.company_id = l.company_id").bind(to.to_string()).execute(&mut **tx).await?.rows_affected();
            let _ = count;
        }
        Ok(moved)
    }
}
