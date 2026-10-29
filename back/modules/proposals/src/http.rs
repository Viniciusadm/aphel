use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use chrono::{DateTime, SecondsFormat, Utc};
use serde::{Deserialize, Deserializer, Serialize};
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;
use uuid::Uuid;

use aphel_kernel::api::dto::common::{Data, ErrorEnvelope};
use aphel_kernel::api::error::ApiError;
use aphel_kernel::api::extractors::{AuthContext, JsonBody};
use aphel_kernel::application::access::Authorizer;
use aphel_kernel::context::Ctx;
use aphel_kernel::domain::access::{AccessScope, Permission};
use aphel_kernel::errors::{AppError, FieldViolation};
use aphel_kernel::state::AppState;

use crate::errors::{self, INVALID_REQUEST};
use crate::markup::{self, Issue};
use crate::render::RenderedBlock;
use crate::service::{self, GENERATE, MANAGE, TemplateChoice, TemplatePatch, clean_description};
use crate::store::{self, DocumentRow, TemplateRow, TemplateValues};
use crate::variables::{self, VariableEntry};

pub(crate) const TAG: &str = "proposals";

pub(crate) fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list_variables))
        .routes(routes!(list_templates, create_template))
        .routes(routes!(validate_template))
        .routes(routes!(get_template, update_template, delete_template))
        .routes(routes!(preview))
        .routes(routes!(list_documents, generate_document))
}

fn timestamp(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(SecondsFormat::Secs, true)
}

pub(crate) fn parse_id(raw: &str, not_found: fn() -> AppError) -> Result<Uuid, AppError> {
    Uuid::parse_str(raw.trim()).map_err(|_| not_found())
}

pub(crate) fn optional_id(raw: Option<&str>, field: &str) -> Result<Option<Uuid>, AppError> {
    raw.map(str::trim)
        .filter(|raw| !raw.is_empty())
        .map(|raw| {
            Uuid::parse_str(raw).map_err(|_| {
                AppError::invalid_fields_code(
                    INVALID_REQUEST,
                    "invalid identifier",
                    vec![FieldViolation::new(field, "must be a valid id")],
                )
            })
        })
        .transpose()
}

fn double_option<'de, D, T>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer).map(Some)
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct ProposalTemplate {
    pub(crate) id: String,
    #[schema(example = "sales_proposal")]
    pub(crate) key: String,
    pub(crate) name: String,
    pub(crate) description: Option<String>,
    pub(crate) body: String,
    pub(crate) is_active: bool,
    pub(crate) is_default: bool,
    #[schema(example = "2026-10-20T13:45:00Z")]
    pub(crate) created_at: String,
    #[schema(example = "2026-10-20T13:45:00Z")]
    pub(crate) updated_at: String,
}

impl From<TemplateRow> for ProposalTemplate {
    fn from(row: TemplateRow) -> Self {
        Self {
            id: row.id.to_string(),
            key: row.key,
            name: row.name,
            description: row.description,
            body: row.body,
            is_active: row.is_active,
            is_default: row.is_default,
            created_at: timestamp(row.created_at),
            updated_at: timestamp(row.updated_at),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct ProposalDocument {
    pub(crate) id: String,
    pub(crate) deal_id: String,
    pub(crate) template_id: Option<String>,
    pub(crate) template_name: String,
    #[schema(example = 12)]
    pub(crate) number: u32,
    pub(crate) file_id: String,
    pub(crate) file_name: String,
    pub(crate) file_size: u64,
    pub(crate) generated_by: Option<String>,
    pub(crate) generated_by_name: Option<String>,
    #[schema(example = "2026-10-20T13:45:00Z")]
    pub(crate) created_at: String,
}

impl From<DocumentRow> for ProposalDocument {
    fn from(row: DocumentRow) -> Self {
        Self {
            id: row.id.to_string(),
            deal_id: row.deal_id.to_string(),
            template_id: row.template_id.map(|id| id.to_string()),
            template_name: row.template_name,
            number: row.number,
            file_id: row.file_id.to_string(),
            file_name: row.file_name,
            file_size: row.file_size,
            generated_by: row.generated_by.map(|id| id.to_string()),
            generated_by_name: row.generated_by_name,
            created_at: timestamp(row.created_at),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct ProposalValidation {
    pub(crate) valid: bool,
    pub(crate) issues: Vec<Issue>,
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct ProposalPreview {
    pub(crate) blocks: Vec<RenderedBlock>,
    pub(crate) issues: Vec<Issue>,
    pub(crate) sample: bool,
}

#[derive(Debug, Deserialize, ToSchema)]
pub(crate) struct CreateProposalTemplateRequest {
    #[schema(example = "sales_proposal")]
    pub(crate) key: Option<String>,
    pub(crate) name: String,
    pub(crate) description: Option<String>,
    pub(crate) body: String,
    pub(crate) is_active: Option<bool>,
    pub(crate) is_default: Option<bool>,
}

#[derive(Debug, Default, Deserialize, ToSchema)]
pub(crate) struct UpdateProposalTemplateRequest {
    pub(crate) name: Option<String>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>)]
    pub(crate) description: Option<Option<String>>,
    pub(crate) body: Option<String>,
    pub(crate) is_active: Option<bool>,
    pub(crate) is_default: Option<bool>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub(crate) struct ValidateProposalTemplateRequest {
    pub(crate) body: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub(crate) struct PreviewProposalRequest {
    pub(crate) body: Option<String>,
    pub(crate) template_id: Option<String>,
    pub(crate) deal_id: Option<String>,
    pub(crate) quote_id: Option<String>,
}

#[derive(Debug, Default, Deserialize, ToSchema)]
pub(crate) struct GenerateProposalRequest {
    pub(crate) template_id: Option<String>,
    pub(crate) quote_id: Option<String>,
}

fn can(scope: &AccessScope, permission: &Permission) -> bool {
    scope.scope_for(permission).is_some()
}

fn require_any(scope: &AccessScope, permissions: &[&Permission]) -> Result<(), AppError> {
    if permissions.iter().any(|permission| can(scope, permission)) {
        Ok(())
    } else {
        scope.require(permissions[0]).map(|_| ())
    }
}

#[utoipa::path(
    get,
    path = "/proposals/variables",
    tag = TAG,
    operation_id = "proposals_variables_list",
    security(("session_cookie" = [])),
    description = "Lists the variables available to proposal templates: deal, contact and company fields (including custom fields visible to the role), owner, user, organization, proposal, item totals and the blocks `items_table` and `page_break`. Requires `proposals.manage` or `proposals.generate`.",
    responses(
        (status = 200, description = "Variable catalog", body = Data<Vec<VariableEntry>>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `proposals.manage` (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Module disabled (`module_disabled`)", body = ErrorEnvelope),
    )
)]
async fn list_variables(
    State(state): State<AppState>,
    tenant: AuthContext,
) -> Result<Json<Data<Vec<VariableEntry>>>, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let scope = ctx.scope;
    require_any(scope, &[&MANAGE, &GENERATE])?;
    let catalog = variables::catalog(&ctx).await?;
    Ok(Json(Data {
        data: catalog.entries,
    }))
}

#[utoipa::path(
    get,
    path = "/proposals/templates",
    tag = TAG,
    operation_id = "proposals_templates_list",
    security(("session_cookie" = [])),
    description = "Lists the proposal templates, the default first. With `proposals.manage` all templates are listed; with only `proposals.generate`, only the active ones.",
    responses(
        (status = 200, description = "Templates", body = Data<Vec<ProposalTemplate>>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `proposals.manage` or `proposals.generate` (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Module disabled (`module_disabled`)", body = ErrorEnvelope),
    )
)]
async fn list_templates(
    State(state): State<AppState>,
    tenant: AuthContext,
) -> Result<Json<Data<Vec<ProposalTemplate>>>, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let scope = ctx.scope;
    require_any(scope, &[&MANAGE, &GENERATE])?;
    let only_active = !can(scope, &MANAGE);
    let templates = store::list_templates(ctx.pool(), only_active).await?;
    Ok(Json(Data {
        data: templates.into_iter().map(Into::into).collect(),
    }))
}

#[utoipa::path(
    post,
    path = "/proposals/templates",
    tag = TAG,
    operation_id = "proposals_templates_create",
    security(("session_cookie" = [])),
    description = "Creates a proposal template (permission `proposals.manage`) and publishes `proposal_template.created`. `key` is the stable key used by configuration packages (derived from the name when omitted, immutable). The body is validated: unknown variables or syntax errors return 422 `proposal_template_invalid` with the position (`line L, column C: <code> {{variable}}`) in `details.fields`. Marking it as default unmarks the previous default.",
    request_body = CreateProposalTemplateRequest,
    responses(
        (status = 201, description = "Template created", body = Data<ProposalTemplate>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `proposals.manage` (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Module disabled (`module_disabled`)", body = ErrorEnvelope),
        (status = 409, description = "Key already used by another template (`proposal_template_key_taken`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid template (`proposal_template_invalid`, `invalid_json`)", body = ErrorEnvelope),
    )
)]
async fn create_template(
    State(state): State<AppState>,
    tenant: AuthContext,
    JsonBody(request): JsonBody<CreateProposalTemplateRequest>,
) -> Result<(StatusCode, Json<Data<ProposalTemplate>>), ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let scope = ctx.scope;
    scope.require(&MANAGE)?;
    let key = match request.key.map(|key| key.trim().to_owned()) {
        Some(key) if !key.is_empty() => key,
        _ => service::unique_key(&ctx, &service::key_from_name(&request.name)).await?,
    };
    let is_active = request.is_active.unwrap_or(true);
    let values = TemplateValues {
        key,
        name: request.name.trim().to_owned(),
        description: clean_description(request.description),
        body: request.body,
        is_active,
        is_default: request.is_default.unwrap_or(false),
    };
    let template = service::create_template(&ctx, values).await?;
    Ok((
        StatusCode::CREATED,
        Json(Data {
            data: template.into(),
        }),
    ))
}

#[utoipa::path(
    post,
    path = "/proposals/templates/validate",
    tag = TAG,
    operation_id = "proposals_templates_validate",
    security(("session_cookie" = [])),
    description = "Validates a template body without saving it (permission `proposals.manage`). Returns every issue with line and column: `unknown_variable`, `invalid_variable`, `empty_variable`, `unclosed_variable` and `block_variable_inline` (`items_table`/`page_break` must be alone on their line).",
    request_body = ValidateProposalTemplateRequest,
    responses(
        (status = 200, description = "Validation result", body = Data<ProposalValidation>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `proposals.manage` (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Module disabled (`module_disabled`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid request body (`invalid_json`)", body = ErrorEnvelope),
    )
)]
async fn validate_template(
    State(state): State<AppState>,
    tenant: AuthContext,
    JsonBody(request): JsonBody<ValidateProposalTemplateRequest>,
) -> Result<Json<Data<ProposalValidation>>, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let scope = ctx.scope;
    scope.require(&MANAGE)?;
    let catalog = variables::catalog(&ctx).await?;
    let issues = catalog.validate(&markup::parse(&request.body));
    Ok(Json(Data {
        data: ProposalValidation {
            valid: issues.is_empty(),
            issues,
        },
    }))
}

#[utoipa::path(
    get,
    path = "/proposals/templates/{template_id}",
    tag = TAG,
    operation_id = "proposals_templates_get",
    security(("session_cookie" = [])),
    params(("template_id" = String, Path, description = "Template id")),
    responses(
        (status = 200, description = "Template", body = Data<ProposalTemplate>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `proposals.manage` or `proposals.generate` (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Template not found (`proposal_template_not_found`) or module disabled (`module_disabled`)", body = ErrorEnvelope),
    )
)]
async fn get_template(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(template_id): Path<String>,
) -> Result<Json<Data<ProposalTemplate>>, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let scope = ctx.scope;
    require_any(scope, &[&MANAGE, &GENERATE])?;
    let id = parse_id(&template_id, errors::template_not_found)?;
    let template = service::load_template(&ctx, id).await?;
    if !template.is_active && !can(scope, &MANAGE) {
        return Err(errors::template_not_found().into());
    }
    Ok(Json(Data {
        data: template.into(),
    }))
}

#[utoipa::path(
    patch,
    path = "/proposals/templates/{template_id}",
    tag = TAG,
    operation_id = "proposals_templates_update",
    security(("session_cookie" = [])),
    description = "Partially updates a template (permission `proposals.manage`) and publishes `proposal_template.updated`; omitted properties stay unchanged. The key is immutable. Deactivating a template also removes its default mark.",
    params(("template_id" = String, Path, description = "Template id")),
    request_body = UpdateProposalTemplateRequest,
    responses(
        (status = 200, description = "Template updated", body = Data<ProposalTemplate>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `proposals.manage` (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Template not found (`proposal_template_not_found`) or module disabled (`module_disabled`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid template (`proposal_template_invalid`, `invalid_json`)", body = ErrorEnvelope),
    )
)]
async fn update_template(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(template_id): Path<String>,
    JsonBody(request): JsonBody<UpdateProposalTemplateRequest>,
) -> Result<Json<Data<ProposalTemplate>>, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let scope = ctx.scope;
    scope.require(&MANAGE)?;
    let id = parse_id(&template_id, errors::template_not_found)?;
    let template = service::update_template(
        &ctx,
        id,
        TemplatePatch {
            name: request.name,
            description: request.description,
            body: request.body,
            is_active: request.is_active,
            is_default: request.is_default,
        },
    )
    .await?;
    Ok(Json(Data {
        data: template.into(),
    }))
}

#[utoipa::path(
    delete,
    path = "/proposals/templates/{template_id}",
    tag = TAG,
    operation_id = "proposals_templates_delete",
    security(("session_cookie" = [])),
    description = "Deletes a template (permission `proposals.manage`) and publishes `proposal_template.deleted`. Proposals already generated keep their PDF and the template name.",
    params(("template_id" = String, Path, description = "Template id")),
    responses(
        (status = 204, description = "Template deleted"),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `proposals.manage` (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Template not found (`proposal_template_not_found`) or module disabled (`module_disabled`)", body = ErrorEnvelope),
    )
)]
async fn delete_template(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(template_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let scope = ctx.scope;
    scope.require(&MANAGE)?;
    let id = parse_id(&template_id, errors::template_not_found)?;
    service::delete_template(&ctx, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    post,
    path = "/proposals/preview",
    tag = TAG,
    operation_id = "proposals_preview",
    security(("session_cookie" = [])),
    description = "Renders a template as structured blocks without generating a PDF. Send `body` (editor content, requires `proposals.manage`) or `template_id` (a saved template, `proposals.manage` or `proposals.generate`). With `deal_id` the variables are filled with the data of that deal as seen by the user (hidden fields stay empty); with `quote_id` the quote (one version) is the data source: `quote.*` variables, and the items table and totals come from the quote lines; without them (`sample: true`) record variables appear as `[variable]` placeholders.",
    request_body = PreviewProposalRequest,
    responses(
        (status = 200, description = "Rendered blocks and template issues", body = Data<ProposalPreview>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`)", body = ErrorEnvelope),
        (status = 403, description = "Missing permission (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Deal not found or not visible (`deal_not_found`), template not found (`proposal_template_not_found`) or module disabled (`module_disabled`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid request (`proposal_invalid_request`, `invalid_json`)", body = ErrorEnvelope),
    )
)]
async fn preview(
    State(state): State<AppState>,
    tenant: AuthContext,
    JsonBody(request): JsonBody<PreviewProposalRequest>,
) -> Result<Json<Data<ProposalPreview>>, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let scope = ctx.scope;
    let deal_id = optional_id(request.deal_id.as_deref(), "deal_id")?;
    let quote_id = optional_id(request.quote_id.as_deref(), "quote_id")?;
    let template_id = optional_id(request.template_id.as_deref(), "template_id")?;
    let (body, name) = match (request.body, template_id) {
        (Some(body), _) => {
            scope.require(&MANAGE)?;
            (body, String::new())
        }
        (None, Some(id)) => {
            require_any(scope, &[&MANAGE, &GENERATE])?;
            let template = service::load_template(&ctx, id).await?;
            (template.body, template.name)
        }
        (None, None) => {
            return Err(
                AppError::validation_code(INVALID_REQUEST, "send body or template_id").into(),
            );
        }
    };
    if deal_id.is_some() || quote_id.is_some() {
        scope.require(&Permission::DEALS_READ)?;
    }
    let preview = service::preview(&ctx, &body, &name, deal_id, quote_id).await?;
    Ok(Json(Data {
        data: ProposalPreview {
            blocks: preview.blocks,
            issues: preview.issues,
            sample: deal_id.is_none() && quote_id.is_none(),
        },
    }))
}

#[utoipa::path(
    get,
    path = "/proposals/deals/{deal_id}/documents",
    tag = TAG,
    operation_id = "proposals_documents_list",
    security(("session_cookie" = [])),
    description = "Lists the proposals generated for a deal, newest first (permissions `proposals.generate` and `deals.read` on the deal). Download the PDF through `GET /api/v1/files/{file_id}`.",
    params(("deal_id" = String, Path, description = "Deal id")),
    responses(
        (status = 200, description = "Generated proposals", body = Data<Vec<ProposalDocument>>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `proposals.generate` or `deals.read` (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Deal not found or not visible (`deal_not_found`) or module disabled (`module_disabled`)", body = ErrorEnvelope),
    )
)]
async fn list_documents(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(deal_id): Path<String>,
) -> Result<Json<Data<Vec<ProposalDocument>>>, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let scope = ctx.scope;
    scope.require(&GENERATE)?;
    scope.require(&Permission::DEALS_READ)?;
    let deal_id = parse_id(&deal_id, errors::deal_not_found)?;
    service::load_deal(&ctx, deal_id).await?;
    let documents = store::list_documents(ctx.pool(), deal_id).await?;
    Ok(Json(Data {
        data: documents.into_iter().map(Into::into).collect(),
    }))
}

#[utoipa::path(
    post,
    path = "/proposals/deals/{deal_id}/documents",
    tag = TAG,
    operation_id = "proposals_documents_generate",
    security(("session_cookie" = [])),
    description = "Generates a PDF proposal for the deal from a template (the default active template when `template_id` is omitted), attaches it to the deal as a file and publishes `proposal.generated` (automation trigger \"Proposta gerada\"). Requires `proposals.generate` and `deals.update` on the deal. Numbers are sequential. With `quote_id` (a quote version of this deal) the quote is the data source of the template: `quote.*` variables, items and totals of the quote.",
    params(("deal_id" = String, Path, description = "Deal id")),
    request_body = GenerateProposalRequest,
    responses(
        (status = 201, description = "Proposal generated", body = Data<ProposalDocument>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `proposals.generate` or `deals.update` (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Deal not found or not visible (`deal_not_found`), quote not found or of another deal (`quote_not_found`), template not found (`proposal_template_not_found`) or module disabled (`module_disabled`)", body = ErrorEnvelope),
        (status = 413, description = "Generated file larger than the upload limit (`file_too_large`)", body = ErrorEnvelope),
        (status = 422, description = "No active template (`proposal_template_required`), inactive template (`proposal_template_inactive`) or invalid request (`proposal_invalid_request`, `invalid_json`)", body = ErrorEnvelope),
    )
)]
async fn generate_document(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(deal_id): Path<String>,
    JsonBody(request): JsonBody<GenerateProposalRequest>,
) -> Result<(StatusCode, Json<Data<ProposalDocument>>), ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let scope = ctx.scope;
    scope.require(&GENERATE)?;
    let deal_id = parse_id(&deal_id, errors::deal_not_found)?;
    let choice = match optional_id(request.template_id.as_deref(), "template_id")? {
        Some(id) => TemplateChoice::Id(id),
        None => TemplateChoice::Default,
    };
    let quote_id = optional_id(request.quote_id.as_deref(), "quote_id")?;
    let document = service::generate(&ctx, deal_id, choice, quote_id).await?;
    Ok((
        StatusCode::CREATED,
        Json(Data {
            data: document.into(),
        }),
    ))
}
