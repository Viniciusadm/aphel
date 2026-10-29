use std::sync::Arc;

use serde::Deserialize;
use serde_json::{Value, json};

use crate::USE;
use crate::config::Feature;
use crate::context::ensure_context_type;
use crate::features::{self, PromptRequest};
use crate::model::{AiComposeRequest, AiRecordRequest};
use crate::provider::AiMessage;
use crate::service::AiSession;
use aphel_kernel::Ctx;
use aphel_kernel::application::access::Authorizer;
use aphel_kernel::errors::AppError;
use aphel_kernel::modules::{OperationFuture, PluginOperation, operation_input, operation_output};

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum PromptRole {
    User,
    Assistant,
}

#[derive(Deserialize)]
struct PromptMessage {
    role: PromptRole,
    content: String,
}

#[derive(Deserialize)]
struct CompleteInput {
    #[serde(default)]
    system: Option<String>,
    #[serde(default)]
    prompt: Option<String>,
    #[serde(default)]
    messages: Option<Vec<PromptMessage>>,
    #[serde(default)]
    max_tokens: Option<u32>,
    #[serde(default)]
    temperature: Option<f32>,
}

impl CompleteInput {
    fn into_request(self) -> Result<PromptRequest, AppError> {
        let messages = match (self.prompt, self.messages) {
            (Some(_), Some(_)) => {
                return Err(AppError::Validation(
                    "send either prompt or messages, not both".to_owned(),
                ));
            }
            (Some(prompt), None) => vec![AiMessage::user(prompt)],
            (None, Some(messages)) => messages
                .into_iter()
                .map(|message| match message.role {
                    PromptRole::User => AiMessage::user(message.content),
                    PromptRole::Assistant => AiMessage::assistant(message.content),
                })
                .collect(),
            (None, None) => Vec::new(),
        };
        Ok(PromptRequest {
            system: self.system,
            messages,
            max_tokens: self.max_tokens,
            temperature: self.temperature,
        })
    }
}

fn complete<'a>(ctx: &'a Ctx<'_>, input: Value) -> OperationFuture<'a> {
    Box::pin(async move {
        ctx.scope.require(&USE)?;
        let request = operation_input::<CompleteInput>(input)?.into_request()?;
        let session = AiSession::open(ctx, Feature::Plugin).await?;
        let completion = features::complete(ctx, &session, request).await?;
        Ok(json!({
            "text": completion.text,
            "input_tokens": completion.input_tokens,
            "output_tokens": completion.output_tokens,
            "provider": completion.provider,
            "model": completion.model,
        }))
    })
}

fn summary<'a>(ctx: &'a Ctx<'_>, input: Value) -> OperationFuture<'a> {
    Box::pin(async move {
        let request: AiRecordRequest = operation_input(input)?;
        ctx.scope.require(&USE)?;
        ensure_context_type(&request.record_type)?;
        let session = AiSession::open(ctx, Feature::Summary).await?;
        operation_output(
            features::summary(ctx, &session, &request.record_type, &request.record_id).await?,
        )
    })
}

fn next_actions<'a>(ctx: &'a Ctx<'_>, input: Value) -> OperationFuture<'a> {
    Box::pin(async move {
        let request: AiRecordRequest = operation_input(input)?;
        ctx.scope.require(&USE)?;
        ensure_context_type(&request.record_type)?;
        let session = AiSession::open(ctx, Feature::NextActions).await?;
        operation_output(
            features::next_actions(ctx, &session, &request.record_type, &request.record_id).await?,
        )
    })
}

fn compose<'a>(ctx: &'a Ctx<'_>, input: Value) -> OperationFuture<'a> {
    Box::pin(async move {
        let request: AiComposeRequest = operation_input(input)?;
        ctx.scope.require(&USE)?;
        features::validate_compose(&request)?;
        if let Some(record_type) = &request.record_type {
            ensure_context_type(record_type)?;
        }
        let session = AiSession::open(ctx, Feature::Compose).await?;
        operation_output(features::compose(ctx, &session, request).await?)
    })
}

pub(crate) fn operations() -> Vec<Arc<PluginOperation>> {
    vec![
        Arc::new(PluginOperation::write(
            "ai.complete",
            "Free completion with the configured AI provider, counted in the daily and per-user limits and logged as feature plugin: { system?, prompt } or { system?, messages: [{ role: user|assistant, content }] }, max_tokens? (1-4096, default 1024), temperature? (0-1) -> { text, input_tokens, output_tokens, provider, model }",
            complete,
        ).with_external_query()),
        Arc::new(PluginOperation::write(
            "ai.summary",
            "AI summary of a record the actor can read: { record_type: contact|company|deal, record_id } -> { text, sources, provider, model }",
            summary,
        ).with_external_query()),
        Arc::new(PluginOperation::write(
            "ai.next_actions",
            "Up to 5 suggested next actions for a record: { record_type: contact|company|deal, record_id } -> { suggestions: [{ title, due_in_days, reason }], sources, provider, model }",
            next_actions,
        ).with_external_query()),
        Arc::new(PluginOperation::write(
            "ai.compose",
            "Write or rewrite a text: { context: note|email|message, instruction, draft?, record_type?, record_id? } -> { text, sources, provider, model }",
            compose,
        ).with_external_query()),
    ]
}
