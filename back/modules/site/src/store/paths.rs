use aphel_sdk::platform::database::transaction::Tx;

pub(crate) async fn lock_claims(tx: &mut Tx) -> Result<(), sqlx::Error> {
    sqlx::query("SELECT id FROM site_path_lock WHERE id = 1 FOR UPDATE")
        .execute(&mut **tx)
        .await?;
    Ok(())
}
