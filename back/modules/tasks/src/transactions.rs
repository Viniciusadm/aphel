use aphel_kernel::{application::shared::RecordRef, database::transaction::Tx};
use aphel_sdk::{
    AppError, Ctx,
    tasks::{CreateActivityCommand, PreparedTask, TaskRecord, TaskTransactions},
};
use uuid::Uuid;
pub(crate) struct Transactions;
struct Draft(crate::service::PreparedActivity);
fn enabled(ctx: &Ctx<'_>) -> Result<(), AppError> {
    if ctx.app().is_enabled(crate::KEY) {
        Ok(())
    } else {
        Err(AppError::unavailable_code(
            "task_transactions_unavailable",
            "task transactions are not enabled",
        ))
    }
}
fn snapshot(record: crate::domain::ActivityRecord) -> TaskRecord {
    TaskRecord {
        id: record.activity.id,
        record: record
            .activity
            .record
            .as_ref()
            .map(|linked| linked.record.clone()),
        response: record.into(),
    }
}
#[async_trait::async_trait]
impl PreparedTask for Draft {
    fn id(&self) -> Uuid {
        self.0.id
    }
    fn record(&self) -> Option<RecordRef> {
        self.0.data.record.clone()
    }
    fn ip(&self) -> Option<String> {
        self.0.ip()
    }
    async fn insert(&self, ctx: &Ctx<'_>, tx: &mut Tx) -> Result<(), AppError> {
        enabled(ctx)?;
        crate::service::insert_prepared(ctx, tx, &self.0).await
    }
    async fn response(&self, ctx: &Ctx<'_>) -> Result<TaskRecord, AppError> {
        enabled(ctx)?;
        Ok(snapshot(crate::service::find_record(ctx, self.0.id).await?))
    }
}
#[async_trait::async_trait]
impl TaskTransactions for Transactions {
    async fn type_for_creation(
        &self,
        ctx: &Ctx<'_>,
        key: &str,
    ) -> Result<Option<aphel_sdk::tasks::ActivityTypeView>, AppError> {
        use aphel_contract_crm::CrmPermissions;
        use aphel_sdk::platform::application::access::Authorizer;
        ctx.scope
            .require(&aphel_sdk::platform::domain::access::Permission::TASKS_CREATE)?;
        Ok(crate::store::ActivityTypeStore::of(ctx)
            .list(false)
            .await?
            .into_iter()
            .find(|view| view.key == key && view.is_active))
    }

    async fn prepare_completed(
        &self,
        ctx: &Ctx<'_>,
        mut command: CreateActivityCommand,
    ) -> Result<Box<dyn PreparedTask>, AppError> {
        command.status = Some("done".to_owned());
        let prepared = crate::service::prepare_create(
            ctx,
            command,
            crate::domain::ActivityScheduleInput {
                reminder_minutes_before: Some(None),
                recurrence: None,
            },
        )
        .await?;
        Ok(Box::new(Draft(prepared)))
    }
    async fn get(&self, ctx: &Ctx<'_>, id: &str) -> Result<TaskRecord, AppError> {
        Ok(snapshot(crate::service::get(ctx, id).await?))
    }
}
