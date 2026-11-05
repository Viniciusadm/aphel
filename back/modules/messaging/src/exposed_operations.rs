use aphel_kernel::modules::OperationSpec;
use aphel_sdk::{OperationKey, operation};

use serde::Deserialize;
use serde_json::Value;
use uuid::Uuid;

use crate::errors::conversation_not_found;
use crate::inbox_routes::message_item;
use crate::model::ReplyRequest;
use crate::sending;
use crate::{REPLY, VIEW};
use aphel_kernel::Ctx;
use aphel_kernel::modules::{OperationFuture, operation_input, operation_output};
use aphel_sdk::platform::application::access::Authorizer;

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

const OP_MESSAGING_REPLY: OperationKey<Value, Value> = operation!("messaging.reply");

pub(crate) fn register(app: &mut aphel_kernel::modules::AppBuilder) {
    app.operation(OP_MESSAGING_REPLY, OperationSpec::write("Queue a reply in a shared inbox conversation (needs messaging view and reply permissions): { conversation_id, body }").durable(), reply);
}
