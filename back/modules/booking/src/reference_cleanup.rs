use std::sync::Arc;

use aphel_kernel::modules::AppBuilder;
use aphel_sdk::AppError;
use aphel_sdk::platform::database::transaction::MySqlTx;
use aphel_sdk::reference_cleanup::ReferenceCleanup;
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
        &["assignment.rule"]
    }

    async fn removed(&self, tx: &mut MySqlTx, kind: &str, id: Uuid) -> Result<(), AppError> {
        let id = id.to_string();
        if kind == "assignment.rule" {
            execute(tx, "UPDATE meeting_types SET distribution_rule_id = NULL WHERE distribution_rule_id = ?", &id, 1).await?;
        }
        Ok(())
    }
}

pub(crate) fn register(app: &mut AppBuilder) {
    app.contribute::<dyn ReferenceCleanup>(Arc::new(Cleanup));
}
