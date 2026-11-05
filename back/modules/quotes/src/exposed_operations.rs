use aphel_kernel::modules::OperationSpec;
use aphel_sdk::{OperationKey, operation};

use serde::Deserialize;
use serde_json::Value;

use crate::domain::QuoteDetails;
use crate::http::{AcceptQuoteManuallyRequest, SendQuoteRequest, SubmitQuoteRequest, quote_detail};
use crate::{service, workflow};
use aphel_kernel::Ctx;
use aphel_kernel::errors::AppError;
use aphel_kernel::modules::{OperationFuture, operation_input};

#[derive(Deserialize)]
struct QuoteId {
    id: String,
}

#[derive(Deserialize)]
struct SubmitInput {
    id: String,
    #[serde(flatten)]
    request: SubmitQuoteRequest,
}

#[derive(Deserialize)]
struct SendInput {
    id: String,
    #[serde(flatten)]
    request: SendQuoteRequest,
}

#[derive(Deserialize)]
struct AcceptInput {
    id: String,
    #[serde(flatten)]
    request: AcceptQuoteManuallyRequest,
}

async fn detail(ctx: &Ctx<'_>, details: QuoteDetails) -> Result<Value, AppError> {
    quote_detail(&ctx.runtime(), ctx.scope, details).await
}

fn submit<'a>(ctx: &'a Ctx<'_>, input: Value) -> OperationFuture<'a> {
    Box::pin(async move {
        let input: SubmitInput = operation_input(input)?;
        let details = workflow::submit(ctx, &input.id, input.request.into_command(None)).await?;
        detail(ctx, details).await
    })
}

fn send<'a>(ctx: &'a Ctx<'_>, input: Value) -> OperationFuture<'a> {
    Box::pin(async move {
        let input: SendInput = operation_input(input)?;
        let details = workflow::send(ctx, &input.id, input.request.into_command(None)).await?;
        detail(ctx, details).await
    })
}

fn accept<'a>(ctx: &'a Ctx<'_>, input: Value) -> OperationFuture<'a> {
    Box::pin(async move {
        let input: AcceptInput = operation_input(input)?;
        let details = workflow::accept(ctx, &input.id, input.request.into_command(None)).await?;
        detail(ctx, details).await
    })
}

fn new_version<'a>(ctx: &'a Ctx<'_>, input: Value) -> OperationFuture<'a> {
    Box::pin(async move {
        let input: QuoteId = operation_input(input)?;
        let details = service::create_version(ctx, &input.id, None).await?;
        detail(ctx, details).await
    })
}

const OP_QUOTES_SUBMIT: OperationKey<Value, Value> = operation!("quotes.submit");
const OP_QUOTES_SEND: OperationKey<Value, Value> = operation!("quotes.send");
const OP_QUOTES_ACCEPT: OperationKey<Value, Value> = operation!("quotes.accept");
const OP_QUOTES_NEW_VERSION: OperationKey<Value, Value> = operation!("quotes.new_version");

pub(crate) fn register(app: &mut aphel_kernel::modules::AppBuilder) {
    app.operation(OP_QUOTES_SUBMIT, OperationSpec::write("Send a draft quote for approval (approved right away when no policy applies): { id, comment? }").durable(), submit);
    app.operation(OP_QUOTES_SEND, OperationSpec::write("Queue the quote link e-mail to the customer: { id, to?: [{ address, name? }], cc?: [{ address, name? }], subject?, body_html?, mailbox_id? }").durable(), send);
    app.operation(OP_QUOTES_ACCEPT, OperationSpec::write("Record the acceptance of a quote on behalf of the customer: { id, name?, email?, note? }").durable(), accept);
    app.operation(
        OP_QUOTES_NEW_VERSION,
        OperationSpec::write("Create a new version of the current quote: { id }").durable(),
        new_version,
    );
}
