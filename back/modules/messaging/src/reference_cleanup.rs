use aphel_contract_crm::CrmRecordTypes;
use aphel_kernel::modules::AppBuilder;
use aphel_sdk::AppError;
use aphel_sdk::platform::database::transaction::Tx;
use aphel_sdk::shared::RecordType;
use uuid::Uuid;

async fn execute(tx: &mut Tx, sql: &'static str, id: &str, binds: usize) -> Result<(), AppError> {
    let mut query = sqlx::query(sql);
    for _ in 0..binds {
        query = query.bind(id.to_owned());
    }
    query.execute(&mut **tx).await?;
    Ok(())
}

async fn purged(tx: &mut Tx, record_type: &RecordType, id: Uuid) -> Result<(), AppError> {
    let id = id.to_string();
    if *record_type == RecordType::CONTACT {
        execute(
            tx,
            "UPDATE messaging_conversations SET contact_id = NULL WHERE contact_id = ?",
            &id,
            1,
        )
        .await?;
    }
    if *record_type == RecordType::DEAL {
        execute(
            tx,
            "UPDATE messaging_conversations SET deal_id = NULL WHERE deal_id = ?",
            &id,
            1,
        )
        .await?;
    }
    Ok(())
}

pub(crate) fn register(app: &mut AppBuilder) {
    app.on(aphel_kernel::records::RecordPurged, 10, |ctx, args| {
        Box::pin(async move { purged(ctx.tx, &args.record_type, args.record.record_id).await })
    });
}
