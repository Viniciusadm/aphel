use aphel_contract_crm::CrmRecordTypes;
use aphel_kernel::modules::AppBuilder;
use aphel_sdk::AppError;
use aphel_sdk::platform::database::transaction::MySqlTx;
use aphel_sdk::shared::RecordType;
use uuid::Uuid;

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

async fn purged(tx: &mut MySqlTx, record_type: &RecordType, id: Uuid) -> Result<(), AppError> {
    let id = id.to_string();
    if *record_type == RecordType::DEAL {
        execute(tx, "DELETE FROM quotes WHERE deal_id = ?", &id, 1).await?;
    }
    if *record_type == RecordType::CONTACT {
        execute(
            tx,
            "UPDATE quotes SET contact_id = NULL WHERE contact_id = ?",
            &id,
            1,
        )
        .await?;
    }
    if *record_type == RecordType::COMPANY {
        execute(
            tx,
            "UPDATE quotes SET company_id = NULL WHERE company_id = ?",
            &id,
            1,
        )
        .await?;
    }
    if *record_type == RecordType::PRODUCT {
        execute(
            tx,
            "UPDATE quote_items SET product_id = NULL WHERE product_id = ?",
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
