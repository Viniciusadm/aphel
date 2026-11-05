use aphel_kernel::database::transaction::MySqlTx;
use aphel_sdk::{AppError, Ctx};
use std::collections::BTreeMap;
use uuid::Uuid;
pub(crate) struct References;
#[async_trait::async_trait]
impl aphel_sdk::merges::MergeReferences for References {
    async fn move_references(
        &self,
        _ctx: &Ctx<'_>,
        tx: &mut MySqlTx,
        record_type: &str,
        from: Uuid,
        to: Uuid,
    ) -> Result<BTreeMap<String, u64>, AppError> {
        let mut moved = BTreeMap::new();
        let tables = &["deals", "deal_contacts"];
        let present = aphel_kernel::records::existing_tables(tx, tables).await?;
        if present.is_empty() {
            return Ok(moved);
        }
        if present.len() != tables.len() {
            return Err(AppError::Internal(
                "partial merge reference owner schema".into(),
            ));
        }
        if record_type == aphel_sdk::people::RECORD_TYPE {
            let count = sqlx::query("UPDATE deals SET contact_id = ? WHERE contact_id = ?")
                .bind(to.to_string())
                .bind(from.to_string())
                .execute(&mut **tx)
                .await?
                .rows_affected();
            *moved.entry("deals".to_owned()).or_default() += count;
        }
        if record_type == aphel_sdk::people::RECORD_TYPE {
            let count =
                sqlx::query("UPDATE IGNORE deal_contacts SET contact_id = ? WHERE contact_id = ?")
                    .bind(to.to_string())
                    .bind(from.to_string())
                    .execute(&mut **tx)
                    .await?
                    .rows_affected();
            *moved.entry("deals".to_owned()).or_default() += count;
        }
        if record_type == aphel_sdk::people::RECORD_TYPE {
            let count = sqlx::query("DELETE FROM deal_contacts WHERE contact_id = ?")
                .bind(from.to_string())
                .execute(&mut **tx)
                .await?
                .rows_affected();
            let _ = count;
        }
        if record_type == aphel_sdk::organizations::RECORD_TYPE {
            let count = sqlx::query("UPDATE deals SET company_id = ? WHERE company_id = ?")
                .bind(to.to_string())
                .bind(from.to_string())
                .execute(&mut **tx)
                .await?
                .rows_affected();
            *moved.entry("deals".to_owned()).or_default() += count;
        }
        Ok(moved)
    }
}
