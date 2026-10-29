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
    HookCtx, OperationFuture, PluginHook, PluginHookKind, PluginOperation, forward_plugin_hook,
    operation_input,
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
    forward_plugin_hook(ctx, ACCEPTING, "quote", accepting.quote_id(), || {
        accepting_json(accepting)
    })
    .await
    .map(drop)
}

pub(crate) fn hooks() -> Vec<Arc<PluginHook>> {
    vec![Arc::new(PluginHook::new(
        ACCEPTING,
        "quote",
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
    quote_detail(&ctx.state(), ctx.scope, details).await
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

pub(crate) fn operations() -> Vec<Arc<PluginOperation>> {
    vec![
        Arc::new(PluginOperation::write(
            "quotes.submit",
            "Send a draft quote for approval (approved right away when no policy applies): { id, comment? }",
            submit,
        )
        .with_durable_mutations()),
        Arc::new(PluginOperation::write(
            "quotes.send",
            "E-mail the quote link to the customer: { id, to?: [{ address, name? }], cc?: [{ address, name? }], subject?, body_html?, mailbox_id? }",
            send,
        )),
        Arc::new(PluginOperation::write(
            "quotes.accept",
            "Record the acceptance of a quote on behalf of the customer: { id, name?, email?, note? }",
            accept,
        )
        .with_durable_mutations()),
        Arc::new(PluginOperation::write(
            "quotes.new_version",
            "Create a new version of the current quote: { id }",
            new_version,
        )
        .with_durable_mutations()),
    ]
}
