use aphel_contract_crm::CrmPermissions;
use aphel_kernel::modules::OperationSpec;
use aphel_sdk::{OperationKey, operation};

use serde::Deserialize;
use serde_json::Value;
use uuid::Uuid;

use crate::SEND;
use crate::http::{CreateEsignRequest, present, present_many};
use crate::service;
use aphel_kernel::Ctx;
use aphel_kernel::modules::{OperationFuture, operation_input, operation_output};
use aphel_sdk::platform::application::access::Authorizer;
use aphel_sdk::platform::domain::access::Permission;

#[derive(Deserialize)]
struct RequestId {
    id: String,
}

fn send<'a>(ctx: &'a Ctx<'_>, input: Value) -> OperationFuture<'a> {
    Box::pin(async move {
        ctx.scope.require(&SEND)?;
        ctx.scope.require(&Permission::DEALS_READ)?;
        let request: CreateEsignRequest = operation_input(input)?;
        let (row, signers) = service::send(ctx, ctx.scope, request.into_input()?).await?;
        operation_output(present(row, signers))
    })
}

fn cancel<'a>(ctx: &'a Ctx<'_>, input: Value) -> OperationFuture<'a> {
    Box::pin(async move {
        ctx.scope.require(&SEND)?;
        ctx.scope.require(&Permission::DEALS_READ)?;
        let input: RequestId = operation_input(input)?;
        let id = Uuid::parse_str(input.id.trim()).map_err(|_| service::request_not_found())?;
        let row = service::cancel(ctx, ctx.scope, id).await?;
        let mut items = present_many(ctx, vec![row]).await?;
        operation_output(items.remove(0))
    })
}

const OP_ESIGN_SEND: OperationKey<Value, Value> = operation!("esign.send");
const OP_ESIGN_CANCEL: OperationKey<Value, Value> = operation!("esign.cancel");

pub(crate) fn register(app: &mut aphel_kernel::modules::AppBuilder) {
    app.operation(OP_ESIGN_SEND, OperationSpec::write("Queue a PDF attached to a deal for electronic signature (needs esign.send): { deal_id, file_id, signers: [{ name, email, contact_id? }], message?, quote_id? }").durable(), send);
    app.operation(
        OP_ESIGN_CANCEL,
        OperationSpec::write(
            "Queue cancellation of an open signature request (needs esign.send): { id }",
        )
        .durable(),
        cancel,
    );
}
