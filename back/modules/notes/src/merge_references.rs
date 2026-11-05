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
        let tables = &["notes"];
        let present = aphel_sdk::records::existing_tables(tx, tables).await?;
        if present.is_empty() {
            return Ok(moved);
        }
        if present.len() != tables.len() {
            return Err(AppError::Internal(
                "partial merge reference owner schema".into(),
            ));
        }
        let count =
            sqlx::query("UPDATE notes SET record_id = ? WHERE record_type = ? AND record_id = ?")
                .bind(to.to_string())
                .bind(record_type)
                .bind(from.to_string())
                .execute(&mut **tx)
                .await?
                .rows_affected();
        *moved.entry("notes".to_owned()).or_default() += count;
        Ok(moved)
    }
}
