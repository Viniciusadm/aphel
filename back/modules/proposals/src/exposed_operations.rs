use aphel_kernel::modules::OperationSpec;
use aphel_sdk::{OperationKey, operation};

use serde::Deserialize;
use serde_json::Value;

use crate::errors;
use crate::http::{GenerateProposalRequest, ProposalDocument, optional_id, parse_id};
use crate::service::{self, GENERATE, TemplateChoice};
use aphel_kernel::Ctx;
use aphel_kernel::modules::{OperationFuture, operation_input, operation_output};
use aphel_sdk::platform::application::access::Authorizer;

#[derive(Deserialize)]
struct GenerateInput {
    deal_id: String,
    #[serde(default)]
    template_key: Option<String>,
    #[serde(flatten)]
    request: GenerateProposalRequest,
}

fn generate<'a>(ctx: &'a Ctx<'_>, input: Value) -> OperationFuture<'a> {
    Box::pin(async move {
        ctx.scope.require(&GENERATE)?;
        let input: GenerateInput = operation_input(input)?;
        let deal_id = parse_id(&input.deal_id, errors::deal_not_found)?;
        let template_key = input
            .template_key
            .map(|key| key.trim().to_owned())
            .filter(|key| !key.is_empty());
        let choice = match (
            optional_id(input.request.template_id.as_deref(), "template_id")?,
            template_key,
        ) {
            (Some(id), _) => TemplateChoice::Id(id),
            (None, Some(key)) => TemplateChoice::Key(key),
            (None, None) => TemplateChoice::Default,
        };
        let quote_id = optional_id(input.request.quote_id.as_deref(), "quote_id")?;
        let document = service::generate(ctx, deal_id, choice, quote_id).await?;
        operation_output(ProposalDocument::from(document))
    })
}

const OP_PROPOSALS_GENERATE: OperationKey<Value, Value> = operation!("proposals.generate");

pub(crate) fn register(app: &mut aphel_kernel::modules::AppBuilder) {
    app.operation(OP_PROPOSALS_GENERATE, OperationSpec::write("Generate a PDF proposal for a deal and attach it (needs proposals.generate and edit access to the deal; the default active template when none is given): { deal_id, template_id?, template_key?, quote_id? }").durable(), generate);
}
