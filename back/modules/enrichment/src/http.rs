use axum::Json;
use axum::extract::{Path, State};
use chrono::{DateTime, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;
use uuid::Uuid;

use crate::service::{self, Applied, Lookup, ProposalItem};
use crate::store::{self, FieldSourceRow};
use crate::{AUTOMATIC_SETTING, CNPJ_FIELD_SETTING, COMPANY, SOURCE_SETTING, USE, cnpj};
use aphel_kernel::api::dto::common::{Data, ErrorEnvelope};
use aphel_kernel::api::error::ApiError;
use aphel_kernel::api::extractors::{AuthContext, JsonBody};
use aphel_kernel::application::access::Authorizer;
use aphel_kernel::context::Ctx;
use aphel_kernel::domain::access::Permission;
use aphel_kernel::errors::{AppError, FieldViolation};
use aphel_kernel::state::AppState;

pub(crate) const TAG: &str = "enrichment";
const INVALID_REQUEST: &str = "enrichment_invalid_request";

pub(crate) fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(company_overview))
        .routes(routes!(create_lookup))
        .routes(routes!(apply_lookup))
}

fn timestamp(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(SecondsFormat::Secs, true)
}

fn parse_uuid(raw: &str, field: &str) -> Result<Uuid, AppError> {
    Uuid::parse_str(raw.trim()).map_err(|_| {
        AppError::invalid_fields_code(
            INVALID_REQUEST,
            "invalid identifier",
            vec![FieldViolation::new(field, "must be a valid id")],
        )
    })
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct EnrichmentFieldSource {
    pub(crate) field_key: String,
    #[schema(example = "legal_name")]
    pub(crate) data_key: String,
    #[schema(example = "brasilapi")]
    pub(crate) source: String,
    #[schema(example = "00.000.000/0001-91")]
    pub(crate) cnpj: String,
    pub(crate) value_snapshot: Option<String>,
    pub(crate) fetched_at: String,
    pub(crate) applied_by: Option<String>,
    pub(crate) applied_at: String,
}

impl From<FieldSourceRow> for EnrichmentFieldSource {
    fn from(row: FieldSourceRow) -> Self {
        Self {
            field_key: row.field_key,
            data_key: row.data_key,
            source: row.source,
            cnpj: cnpj::format(&row.source_reference),
            value_snapshot: row.value_snapshot,
            fetched_at: timestamp(row.fetched_at),
            applied_by: row.applied_by.map(|id| id.to_string()),
            applied_at: timestamp(row.applied_at),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct EnrichmentCompanyOverview {
    pub(crate) company_id: String,
    pub(crate) cnpj_field: Option<String>,
    pub(crate) cnpj: Option<String>,
    #[schema(example = "brasilapi_then_receitaws")]
    pub(crate) source_mode: String,
    pub(crate) automatic: bool,
    pub(crate) sources: Vec<EnrichmentFieldSource>,
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct EnrichmentProposalItem {
    #[schema(example = "legal_name")]
    pub(crate) data_key: String,
    pub(crate) label: String,
    pub(crate) field_key: Option<String>,
    pub(crate) field_label: Option<String>,
    #[schema(value_type = Object)]
    pub(crate) current: Value,
    pub(crate) proposed: String,
    pub(crate) changed: bool,
}

impl From<ProposalItem> for EnrichmentProposalItem {
    fn from(item: ProposalItem) -> Self {
        Self {
            data_key: item.data_key,
            label: item.label,
            field_key: item.field_key,
            field_label: item.field_label,
            current: item.current,
            proposed: item.proposed,
            changed: item.changed,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct EnrichmentLookup {
    pub(crate) id: String,
    pub(crate) company_id: String,
    #[schema(example = "00.000.000/0001-91")]
    pub(crate) cnpj: String,
    #[schema(example = "brasilapi")]
    pub(crate) source: String,
    pub(crate) fetched_at: String,
    pub(crate) items: Vec<EnrichmentProposalItem>,
}

impl From<Lookup> for EnrichmentLookup {
    fn from(lookup: Lookup) -> Self {
        Self {
            id: lookup.id.to_string(),
            company_id: lookup.company_id.to_string(),
            cnpj: cnpj::format(&lookup.cnpj),
            source: lookup.source,
            fetched_at: timestamp(lookup.fetched_at),
            items: lookup.items.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct EnrichmentApplied {
    pub(crate) company_id: String,
    pub(crate) source: String,
    pub(crate) cnpj: String,
    pub(crate) fields: Vec<String>,
}

impl From<Applied> for EnrichmentApplied {
    fn from(applied: Applied) -> Self {
        Self {
            company_id: applied.company_id.to_string(),
            source: applied.source,
            cnpj: cnpj::format(&applied.cnpj),
            fields: applied.fields,
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub(crate) struct CreateEnrichmentLookup {
    pub(crate) company_id: String,
    #[schema(example = "00.000.000/0001-91")]
    pub(crate) cnpj: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub(crate) struct ApplyEnrichmentLookup {
    pub(crate) items: Vec<String>,
    #[serde(default)]
    pub(crate) only_blank: bool,
}

#[utoipa::path(
    get,
    path = "/enrichment/companies/{company_id}",
    tag = TAG,
    operation_id = "enrichment_company",
    security(("session_cookie" = [])),
    description = "Enrichment state of a company: configured CNPJ field and its value, source mode, whether automatic enrichment is allowed and the source (provider, CNPJ, date) of every field filled by the module. Requires `enrichment.use` and `companies.read`.",
    params(("company_id" = String, Path, description = "Company id")),
    responses(
        (status = 200, description = "Company enrichment state", body = Data<EnrichmentCompanyOverview>),
        (status = 403, description = "Missing permission (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Company not found or not visible (`company_not_found`) or module disabled (`module_disabled`)", body = ErrorEnvelope),
    )
)]
async fn company_overview(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(company_id): Path<String>,
) -> Result<Json<Data<EnrichmentCompanyOverview>>, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    ctx.scope.require(&USE)?;
    ctx.scope.require(&Permission::COMPANIES_READ)?;
    let company_id =
        Uuid::parse_str(company_id.trim()).map_err(|_| service::company_not_found())?;
    let record = service::load_company(&ctx, company_id).await?;
    let settings = service::settings(&ctx).await?;
    let cnpj_field = settings
        .get(CNPJ_FIELD_SETTING)
        .and_then(Value::as_str)
        .filter(|key| !key.is_empty())
        .map(str::to_owned);
    let cnpj = cnpj_field.as_ref().and_then(|field| {
        record
            .get(field)
            .or_else(|| {
                record
                    .get("custom_fields")
                    .and_then(|custom| custom.get(field))
            })
            .and_then(Value::as_str)
            .map(str::to_owned)
    });
    let sources = store::field_sources(ctx.pool(), COMPANY, company_id).await?;
    Ok(Json(Data {
        data: EnrichmentCompanyOverview {
            company_id: company_id.to_string(),
            cnpj_field,
            cnpj,
            source_mode: settings
                .get(SOURCE_SETTING)
                .and_then(Value::as_str)
                .unwrap_or(crate::sources::BRASILAPI_THEN_RECEITAWS)
                .to_owned(),
            automatic: settings
                .get(AUTOMATIC_SETTING)
                .and_then(Value::as_bool)
                .unwrap_or(false),
            sources: sources.into_iter().map(Into::into).collect(),
        },
    }))
}

#[utoipa::path(
    post,
    path = "/enrichment/lookups",
    tag = TAG,
    operation_id = "enrichment_lookup",
    security(("session_cookie" = [])),
    description = "Looks up the company CNPJ (typed or read from the configured CNPJ field) in the configured public source (BrasilAPI, ReceitaWS or BrasilAPI with ReceitaWS as fallback) and returns the proposal field by field (current x new value). Nothing is written to the company: apply with `POST /lookups/{lookup_id}/apply` within 60 minutes. Requires `enrichment.use` and `companies.read`.",
    request_body = CreateEnrichmentLookup,
    responses(
        (status = 201, description = "Lookup result", body = Data<EnrichmentLookup>),
        (status = 403, description = "Missing permission (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Company not found (`company_not_found`), CNPJ not found in the source (`enrichment_cnpj_not_found`) or module disabled (`module_disabled`)", body = ErrorEnvelope),
        (status = 422, description = "Missing CNPJ (`enrichment_cnpj_missing`), invalid CNPJ (`enrichment_invalid_cnpj`) or invalid request (`enrichment_invalid_request`)", body = ErrorEnvelope),
        (status = 429, description = "Too many lookups (`rate_limited`) or source limit reached (`enrichment_source_rate_limited`)", body = ErrorEnvelope),
        (status = 503, description = "Source unavailable (`enrichment_source_unavailable`)", body = ErrorEnvelope),
    )
)]
async fn create_lookup(
    State(state): State<AppState>,
    tenant: AuthContext,
    JsonBody(request): JsonBody<CreateEnrichmentLookup>,
) -> Result<(axum::http::StatusCode, Json<Data<EnrichmentLookup>>), ApiError> {
    let company_id = parse_uuid(&request.company_id, "company_id")?;
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let lookup = service::lookup(&ctx, company_id, request.cnpj).await?;
    Ok((
        axum::http::StatusCode::CREATED,
        Json(Data {
            data: lookup.into(),
        }),
    ))
}

#[utoipa::path(
    post,
    path = "/enrichment/lookups/{lookup_id}/apply",
    tag = TAG,
    operation_id = "enrichment_apply",
    security(("session_cookie" = [])),
    description = "Writes the chosen items of a lookup to the mapped company fields through the core (same permissions and validation as editing the company), records the source of each filled field and publishes `company.enriched` with `changes`. `only_blank` skips fields that already have a value. Requires `enrichment.use` and `companies.update`.",
    params(("lookup_id" = String, Path, description = "Lookup id")),
    request_body = ApplyEnrichmentLookup,
    responses(
        (status = 200, description = "Fields applied", body = Data<EnrichmentApplied>),
        (status = 403, description = "Missing permission (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Lookup not found (`enrichment_lookup_not_found`), company not found (`company_not_found`) or module disabled (`module_disabled`)", body = ErrorEnvelope),
        (status = 422, description = "Lookup expired (`enrichment_lookup_expired`), nothing to apply (`enrichment_nothing_to_apply`) or a value refused by the company validation (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn apply_lookup(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(lookup_id): Path<String>,
    JsonBody(request): JsonBody<ApplyEnrichmentLookup>,
) -> Result<Json<Data<EnrichmentApplied>>, ApiError> {
    let lookup_id = Uuid::parse_str(lookup_id.trim()).map_err(|_| {
        AppError::not_found_code(service::LOOKUP_NOT_FOUND, "consulta não encontrada")
    })?;
    if request.items.len() > 64 {
        return Err(AppError::validation_code(INVALID_REQUEST, "too many items").into());
    }
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let applied = service::apply(&ctx, lookup_id, &request.items, request.only_blank).await?;
    Ok(Json(Data {
        data: applied.into(),
    }))
}
