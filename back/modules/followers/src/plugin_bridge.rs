use std::sync::Arc;

use serde::Deserialize;
use serde_json::Value;

use crate::http::RecordFollowersResponse;
use crate::service;
use aphel_kernel::Ctx;
use aphel_kernel::errors::AppError;
use aphel_kernel::modules::{OperationFuture, PluginOperation, operation_input, operation_output};

#[derive(Deserialize)]
struct RecordInput {
    record_type: String,
    record_id: String,
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
        let input: RecordInput = operation_input(input)?;
        let followers = service::followers(ctx, &input.record_type, &input.record_id).await?;
        operation_output(RecordFollowersResponse::from(followers))
    })
}

fn follow<'a>(ctx: &'a Ctx<'_>, input: Value) -> OperationFuture<'a> {
    Box::pin(async move {
        require_user_actor(ctx)?;
        let input: RecordInput = operation_input(input)?;
        let followers = service::follow(ctx, &input.record_type, &input.record_id).await?;
        operation_output(RecordFollowersResponse::from(followers))
    })
}

fn unfollow<'a>(ctx: &'a Ctx<'_>, input: Value) -> OperationFuture<'a> {
    Box::pin(async move {
        require_user_actor(ctx)?;
        let input: RecordInput = operation_input(input)?;
        let followers = service::unfollow(ctx, &input.record_type, &input.record_id).await?;
        operation_output(RecordFollowersResponse::from(followers))
    })
}

pub(crate) fn operations() -> Vec<Arc<PluginOperation>> {
    vec![
        Arc::new(PluginOperation::read(
            "followers.list",
            "Followers of a record the actor can read: { record_type, record_id } -> { record_type, record_id, following, total, followers }",
            list,
        )),
        Arc::new(PluginOperation::write(
            "followers.follow",
            "Make the acting user follow a record (needs a user actor): { record_type, record_id }",
            follow,
        )
        .with_durable_mutations()),
        Arc::new(PluginOperation::write(
            "followers.unfollow",
            "Make the acting user stop following a record (needs a user actor): { record_type, record_id }",
            unfollow,
        )
        .with_durable_mutations()),
    ]
}
