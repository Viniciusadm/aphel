use aphel_sdk::{
    AppError, Ctx,
    tasks::{CreateActivityCommand, TaskWriter},
};
use uuid::Uuid;

pub(crate) struct Writer;

#[async_trait::async_trait]
impl TaskWriter for Writer {
    async fn create(
        &self,
        ctx: &Ctx<'_>,
        command: CreateActivityCommand,
    ) -> Result<Uuid, AppError> {
        Ok(crate::service::create(ctx, command).await?.activity.id)
    }
}

#[async_trait::async_trait]
impl aphel_sdk::tasks::TaskTypes for Writer {
    fn require_reader(&self, scope: &aphel_sdk::access::AccessScope) -> Result<(), AppError> {
        crate::types::require_activity_type_reader(scope)
    }

    async fn list(
        &self,
        ctx: &Ctx<'_>,
        include_inactive: bool,
        window: aphel_kernel::application::organization_admin::ListWindow,
    ) -> Result<
        aphel_kernel::application::organization_admin::Listing<aphel_sdk::tasks::ActivityTypeView>,
        AppError,
    > {
        crate::types::list(ctx, include_inactive, window).await
    }
}

fn instant_text(value: chrono::DateTime<chrono::Utc>) -> String {
    value.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

#[async_trait::async_trait]
impl aphel_sdk::tasks::MeetingScheduler for Writer {
    async fn schedule(
        &self,
        ctx: &Ctx<'_>,
        tx: &mut aphel_kernel::database::transaction::MySqlTx,
        meeting: aphel_sdk::tasks::MeetingActivity,
    ) -> Result<Uuid, AppError> {
        let mut prepared = crate::service::prepare_create(
            ctx,
            CreateActivityCommand {
                title: meeting.title,
                activity_type_id: meeting.activity_type_id.to_string(),
                due_at: Some(instant_text(meeting.starts_at)),
                owner_id: Some(meeting.owner_id.as_str().to_owned()),
                duration_minutes: Some(i64::from(meeting.duration_minutes)),
                description: meeting.description,
                ip: meeting.ip,
                ..CreateActivityCommand::default()
            },
            crate::domain::ActivityScheduleInput::default(),
        )
        .await?;
        prepared.data.record = Some(meeting.record);
        crate::service::insert_prepared(ctx, tx, &prepared).await?;
        Ok(prepared.id)
    }

    async fn move_to(
        &self,
        ctx: &Ctx<'_>,
        tx: &mut aphel_kernel::database::transaction::MySqlTx,
        scope: &aphel_sdk::access::AccessScope,
        activity_id: Uuid,
        change: aphel_sdk::tasks::MeetingActivityMove,
    ) -> Result<Option<aphel_sdk::records::PendingCommit>, AppError> {
        crate::service::update_in_tx(
            &ctx.with_scope(scope),
            tx,
            &activity_id.to_string(),
            crate::domain::UpdateActivityCommand {
                due_at: Some(Some(instant_text(change.starts_at))),
                duration_minutes: Some(Some(i64::from(change.duration_minutes))),
                ip: change.ip,
                ..crate::domain::UpdateActivityCommand::default()
            },
        )
        .await
    }

    async fn cancel(
        &self,
        ctx: &Ctx<'_>,
        tx: &mut aphel_kernel::database::transaction::MySqlTx,
        scope: &aphel_sdk::access::AccessScope,
        activity_id: Uuid,
        ip: Option<String>,
    ) -> Result<Option<aphel_sdk::records::PendingCommit>, AppError> {
        crate::service::update_in_tx(
            &ctx.with_scope(scope),
            tx,
            &activity_id.to_string(),
            crate::domain::UpdateActivityCommand {
                status: Some("canceled".to_owned()),
                ip,
                ..crate::domain::UpdateActivityCommand::default()
            },
        )
        .await
    }

    async fn busy(
        &self,
        ctx: &Ctx<'_>,
        owner: &aphel_kernel::domain::UserId,
        from: chrono::DateTime<chrono::Utc>,
        to: chrono::DateTime<chrono::Utc>,
        exclude: Option<Uuid>,
    ) -> Result<Vec<aphel_sdk::calendar::BusyInterval>, AppError> {
        crate::store::ActivityStore::of(ctx)
            .busy_intervals(owner, from, to, exclude)
            .await
    }
}
