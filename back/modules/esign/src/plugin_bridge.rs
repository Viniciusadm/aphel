use aphel_contract_crm::CrmPermissions;
use std::sync::Arc;

use serde::Deserialize;
use serde_json::Value;
use uuid::Uuid;

use crate::SEND;
use crate::http::{CreateEsignRequest, present, present_many};
use crate::service;
use aphel_kernel::Ctx;
use aphel_kernel::application::access::Authorizer;
use aphel_kernel::domain::access::Permission;
use aphel_kernel::modules::{OperationFuture, PluginOperation, operation_input, operation_output};

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

pub(crate) fn operations() -> Vec<Arc<PluginOperation>> {
    vec![
        Arc::new(PluginOperation::write(
            "esign.send",
            "Queue a PDF attached to a deal for electronic signature (needs esign.send): { deal_id, file_id, signers: [{ name, email, contact_id? }], message?, quote_id? }",
            send,
        ).with_durable_mutations()),
        Arc::new(PluginOperation::write(
            "esign.cancel",
            "Queue cancellation of an open signature request (needs esign.send): { id }",
            cancel,
        ).with_durable_mutations()),
    ]
}
