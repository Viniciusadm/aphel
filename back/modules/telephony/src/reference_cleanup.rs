use std::sync::Arc;

use aphel_contract_crm::CrmRecordTypes;
use aphel_kernel::database::transaction::MySqlTx;
use aphel_kernel::modules::AppBuilder;
use aphel_sdk::AppError;
use aphel_sdk::reference_cleanup::ReferenceCleanup;
use aphel_sdk::shared::RecordType;
use async_trait::async_trait;
use uuid::Uuid;

struct Cleanup;

async fn execute(
    tx: &mut MySqlTx,
    sql: &'static str,
    id: &str,
    binds: usize,
) -> Result<(), AppError> {
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
        &["saved_views.view"]
    }

    async fn removed(&self, tx: &mut MySqlTx, kind: &str, id: Uuid) -> Result<(), AppError> {
        let id = id.to_string();
        if kind == "saved_views.view" {
            execute(
                tx,
                "UPDATE call_sessions SET view_id = NULL WHERE view_id = ?",
                &id,
                1,
            )
            .await?;
        }
        Ok(())
    }
}

async fn purged(tx: &mut MySqlTx, record_type: &RecordType, id: Uuid) -> Result<(), AppError> {
    let id = id.to_string();
    if *record_type == RecordType::ACTIVITY {
        execute(
            tx,
            "DELETE FROM activity_calls WHERE activity_id = ?",
            &id,
            1,
        )
        .await?;
        execute(
            tx,
            "UPDATE call_session_items SET activity_id = NULL WHERE activity_id = ?",
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
