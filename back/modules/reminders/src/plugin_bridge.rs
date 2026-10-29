use std::sync::Arc;

use serde::Deserialize;
use serde_json::{Value, json};

use crate::domain::ListRemindersQuery;
use crate::http::{CreateReminderRequest, ReminderListParams, ReminderResponse};
use crate::service;
use aphel_kernel::Ctx;
use aphel_kernel::errors::AppError;
use aphel_kernel::modules::{
    OperationFuture, OperationPage, PluginOperation, operation_input, operation_output,
};

#[derive(Deserialize)]
struct ListInput {
    #[serde(flatten)]
    params: ReminderListParams,
    #[serde(flatten)]
    page: OperationPage,
}

#[derive(Deserialize)]
struct ReminderId {
    id: String,
}

fn require_user_actor(ctx: &Ctx<'_>) -> Result<(), AppError> {
    if ctx.scope.plugin_key().is_some() {
        return Err(AppError::Validation(
            "this operation needs a user actor: call it from a hook or an authenticated route"
                .into(),
        ));
    }
    Ok(())
}

fn list<'a>(ctx: &'a Ctx<'_>, input: Value) -> OperationFuture<'a> {
    Box::pin(async move {
        require_user_actor(ctx)?;
        let input: ListInput = operation_input(input)?;
        let listing = service::list(
            ctx,
            ListRemindersQuery {
                status: input.params.status,
                record_type: input.params.record_type,
                record_id: input.params.record_id,
                window: input.page.window(),
            },
        )
        .await?;
        let reminders: Vec<ReminderResponse> = listing.items.into_iter().map(Into::into).collect();
        Ok(json!({ "data": operation_output(reminders)?, "total": listing.total }))
    })
}

fn create<'a>(ctx: &'a Ctx<'_>, input: Value) -> OperationFuture<'a> {
    Box::pin(async move {
        require_user_actor(ctx)?;
        let request: CreateReminderRequest = operation_input(input)?;
        operation_output(ReminderResponse::from(
            service::create(ctx, request.into()).await?,
        ))
    })
}

fn delete<'a>(ctx: &'a Ctx<'_>, input: Value) -> OperationFuture<'a> {
    Box::pin(async move {
        require_user_actor(ctx)?;
        let input: ReminderId = operation_input(input)?;
        service::delete(ctx, &input.id).await?;
        Ok(json!({ "deleted": true }))
    })
}

pub(crate) fn operations() -> Vec<Arc<PluginOperation>> {
    vec![
        Arc::new(PluginOperation::read(
            "reminders.list",
            "Reminders of the acting user sorted by remind_at (needs a user actor): { status?: pending|sent|dismissed, record_type?, record_id?, page?, per_page? } -> { data, total }",
            list,
        )),
        Arc::new(PluginOperation::write(
            "reminders.create",
            "Create a reminder for the acting user (needs a user actor): { remind_at, note?, record_type?, record_id? }",
            create,
        )),
        Arc::new(PluginOperation::write(
            "reminders.delete",
            "Delete a reminder of the acting user (needs a user actor): { id }",
            delete,
        )),
    ]
}
