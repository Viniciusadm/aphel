use aphel_kernel::modules::OperationSpec;
use aphel_sdk::{OperationKey, operation};

use serde::Deserialize;
use serde_json::Value;

use crate::http::RecordFollowersResponse;
use crate::service;
use aphel_kernel::Ctx;
use aphel_kernel::errors::AppError;
use aphel_kernel::modules::{OperationFuture, operation_input, operation_output};

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

const OP_FOLLOWERS_LIST: OperationKey<Value, Value> = operation!("followers.list");
const OP_FOLLOWERS_FOLLOW: OperationKey<Value, Value> = operation!("followers.follow");
const OP_FOLLOWERS_UNFOLLOW: OperationKey<Value, Value> = operation!("followers.unfollow");

pub(crate) fn register(app: &mut aphel_kernel::modules::AppBuilder) {
    app.operation(OP_FOLLOWERS_LIST, OperationSpec::read("Followers of a record the actor can read: { record_type, record_id } -> { record_type, record_id, following, total, followers }"), list);
    app.operation(
        OP_FOLLOWERS_FOLLOW,
        OperationSpec::write(
            "Make the acting user follow a record (needs a user actor): { record_type, record_id }",
        )
        .durable(),
        follow,
    );
    app.operation(OP_FOLLOWERS_UNFOLLOW, OperationSpec::write("Make the acting user stop following a record (needs a user actor): { record_type, record_id }").durable(), unfollow);
}
