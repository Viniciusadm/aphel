use std::sync::Arc;

use aphel_contract_crm::CrmRecordTypes;
use aphel_kernel::modules::AppBuilder;
use aphel_sdk::AppError;
use aphel_sdk::platform::database::transaction::Tx;
use aphel_sdk::reference_cleanup::ReferenceCleanup;
use aphel_sdk::shared::RecordType;
use async_trait::async_trait;
use uuid::Uuid;

struct Cleanup;

async fn execute(tx: &mut Tx, sql: &'static str, id: &str, binds: usize) -> Result<(), AppError> {
    let mut query = sqlx::query(sql);
    for _ in 0..binds {
        query = query.bind(id.to_owned());
    }
    query.execute(&mut **tx).await?;
    Ok(())
}

#[async_trait]
impl ReferenceCleanup for Cleanup {
    fn kinds(&self) -> &'static [&'static str] {
        &[
            aphel_contract_crm::keys::PIPELINE,
            aphel_contract_crm::keys::PIPELINE_STAGE,
        ]
    }

    async fn removed(&self, tx: &mut Tx, kind: &str, id: Uuid) -> Result<(), AppError> {
        let id = id.to_string();
        match kind {
            aphel_contract_crm::keys::PIPELINE => {
                execute(tx, "DELETE FROM pipeline_renewal_settings WHERE pipeline_id = ? OR target_pipeline_id = ?", &id, 2).await?;
            }
            aphel_contract_crm::keys::PIPELINE_STAGE => {
                execute(
                    tx,
                    "DELETE FROM pipeline_renewal_settings WHERE target_stage_id = ?",
                    &id,
                    1,
                )
                .await?;
            }
            _ => {}
        }
        Ok(())
    }
}

async fn purged(tx: &mut Tx, record_type: &RecordType, id: Uuid) -> Result<(), AppError> {
    let id = id.to_string();
    if *record_type == RecordType::DEAL {
        execute(
            tx,
            "DELETE FROM deal_renewals WHERE original_deal_id = ?",
            &id,
            1,
        )
        .await?;
        execute(
            tx,
            "UPDATE deal_renewals SET renewal_deal_id = NULL WHERE renewal_deal_id = ?",
            &id,
            1,
        )
        .await?;
    }
    Ok(())
}

pub(crate) fn register(app: &mut AppBuilder) {
    app.contribute::<dyn ReferenceCleanup>(Arc::new(Cleanup));
    app.on(aphel_kernel::records::RecordPurged, 10, |ctx, args| {
        Box::pin(async move { purged(ctx.tx, &args.record_type, args.record.record_id).await })
    });
}
