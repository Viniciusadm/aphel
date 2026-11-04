use aphel_contract_crm::CrmRecordTypes;
use async_trait::async_trait;
use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;
use uuid::Uuid;

use crate::domain::{CompanyDetails, CompanyInput, SaveCompanyCommand};
use crate::service;
use aphel_kernel::Ctx;
use aphel_kernel::api::dto::common::{
    Data, DuplicateMatchResponse, ErrorEnvelope, OwnerResponse, Page, QuickSearchParams,
    double_option, timestamp,
};
use aphel_kernel::api::dto::query::{RecordListQuery, RecordQueryParams};
use aphel_kernel::api::error::ApiError;
use aphel_kernel::api::extractors::{
    AuthContext, ClientInfo, IfMatch, IncludeQuery, JsonBody, Pagination, PaginationQuery,
};
use aphel_kernel::api::routes::v1::field_permissions::{redacted, redacted_items};
use aphel_kernel::api::state::AppState;
use aphel_kernel::application::shared::RecordType;
use aphel_kernel::database::transaction::MySqlTx;
use aphel_kernel::errors::AppError;
use aphel_kernel::records::{AddressInput, merge_survivor, possible_duplicates};
use aphel_kernel::records::{
    PendingCommit, RecordCtx, RecordTypeHandler, optional_record, values_request,
};
use aphel_sdk::records::record_includes;

pub(crate) const TAG: &str = "organizations";

pub(crate) fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list_companies, create_company))
        .routes(routes!(get_company, update_company, delete_company))
        .routes(routes!(archive_company))
        .routes(routes!(unarchive_company))
}

#[utoipa::path(
    get,
    path = "/organizations",
    tag = TAG,
    operation_id = "companies_list",
    description = "Lists the organizations visible to the user (permission `organizations.read`, limited by its record scope). Filters accept the JSON AST in `filter` or simple conditions as `filter[field][operator]=value` (`filter[industry][eq]=Varejo`, `filter[owner_id][in]=a,b`, `filter[domain]=acme.com.br` means `eq`), combined with `and`. `sort=field,-other` (up to 3); without `sort`, newest first. `search` (or its alias `q`) matches name, domain, email and phone; phone terms are normalized to E.164. `archived=exclude|include|only` (default `exclude`). The owner is always embedded; Enabled `include` contributions can enrich each company.",
    params(RecordQueryParams, QuickSearchParams, IncludeQuery, PaginationQuery),
    responses(
        (status = 200, description = "Page of organizations with their custom fields", body = Page<CompanyResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `organizations.read` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid filter, sort, search, include or pagination (`validation_error` with `details.fields`)", body = ErrorEnvelope),
    )
)]
async fn list_companies(
    State(state): State<AppState>,
    tenant: AuthContext,
    RecordListQuery(mut query): RecordListQuery,
    Query(quick): Query<QuickSearchParams>,
    Query(include): Query<IncludeQuery>,
    pagination: Pagination,
) -> Result<Json<Page<Value>>, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let includes = record_includes(
        &ctx,
        "organizations.organization",
        include.include.as_deref(),
    )?;
    quick.merge_into(&mut query.search);
    let listing = service::list(&ctx, query).await?;
    let items: Vec<CompanyResponse> = listing.items.into_iter().map(Into::into).collect();
    let mut items = redacted_items(&state, &tenant.scope, &RecordType::COMPANY, items).await?;
    includes.enrich(&ctx, &mut items).await?;
    Ok(Json(Page::new(items, listing.total, pagination)))
}

#[utoipa::path(
    post,
    path = "/organizations",
    tag = TAG,
    operation_id = "companies_create",
    security(("session_cookie" = [])),
    description = "Creates a company (permission `organizations.create`) and publishes `company.created`. Email is lowercased, phone normalized to E.164 and the domain taken from the website when missing; the owner defaults to the current user. Custom fields go in `custom_fields`. `possible_duplicates` (only in this response, empty when none) lists visible organizations that the duplicate rules match (a warning, never an error); those pairs also enter the duplicate queue.",
    request_body = CompanyRequest,
    responses(
        (status = 201, description = "Company created", body = Data<CompanyResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `organizations.create` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid system or custom fields (`validation_error`, fields such as `name` or `custom_fields.<key>`)", body = ErrorEnvelope),
    )
)]
async fn create_company(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    JsonBody(request): JsonBody<CompanyRequest>,
) -> Result<(StatusCode, Json<Data<Value>>), ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let company = service::create(&ctx, request.into_command(client.0.ip, None)).await?;
    let mut response: CompanyResponse = company.into();
    let matches = possible_duplicates(&ctx, &RecordType::COMPANY, &response.id).await;
    response.possible_duplicates = Some(matches.into_iter().map(Into::into).collect());
    Ok((
        StatusCode::CREATED,
        Json(Data::new(
            redacted(&state, &tenant.scope, &RecordType::COMPANY, response).await?,
        )),
    ))
}

#[utoipa::path(
    get,
    path = "/organizations/{company_id}",
    tag = TAG,
    operation_id = "companies_get",
    security(("session_cookie" = [])),
    description = "Returns a company with its custom fields and owner (permission `organizations.read`). The id of a company merged into another one answers the company that absorbed it (compare `id`). Enabled `include` contributions can enrich this response.",
    params(
        ("company_id" = String, Path, description = "Company id"),
        IncludeQuery,
    ),
    responses(
        (status = 200, description = "The company", body = Data<CompanyResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `organizations.read` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Company not found or not visible (`company_not_found`)", body = ErrorEnvelope),
        (status = 422, description = "Unknown relationship in `include` (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn get_company(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(company_id): Path<String>,
    Query(include): Query<IncludeQuery>,
) -> Result<Json<Data<Value>>, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let includes = record_includes(
        &ctx,
        "organizations.organization",
        include.include.as_deref(),
    )?;
    let company = match service::get(&ctx, &company_id).await {
        Err(
            error @ AppError::Coded {
                code: "company_not_found",
                ..
            },
        ) => match merge_survivor(&ctx, &RecordType::COMPANY, &company_id).await? {
            Some(survivor_id) => service::get(&ctx, &survivor_id).await?,
            None => return Err(error.into()),
        },
        result => result?,
    };
    let response: CompanyResponse = company.into();
    let mut values = [redacted(&state, &tenant.scope, &RecordType::COMPANY, response).await?];
    includes.enrich(&ctx, &mut values).await?;
    Ok(Json(Data::new(values[0].take())))
}

#[utoipa::path(
    patch,
    path = "/organizations/{company_id}",
    tag = TAG,
    operation_id = "companies_update",
    security(("session_cookie" = [])),
    description = "Partially updates a company (permission `organizations.update` on this company) and publishes `company.updated` with the changes. Omitted properties stay unchanged and `null` clears an optional one; `custom_fields` only touches the keys sent. Send `If-Match` with the `updated_at` you read to avoid overwriting someone else's change.",
    params(
        ("company_id" = String, Path, description = "Company id"),
        ("If-Match" = Option<String>, Header, description = "`updated_at` of the company as read (ISO-8601, quotes optional); the update fails with 409 when it changed since"),
    ),
    request_body = CompanyRequest,
    responses(
        (status = 200, description = "Company updated", body = Data<CompanyResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `organizations.update` for this company (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Company not found or not visible (`company_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "The company changed since the `If-Match` version (`version_conflict`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid system or custom fields or malformed `If-Match` (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn update_company(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    IfMatch(expected): IfMatch,
    Path(company_id): Path<String>,
    JsonBody(request): JsonBody<CompanyRequest>,
) -> Result<Json<Data<Value>>, ApiError> {
    let company = service::update(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &company_id,
        request.into_command(client.0.ip, expected),
    )
    .await?;
    Ok(Json(Data::new(
        redacted(
            &state,
            &tenant.scope,
            &RecordType::COMPANY,
            CompanyResponse::from(company),
        )
        .await?,
    )))
}

#[utoipa::path(
    delete,
    path = "/organizations/{company_id}",
    tag = TAG,
    operation_id = "companies_delete",
    security(("session_cookie" = [])),
    params(("company_id" = String, Path, description = "Company id")),
    responses(
        (status = 204, description = "Company moved to the trash (soft delete); publishes `company.deleted`. Restore it through `POST /trash/company/{id}/restore`"),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `organizations.delete` for this company (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Company not found or not visible (`company_not_found`)", body = ErrorEnvelope),
    )
)]
async fn delete_company(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(company_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    service::delete(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &company_id,
        client.0.ip,
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    post,
    path = "/organizations/{company_id}/archive",
    tag = TAG,
    operation_id = "companies_archive",
    security(("session_cookie" = [])),
    params(("company_id" = String, Path, description = "Company id")),
    responses(
        (status = 200, description = "Company archived (hidden from listings unless `archived=include|only`); publishes `company.archived`", body = Data<CompanyResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `organizations.update` for this company (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Company not found or not visible (`company_not_found`)", body = ErrorEnvelope),
    )
)]
async fn archive_company(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(company_id): Path<String>,
) -> Result<Json<Data<Value>>, ApiError> {
    let company = service::archive(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &company_id,
        client.0.ip,
    )
    .await?;
    Ok(Json(Data::new(
        redacted(
            &state,
            &tenant.scope,
            &RecordType::COMPANY,
            CompanyResponse::from(company),
        )
        .await?,
    )))
}

#[utoipa::path(
    post,
    path = "/organizations/{company_id}/unarchive",
    tag = TAG,
    operation_id = "companies_unarchive",
    security(("session_cookie" = [])),
    params(("company_id" = String, Path, description = "Company id")),
    responses(
        (status = 200, description = "Company restored from the archive; publishes `company.unarchived`", body = Data<CompanyResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `organizations.update` for this company (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Company not found or not visible (`company_not_found`)", body = ErrorEnvelope),
    )
)]
async fn unarchive_company(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(company_id): Path<String>,
) -> Result<Json<Data<Value>>, ApiError> {
    let company = service::unarchive(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &company_id,
        client.0.ip,
    )
    .await?;
    Ok(Json(Data::new(
        redacted(
            &state,
            &tenant.scope,
            &RecordType::COMPANY,
            CompanyResponse::from(company),
        )
        .await?,
    )))
}

#[derive(Debug, Default, Deserialize, ToSchema)]
pub(crate) struct CompanyRequest {
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>, example = "Acme Inc.")]
    pub(crate) name: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>, example = "acme.com.br")]
    pub(crate) domain: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>, example = "https://www.acme.com.br")]
    pub(crate) website: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>)]
    pub(crate) industry: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>, example = "size_11_50")]
    pub(crate) size_range: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>, example = "(11) 3333-4444")]
    pub(crate) phone: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>)]
    pub(crate) email: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>)]
    pub(crate) owner_id: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>)]
    pub(crate) address_street: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>)]
    pub(crate) address_number: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>)]
    pub(crate) address_complement: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>)]
    pub(crate) address_district: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>)]
    pub(crate) address_city: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>)]
    pub(crate) address_state: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>)]
    pub(crate) address_postal_code: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>, example = "BR")]
    pub(crate) address_country: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>)]
    pub(crate) description: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>)]
    pub(crate) source: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>)]
    pub(crate) parent_company_id: Option<Option<String>>,
    #[serde(default)]
    #[schema(value_type = Option<Object>)]
    pub(crate) custom_fields: Option<Map<String, Value>>,
    #[serde(flatten)]
    pub(crate) contributed: Map<String, Value>,
}

impl CompanyRequest {
    pub(crate) fn into_command(
        self,
        ip: Option<String>,
        expected_updated_at: Option<DateTime<Utc>>,
    ) -> SaveCompanyCommand {
        SaveCompanyCommand {
            input: CompanyInput {
                name: self.name,
                domain: self.domain,
                website: self.website,
                industry: self.industry,
                size_range: self.size_range,
                phone: self.phone,
                email: self.email,
                owner_id: self.owner_id,
                address: AddressInput {
                    street: self.address_street,
                    number: self.address_number,
                    complement: self.address_complement,
                    district: self.address_district,
                    city: self.address_city,
                    state: self.address_state,
                    postal_code: self.address_postal_code,
                    country: self.address_country,
                },
                description: self.description,
                source: self.source,
                parent_company_id: self.parent_company_id,
                contributed: self.contributed,
            },
            custom_fields: self.custom_fields.unwrap_or_default(),
            ip,
            expected_updated_at,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct CompanyResponse {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) domain: Option<String>,
    pub(crate) website: Option<String>,
    pub(crate) industry: Option<String>,
    pub(crate) size_range: Option<String>,
    pub(crate) phone: Option<String>,
    pub(crate) email: Option<String>,
    pub(crate) owner_id: Option<String>,
    pub(crate) address_street: Option<String>,
    pub(crate) address_number: Option<String>,
    pub(crate) address_complement: Option<String>,
    pub(crate) address_district: Option<String>,
    pub(crate) address_city: Option<String>,
    pub(crate) address_state: Option<String>,
    pub(crate) address_postal_code: Option<String>,
    pub(crate) address_country: Option<String>,
    pub(crate) description: Option<String>,
    pub(crate) source: Option<String>,
    pub(crate) parent_company_id: Option<String>,
    #[schema(example = "2026-10-20T13:45:00Z")]
    pub(crate) last_contacted_at: Option<String>,
    #[schema(example = "2026-10-20T13:45:00Z")]
    pub(crate) last_email_received_at: Option<String>,
    #[schema(example = "2026-10-20T13:45:00Z")]
    pub(crate) last_email_sent_at: Option<String>,
    pub(crate) interaction_count: i64,
    pub(crate) last_contacted_by: Option<String>,
    pub(crate) created_by: Option<String>,
    #[schema(example = "2026-10-20T13:45:00Z")]
    pub(crate) archived_at: Option<String>,
    #[schema(example = "2026-10-20T13:45:00Z")]
    pub(crate) created_at: String,
    #[schema(example = "2026-10-20T13:45:00Z")]
    pub(crate) updated_at: String,
    #[schema(value_type = Object, example = json!({ "customer_tier": "gold" }))]
    pub(crate) custom_fields: Map<String, Value>,
    pub(crate) owner: Option<OwnerResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) possible_duplicates: Option<Vec<DuplicateMatchResponse>>,
    #[serde(flatten)]
    pub(crate) contributed: Map<String, Value>,
}

impl From<CompanyDetails> for CompanyResponse {
    fn from(details: CompanyDetails) -> Self {
        let company = details.view.company;
        let data = company.data;
        let address = data.address;
        Self {
            id: company.id.to_string(),
            name: data.name,
            domain: data.domain,
            website: data.website,
            industry: data.industry,
            size_range: data.size_range,
            phone: data.phone,
            email: data.email,
            owner_id: data.owner_id.map(|id| id.into_inner()),
            address_street: address.street,
            address_number: address.number,
            address_complement: address.complement,
            address_district: address.district,
            address_city: address.city,
            address_state: address.state,
            address_postal_code: address.postal_code,
            address_country: address.country,
            description: data.description,
            source: data.source,
            parent_company_id: data.parent_company_id.map(|id| id.to_string()),
            last_contacted_at: company.engagement.last_contacted_at.map(timestamp),
            last_email_received_at: company.engagement.last_email_received_at.map(timestamp),
            last_email_sent_at: company.engagement.last_email_sent_at.map(timestamp),
            interaction_count: company.engagement.interaction_count,
            last_contacted_by: company
                .engagement
                .last_contacted_by
                .map(|id| id.into_inner()),
            created_by: company.created_by.map(|id| id.into_inner()),
            archived_at: company.archived_at.map(timestamp),
            created_at: timestamp(company.created_at),
            updated_at: timestamp(company.updated_at),
            custom_fields: details.custom_fields,
            owner: details.view.owner.map(Into::into),
            possible_duplicates: None,
            contributed: details.view.contributed,
        }
    }
}

pub(crate) struct CompanyHandler;

#[async_trait]
impl RecordTypeHandler for CompanyHandler {
    fn durable_mutations(&self) -> bool {
        true
    }

    fn record_type(&self) -> &str {
        "organizations.organization"
    }

    async fn create(&self, ctx: &RecordCtx<'_>, values: Value) -> Result<Value, AppError> {
        let command = values_request::<CompanyRequest>(values)?.into_command(None, None);
        let details =
            service::create(&Ctx::from_state(ctx.state, ctx.scope).await?, command).await?;
        redacted(
            ctx.state,
            ctx.scope,
            &RecordType::COMPANY,
            CompanyResponse::from(details),
        )
        .await
    }

    async fn create_in_tx(
        &self,
        ctx: &RecordCtx<'_>,
        tx: &mut MySqlTx,
        values: Value,
    ) -> Result<PendingCommit, AppError> {
        let command = values_request::<CompanyRequest>(values)?.into_command(None, None);
        service::create_in_tx(&Ctx::from_state(ctx.state, ctx.scope).await?, tx, command).await
    }

    async fn update(
        &self,
        ctx: &RecordCtx<'_>,
        id: Uuid,
        values: Value,
    ) -> Result<Value, AppError> {
        let command = values_request::<CompanyRequest>(values)?.into_command(None, None);
        let details = service::update(
            &Ctx::from_state(ctx.state, ctx.scope).await?,
            &id.to_string(),
            command,
        )
        .await?;
        redacted(
            ctx.state,
            ctx.scope,
            &RecordType::COMPANY,
            CompanyResponse::from(details),
        )
        .await
    }

    async fn update_in_tx(
        &self,
        ctx: &RecordCtx<'_>,
        tx: &mut MySqlTx,
        id: Uuid,
        values: Value,
    ) -> Result<Option<PendingCommit>, AppError> {
        let command = values_request::<CompanyRequest>(values)?.into_command(None, None);
        let (_, pending) = service::update_in_tx(
            &Ctx::from_state(ctx.state, ctx.scope).await?,
            tx,
            &id.to_string(),
            command,
        )
        .await?;
        Ok(pending)
    }

    async fn delete(&self, ctx: &RecordCtx<'_>, id: Uuid) -> Result<(), AppError> {
        service::delete(
            &Ctx::from_state(ctx.state, ctx.scope).await?,
            &id.to_string(),
            None,
        )
        .await
    }

    async fn load(&self, ctx: &RecordCtx<'_>, id: Uuid) -> Result<Option<Value>, AppError> {
        let module_ctx = Ctx::from_state(ctx.state, ctx.scope).await?;
        let Some(details) = optional_record(service::get(&module_ctx, &id.to_string()).await)?
        else {
            return Ok(None);
        };
        Ok(Some(
            redacted(
                ctx.state,
                ctx.scope,
                &RecordType::COMPANY,
                CompanyResponse::from(details),
            )
            .await?,
        ))
    }

    async fn archive(&self, ctx: &RecordCtx<'_>, id: Uuid) -> Result<(), AppError> {
        service::archive(
            &Ctx::from_state(ctx.state, ctx.scope).await?,
            &id.to_string(),
            None,
        )
        .await
        .map(|_| ())
    }

    async fn restore(&self, ctx: &RecordCtx<'_>, id: Uuid) -> Result<(), AppError> {
        service::unarchive(
            &Ctx::from_state(ctx.state, ctx.scope).await?,
            &id.to_string(),
            None,
        )
        .await
        .map(|_| ())
    }
}
