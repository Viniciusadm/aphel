use aphel_kernel::modules::OperationSpec;
use aphel_sdk::{OperationKey, operation};
use std::sync::Arc;

use serde::Deserialize;
use serde_json::{Value, json};

use crate::domain::QuoteDetails;
use crate::hooks::AcceptingQuote;
use crate::http::{AcceptQuoteManuallyRequest, SendQuoteRequest, SubmitQuoteRequest, quote_detail};
use crate::{service, workflow};
use aphel_kernel::Ctx;
use aphel_kernel::errors::AppError;
use aphel_kernel::modules::{
    HookCtx, OperationFuture, PluginHook, PluginHookKind, forward_plugin_hook, operation_input,
};

pub(crate) const ACCEPTING: &str = "quote.accepting";

fn accepting_json(accepting: &AcceptingQuote) -> Value {
    let mut quote = service::quote_snapshot(&accepting.quote);
    if let Value::Object(map) = &mut quote {
        map.insert("id".to_owned(), json!(accepting.quote_id()));
    }
    json!({
        "quote": quote,
        "deal_id": accepting.deal_id(),
        "acceptor": {
            "name": accepting.acceptor_name(),
            "email": accepting.acceptor_email(),
        },
        "manual": accepting.acceptance.recorded_by.is_some(),
    })
}

pub(crate) async fn accepting(
    ctx: &mut HookCtx<'_>,
    accepting: &AcceptingQuote,
) -> Result<(), AppError> {
    forward_plugin_hook(ctx, ACCEPTING, "quotes.quote", accepting.quote_id(), || {
        accepting_json(accepting)
    })
    .await
    .map(drop)
}

pub(crate) fn hooks() -> Vec<Arc<PluginHook>> {
    vec![Arc::new(PluginHook::new(
        ACCEPTING,
        "quotes.quote",
        PluginHookKind::Veto,
        "Before a quote is accepted (public page or manually); values: { quote, deal_id, acceptor: { name, email }, manual }. An error blocks the acceptance",
    ))]
}

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
