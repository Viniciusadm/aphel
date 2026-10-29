use std::sync::Arc;

use serde::Deserialize;
use serde_json::Value;
use uuid::Uuid;

use crate::errors::conversation_not_found;
use crate::inbox_routes::message_item;
use crate::model::ReplyRequest;
use crate::sending;
use crate::{REPLY, VIEW};
use aphel_kernel::Ctx;
use aphel_kernel::application::access::Authorizer;
use aphel_kernel::modules::{OperationFuture, PluginOperation, operation_input, operation_output};

#[derive(Deserialize)]
struct ReplyInput {
    conversation_id: String,
    #[serde(flatten)]
    request: ReplyRequest,
}

fn reply<'a>(ctx: &'a Ctx<'_>, input: Value) -> OperationFuture<'a> {
    Box::pin(async move {
        ctx.scope.require(&VIEW)?;
        ctx.scope.require(&REPLY)?;
        let input: ReplyInput = operation_input(input)?;
        let conversation_id =
            Uuid::parse_str(&input.conversation_id).map_err(|_| conversation_not_found())?;
        let (_, _, message) =
            sending::queue_reply(ctx, ctx.scope, conversation_id, &input.request.body).await?;
        operation_output(message_item(message))
    })
}

pub(crate) fn operations() -> Vec<Arc<PluginOperation>> {
    vec![Arc::new(PluginOperation::write(
        "messaging.reply",
        "Queue a reply in a shared inbox conversation (needs messaging view and reply permissions): { conversation_id, body }",
        reply,
    )
        .with_durable_mutations())]
}
