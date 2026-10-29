use std::sync::Arc;

use serde::Deserialize;
use serde_json::Value;
use uuid::Uuid;

use crate::http::{
    ApplyEnrichmentLookup, CreateEnrichmentLookup, EnrichmentApplied, EnrichmentLookup, parse_uuid,
};
use crate::service;
use aphel_kernel::Ctx;
use aphel_kernel::errors::AppError;
use aphel_kernel::modules::{OperationFuture, PluginOperation, operation_input, operation_output};

const MAX_APPLY_ITEMS: usize = 64;

#[derive(Deserialize)]
struct ApplyInput {
    lookup_id: String,
    #[serde(flatten)]
    request: ApplyEnrichmentLookup,
}

fn lookup<'a>(ctx: &'a Ctx<'_>, input: Value) -> OperationFuture<'a> {
    Box::pin(async move {
        let request: CreateEnrichmentLookup = operation_input(input)?;
        let company_id = parse_uuid(&request.company_id, "company_id")?;
        let lookup = service::lookup(ctx, company_id, request.cnpj).await?;
        operation_output(EnrichmentLookup::from(lookup))
    })
}

fn apply<'a>(ctx: &'a Ctx<'_>, input: Value) -> OperationFuture<'a> {
    Box::pin(async move {
        let input: ApplyInput = operation_input(input)?;
        let lookup_id = Uuid::parse_str(input.lookup_id.trim()).map_err(|_| {
            AppError::not_found_code(service::LOOKUP_NOT_FOUND, "consulta não encontrada")
        })?;
        if input.request.items.len() > MAX_APPLY_ITEMS {
            return Err(AppError::validation_code(
                "enrichment_invalid_request",
                "too many items",
            ));
        }
        let applied = service::apply(
            ctx,
            lookup_id,
            &input.request.items,
            input.request.only_blank,
        )
        .await?;
        operation_output(EnrichmentApplied::from(applied))
    })
}

pub(crate) fn operations() -> Vec<Arc<PluginOperation>> {
    vec![
        Arc::new(PluginOperation::write(
            "enrichment.lookup",
            "Look up a company's CNPJ in the configured public source and return the field-by-field proposal (nothing is written): { company_id, cnpj? } -> { id, company_id, cnpj, source, fetched_at, items: [{ data_key, label, field_key, current, proposed, changed }] }",
            lookup,
        )),
        Arc::new(PluginOperation::write(
            "enrichment.apply",
            "Apply chosen items of a lookup (within 60 minutes) to the company: { lookup_id, items: [data_key], only_blank? } -> { company_id, source, cnpj, fields }",
            apply,
        )),
    ]
}
