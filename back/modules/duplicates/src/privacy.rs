use aphel_kernel::{
    Ctx, application::fields::LocalizedLabels, database::transaction::MySqlTx,
    domain::access::Permission, errors::AppError,
};
use aphel_sdk::platform::application::access::Authorizer;
use aphel_sdk::privacy::{PrivacySource, SourceOutcome, SourceRequest};
pub(crate) struct DuplicatePrivacy;
#[async_trait::async_trait]
impl PrivacySource for DuplicatePrivacy {
    fn key(&self) -> &'static str {
        "duplicates"
    }
    fn labels(&self) -> LocalizedLabels {
        LocalizedLabels(std::collections::BTreeMap::from([
            ("en".into(), "Duplicate matches".into()),
            ("pt-BR".into(), "Correspondências de duplicados".into()),
        ]))
    }
    async fn available(&self, ctx: &Ctx<'_>) -> Result<bool, AppError> {
        aphel_sdk::privacy::sql::table_exists(ctx, "duplicate_candidates").await
    }
    async fn erase(
        &self,
        ctx: &Ctx<'_>,
        tx: &mut MySqlTx,
        request: &SourceRequest<'_>,
    ) -> Result<SourceOutcome, AppError> {
        ctx.scope.require(&Permission::PRIVACY_MANAGE)?;
        let mut affected = 0;
        for record in request.records {
            affected+=sqlx::query("DELETE FROM duplicate_candidates WHERE record_type = ? AND (record_id = ? OR duplicate_id = ?)").bind(record.record_type.as_str()).bind(record.record_id.to_string()).bind(record.record_id.to_string()).execute(&mut **tx).await?.rows_affected();
        }
        Ok(SourceOutcome {
            affected,
            storage_keys: Vec::new(),
        })
    }
}
