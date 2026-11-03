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

use crate::domain::{
    CompanyContactLink, CompanyReference, ContactCompanyLink, ContactCompanyRole, ContactDetails,
    ContactInput, LinkCompanyCommand, SaveContactCommand,
};
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
use aphel_kernel::api::routes::v1::field_permissions::{
    field_redaction, nested_redactor, redacted, redacted_items,
};
use aphel_kernel::api::routes::v1::members::window;
use aphel_kernel::api::state::AppState;
use aphel_kernel::application::shared::RecordType;
use aphel_kernel::database::transaction::MySqlTx;
use aphel_kernel::domain::access::AccessScope;
use aphel_kernel::errors::AppError;
use aphel_kernel::records::{
    AddressInput, merge_survivor, possible_duplicates, serialize_nested_records,
};
use aphel_kernel::records::{PendingCommit, RecordCtx, values_request};
use tags::dto::TagSummary;
use tags::http::{RECORD_INCLUDES, record_tag_map};

pub(crate) const TAG: &str = "people";

pub(crate) fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list_contacts, create_contact))
        .routes(routes!(get_contact, update_contact, delete_contact))
        .routes(routes!(archive_contact))
        .routes(routes!(unarchive_contact))
        .routes(routes!(list_people_organizations, link_company))
        .routes(routes!(unlink_company))
        .routes(routes!(list_company_contacts))
}

#[utoipa::path(
    get,
    path = "/people",
    tag = TAG,
    operation_id = "contacts_list",
    description = "Lists the people visible to the user (permission `people.read`, limited by its record scope). Filters accept the JSON AST in `filter` or simple conditions as `filter[field][operator]=value` (`filter[email][contains]=acme`, `filter[owner_id][in]=a,b`, `filter[company_id]=<id>` means `eq`), combined with `and`. `sort=field,-other` (up to 3); without `sort`, newest first. `search` (or its alias `q`) matches name, email, phone and mobile; phone terms are normalized to E.164. `archived=exclude|include|only` (default `exclude`). The owner and the primary company are always embedded; `include=tags` also embeds the tags of each contact.",
    params(RecordQueryParams, QuickSearchParams, IncludeQuery, PaginationQuery),
    responses(
        (status = 200, description = "Page of people with their custom fields", body = Page<ContactResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `people.read` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid filter, sort, search, include or pagination (`validation_error` with `details.fields`)", body = ErrorEnvelope),
    )
)]
async fn list_contacts(
    State(state): State<AppState>,
    tenant: AuthContext,
    RecordListQuery(mut query): RecordListQuery,
    Query(quick): Query<QuickSearchParams>,
    Query(include): Query<IncludeQuery>,
    pagination: Pagination,
) -> Result<Json<Page<Value>>, ApiError> {
    let includes = include.parse(RECORD_INCLUDES)?;
    quick.merge_into(&mut query.search);
    let listing = service::list(&Ctx::from_state(&state, &tenant.scope).await?, query).await?;
    let mut items: Vec<ContactResponse> = listing.items.into_iter().map(Into::into).collect();
    if includes.contains("tags") {
        let ids: Vec<String> = items.iter().map(|item| item.id.clone()).collect();
        let mut tags = record_tag_map(&state, &tenant.scope, "people.person", &ids).await?;
        for item in &mut items {
            item.tags = Some(tags.remove(&item.id).unwrap_or_default());
        }
    }
    Ok(Json(Page::new(
        redacted_items(&state, &tenant.scope, &RecordType::CONTACT, items).await?,
        listing.total,
        pagination,
    )))
}

#[utoipa::path(
    post,
    path = "/people",
    tag = TAG,
    operation_id = "contacts_create",
    description = "Creates a contact (permission `people.create`) and publishes `contact.created`. Email is lowercased and phones normalized to E.164; `name` is derived from first and last names; the owner defaults to the current user. Custom fields go in `custom_fields`. `possible_duplicates` (only in this response, empty when none) lists visible people that the duplicate rules match (a warning, never an error); those pairs also enter the duplicate queue.",
    request_body = ContactRequest,
    responses(
        (status = 201, description = "Contact created", body = Data<ContactResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `people.create` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid system or custom fields (`validation_error`, fields such as `email` or `custom_fields.<key>`)", body = ErrorEnvelope),
    )
)]
async fn create_contact(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    JsonBody(request): JsonBody<ContactRequest>,
) -> Result<(StatusCode, Json<Data<Value>>), ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let contact = service::create(&ctx, request.into_command(client.0.ip, None)).await?;
    let mut response: ContactResponse = contact.into();
    let matches = possible_duplicates(&ctx, &RecordType::CONTACT, &response.id).await;
    response.possible_duplicates = Some(matches.into_iter().map(Into::into).collect());
    Ok((
        StatusCode::CREATED,
        Json(Data::new(
            redacted(&state, &tenant.scope, &RecordType::CONTACT, response).await?,
        )),
    ))
}

#[utoipa::path(
    get,
    path = "/people/{contact_id}",
    tag = TAG,
    operation_id = "contacts_get",
    security(("session_cookie" = [])),
    description = "Returns a contact with its custom fields, owner and primary company (permission `people.read`). The id of a contact merged into another one answers the contact that absorbed it (compare `id`). `include=tags` embeds its tags.",
    params(
        ("contact_id" = String, Path, description = "Contact id"),
        IncludeQuery,
    ),
    responses(
        (status = 200, description = "The contact", body = Data<ContactResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `people.read` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Contact not found or not visible (`contact_not_found`)", body = ErrorEnvelope),
        (status = 422, description = "Unknown relationship in `include` (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn get_contact(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(contact_id): Path<String>,
    Query(include): Query<IncludeQuery>,
) -> Result<Json<Data<Value>>, ApiError> {
    let includes = include.parse(RECORD_INCLUDES)?;
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let contact = match service::get(&ctx, &contact_id).await {
        Err(
            error @ AppError::Coded {
                code: "contact_not_found",
                ..
            },
        ) => match merge_survivor(&ctx, &RecordType::CONTACT, &contact_id).await? {
            Some(survivor_id) => service::get(&ctx, &survivor_id).await?,
            None => return Err(error.into()),
        },
        result => result?,
    };
    let mut response: ContactResponse = contact.into();
    if includes.contains("tags") {
        let mut tags = record_tag_map(
            &state,
            &tenant.scope,
            "people.person",
            std::slice::from_ref(&response.id),
        )
        .await?;
        response.tags = Some(tags.remove(&response.id).unwrap_or_default());
    }
    Ok(Json(Data::new(
        redacted(&state, &tenant.scope, &RecordType::CONTACT, response).await?,
    )))
}

#[utoipa::path(
    patch,
    path = "/people/{contact_id}",
    tag = TAG,
    operation_id = "contacts_update",
    security(("session_cookie" = [])),
    description = "Partially updates a contact (permission `people.update` on this contact) and publishes `contact.updated` with the changes. Omitted properties stay unchanged and `null` clears an optional one; `custom_fields` only touches the keys sent. Send `If-Match` with the `updated_at` you read to avoid overwriting someone else's change.",
    params(
        ("contact_id" = String, Path, description = "Contact id"),
        ("If-Match" = Option<String>, Header, description = "`updated_at` of the contact as read (ISO-8601, quotes optional); the update fails with 409 when it changed since"),
    ),
    request_body = ContactRequest,
    responses(
        (status = 200, description = "Contact updated", body = Data<ContactResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `people.update` for this contact (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Contact not found or not visible (`contact_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "The contact changed since the `If-Match` version (`version_conflict`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid system or custom fields or malformed `If-Match` (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn update_contact(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    IfMatch(expected): IfMatch,
    Path(contact_id): Path<String>,
    JsonBody(request): JsonBody<ContactRequest>,
) -> Result<Json<Data<Value>>, ApiError> {
    let contact = service::update(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &contact_id,
        request.into_command(client.0.ip, expected),
    )
    .await?;
    Ok(Json(Data::new(
        redacted(
            &state,
            &tenant.scope,
            &RecordType::CONTACT,
            ContactResponse::from(contact),
        )
        .await?,
    )))
}

#[utoipa::path(
    delete,
    path = "/people/{contact_id}",
    tag = TAG,
    operation_id = "contacts_delete",
    security(("session_cookie" = [])),
    params(("contact_id" = String, Path, description = "Contact id")),
    responses(
        (status = 204, description = "Contact moved to the trash (soft delete); publishes `contact.deleted`. Restore it through `POST /trash/contact/{id}/restore`"),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `people.delete` for this contact (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Contact not found or not visible (`contact_not_found`)", body = ErrorEnvelope),
    )
)]
async fn delete_contact(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(contact_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    service::delete(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &contact_id,
        client.0.ip,
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    post,
    path = "/people/{contact_id}/archive",
    tag = TAG,
    operation_id = "contacts_archive",
    security(("session_cookie" = [])),
    params(("contact_id" = String, Path, description = "Contact id")),
    responses(
        (status = 200, description = "Contact archived (hidden from listings unless `archived=include|only`); publishes `contact.archived`", body = Data<ContactResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `people.update` for this contact (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Contact not found or not visible (`contact_not_found`)", body = ErrorEnvelope),
    )
)]
async fn archive_contact(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(contact_id): Path<String>,
) -> Result<Json<Data<Value>>, ApiError> {
    let contact = service::archive(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &contact_id,
        client.0.ip,
    )
    .await?;
    Ok(Json(Data::new(
        redacted(
            &state,
            &tenant.scope,
            &RecordType::CONTACT,
            ContactResponse::from(contact),
        )
        .await?,
    )))
}

#[utoipa::path(
    post,
    path = "/people/{contact_id}/unarchive",
    tag = TAG,
    operation_id = "contacts_unarchive",
    security(("session_cookie" = [])),
    params(("contact_id" = String, Path, description = "Contact id")),
    responses(
        (status = 200, description = "Contact restored from the archive; publishes `contact.unarchived`", body = Data<ContactResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `people.update` for this contact (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Contact not found or not visible (`contact_not_found`)", body = ErrorEnvelope),
    )
)]
async fn unarchive_contact(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(contact_id): Path<String>,
) -> Result<Json<Data<Value>>, ApiError> {
    let contact = service::unarchive(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &contact_id,
        client.0.ip,
    )
    .await?;
    Ok(Json(Data::new(
        redacted(
            &state,
            &tenant.scope,
            &RecordType::CONTACT,
            ContactResponse::from(contact),
        )
        .await?,
    )))
}

#[utoipa::path(
    get,
    path = "/people/{contact_id}/organizations",
    tag = TAG,
    operation_id = "contacts_list_companies",
    security(("session_cookie" = [])),
    params(
        ("contact_id" = String, Path, description = "Contact id"),
        PaginationQuery,
    ),
    responses(
        (status = 200, description = "Companies of the contact: the primary one (`people.company_id`) first, then the additional links with their roles. Only organizations the user can read are listed", body = Page<ContactCompanyLinkResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `people.read` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Contact not found or not visible (`contact_not_found`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid pagination (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn list_people_organizations(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(contact_id): Path<String>,
    pagination: Pagination,
) -> Result<Json<Page<Value>>, ApiError> {
    let listing = service::list_companies(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &contact_id,
        window(pagination),
    )
    .await?;
    let redactor = nested_redactor(
        "company",
        field_redaction(&state, &tenant.scope, &RecordType::COMPANY).await?,
    );
    let items: Vec<ContactCompanyLinkResponse> =
        listing.items.into_iter().map(Into::into).collect();
    let items = serialized_nested(
        &state,
        &tenant.scope,
        &RecordType::COMPANY,
        "company",
        items,
    )
    .await?;
    Ok(Json(Page::new(
        redactor.items(items)?,
        listing.total,
        pagination,
    )))
}

#[utoipa::path(
    post,
    path = "/people/{contact_id}/organizations",
    tag = TAG,
    operation_id = "contacts_link_company",
    security(("session_cookie" = [])),
    params(("contact_id" = String, Path, description = "Contact id")),
    request_body = LinkCompanyRequest,
    responses(
        (status = 201, description = "Additional company linked to the contact; publishes `contact.company_linked`", body = Data<ContactCompanyLinkResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `people.update` for this contact (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Contact not found or not visible (`contact_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "The company is already linked or is the primary company (`already_linked`)", body = ErrorEnvelope),
        (status = 422, description = "Unknown or invisible company, or invalid role (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn link_company(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(contact_id): Path<String>,
    JsonBody(request): JsonBody<LinkCompanyRequest>,
) -> Result<(StatusCode, Json<Data<ContactCompanyLinkResponse>>), ApiError> {
    let link = service::link_company(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &contact_id,
        request.into_command(client.0.ip),
    )
    .await?;
    Ok((StatusCode::CREATED, Json(Data::new(link.into()))))
}

#[utoipa::path(
    delete,
    path = "/people/{contact_id}/organizations/{company_id}",
    tag = TAG,
    operation_id = "contacts_unlink_company",
    security(("session_cookie" = [])),
    params(
        ("contact_id" = String, Path, description = "Contact id"),
        ("company_id" = String, Path, description = "Company id"),
    ),
    responses(
        (status = 204, description = "Additional link removed (the primary company is changed through `company_id`); publishes `contact.company_unlinked`"),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `people.update` for this contact (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Contact not found (`contact_not_found`) or not linked (`link_not_found`)", body = ErrorEnvelope),
    )
)]
async fn unlink_company(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path((contact_id, company_id)): Path<(String, String)>,
) -> Result<StatusCode, ApiError> {
    service::unlink_company(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &contact_id,
        &company_id,
        client.0.ip,
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    get,
    path = "/organizations/{company_id}/people",
    tag = TAG,
    operation_id = "companies_list_contacts",
    security(("session_cookie" = [])),
    params(
        ("company_id" = String, Path, description = "Company id"),
        PaginationQuery,
    ),
    responses(
        (status = 200, description = "Contacts of the company the user can read: those whose primary company it is first, then the additional links", body = Page<CompanyContactResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 403, description = "Missing `organizations.read` or `people.read` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Company not found or not visible (`company_not_found`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid pagination (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn list_company_contacts(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(company_id): Path<String>,
    pagination: Pagination,
) -> Result<Json<Page<Value>>, ApiError> {
    let listing = service::list_company_contacts(
        &Ctx::from_state(&state, &tenant.scope).await?,
        &company_id,
        window(pagination),
    )
    .await?;
    let redactor = nested_redactor(
        "contact",
        field_redaction(&state, &tenant.scope, &RecordType::CONTACT).await?,
    );
    let items: Vec<CompanyContactResponse> = listing.items.into_iter().map(Into::into).collect();
    let items = serialized_nested(
        &state,
        &tenant.scope,
        &RecordType::CONTACT,
        "contact",
        items,
    )
    .await?;
    Ok(Json(Page::new(
        redactor.items(items)?,
        listing.total,
        pagination,
    )))
}

async fn serialized_nested<T: Serialize>(
    state: &AppState,
    scope: &AccessScope,
    record_type: &RecordType,
    key: &str,
    items: Vec<T>,
) -> Result<Vec<Value>, AppError> {
    let items = items
        .into_iter()
        .map(serde_json::to_value)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| AppError::Internal(format!("record could not be serialized: {error}")))?;
    match &state.db {
        Some(pool) => serialize_nested_records(pool, scope, record_type, key, items).await,
        None => Ok(items),
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[schema(as = ContactCompanyRole)]
pub(crate) enum ContactCompanyRoleSchema {
    DecisionMaker,
    Buyer,
    Influencer,
    User,
    Other,
}

impl From<ContactCompanyRole> for ContactCompanyRoleSchema {
    fn from(role: ContactCompanyRole) -> Self {
        match role {
            ContactCompanyRole::DecisionMaker => Self::DecisionMaker,
            ContactCompanyRole::Buyer => Self::Buyer,
            ContactCompanyRole::Influencer => Self::Influencer,
            ContactCompanyRole::User => Self::User,
            ContactCompanyRole::Other => Self::Other,
        }
    }
}

impl From<ContactCompanyRoleSchema> for ContactCompanyRole {
    fn from(role: ContactCompanyRoleSchema) -> Self {
        match role {
            ContactCompanyRoleSchema::DecisionMaker => Self::DecisionMaker,
            ContactCompanyRoleSchema::Buyer => Self::Buyer,
            ContactCompanyRoleSchema::Influencer => Self::Influencer,
            ContactCompanyRoleSchema::User => Self::User,
            ContactCompanyRoleSchema::Other => Self::Other,
        }
    }
}

#[derive(Debug, Default, Deserialize, ToSchema)]
pub(crate) struct ContactRequest {
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>, example = "Ana")]
    pub(crate) first_name: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>, example = "Souza")]
    pub(crate) last_name: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>, example = "ana@acme.com.br")]
    pub(crate) email: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>, example = "(11) 3333-4444")]
    pub(crate) phone: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>, example = "(11) 98888-7777")]
    pub(crate) mobile: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>)]
    pub(crate) job_title: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>)]
    pub(crate) company_id: Option<Option<String>>,
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
    pub(crate) source: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>)]
    pub(crate) notes: Option<Option<String>>,
    #[serde(default)]
    #[schema(value_type = Option<Object>)]
    pub(crate) custom_fields: Option<Map<String, Value>>,
    #[serde(flatten)]
    pub(crate) contributed: Map<String, Value>,
}

impl ContactRequest {
    pub(crate) fn into_command(
        self,
        ip: Option<String>,
        expected_updated_at: Option<DateTime<Utc>>,
    ) -> SaveContactCommand {
        SaveContactCommand {
            input: ContactInput {
                first_name: self.first_name,
                last_name: self.last_name,
                email: self.email,
                phone: self.phone,
                mobile: self.mobile,
                job_title: self.job_title,
                company_id: self.company_id,
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
                source: self.source,
                notes: self.notes,
                contributed: self.contributed,
            },
            custom_fields: self.custom_fields.unwrap_or_default(),
            ip,
            expected_updated_at,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct CompanyReferenceResponse {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) domain: Option<String>,
}

impl From<CompanyReference> for CompanyReferenceResponse {
    fn from(company: CompanyReference) -> Self {
        Self {
            id: company.id.to_string(),
            name: company.name,
            domain: company.domain,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ContactResponse {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) first_name: String,
    pub(crate) last_name: Option<String>,
    pub(crate) email: Option<String>,
    pub(crate) phone: Option<String>,
    pub(crate) mobile: Option<String>,
    pub(crate) job_title: Option<String>,
    pub(crate) company_id: Option<String>,
    pub(crate) owner_id: Option<String>,
    pub(crate) address_street: Option<String>,
    pub(crate) address_number: Option<String>,
    pub(crate) address_complement: Option<String>,
    pub(crate) address_district: Option<String>,
    pub(crate) address_city: Option<String>,
    pub(crate) address_state: Option<String>,
    pub(crate) address_postal_code: Option<String>,
    pub(crate) address_country: Option<String>,
    pub(crate) source: Option<String>,
    pub(crate) notes: Option<String>,
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
    #[schema(value_type = Object, example = json!({ "preferred_channel": "whatsapp" }))]
    pub(crate) custom_fields: Map<String, Value>,
    pub(crate) owner: Option<OwnerResponse>,
    pub(crate) company: Option<CompanyReferenceResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) tags: Option<Vec<TagSummary>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) possible_duplicates: Option<Vec<DuplicateMatchResponse>>,
    #[serde(flatten)]
    pub(crate) contributed: Map<String, Value>,
}

impl From<ContactDetails> for ContactResponse {
    fn from(details: ContactDetails) -> Self {
        let contact = details.view.contact;
        let name = contact.full_name();
        let data = contact.data;
        let address = data.address;
        Self {
            id: contact.id.to_string(),
            name,
            first_name: data.first_name,
            last_name: data.last_name,
            email: data.email,
            phone: data.phone,
            mobile: data.mobile,
            job_title: data.job_title,
            company_id: data.company_id.map(|id| id.to_string()),
            owner_id: data.owner_id.map(|id| id.into_inner()),
            address_street: address.street,
            address_number: address.number,
            address_complement: address.complement,
            address_district: address.district,
            address_city: address.city,
            address_state: address.state,
            address_postal_code: address.postal_code,
            address_country: address.country,
            source: data.source,
            notes: data.notes,
            last_contacted_at: contact.engagement.last_contacted_at.map(timestamp),
            last_email_received_at: contact.engagement.last_email_received_at.map(timestamp),
            last_email_sent_at: contact.engagement.last_email_sent_at.map(timestamp),
            interaction_count: contact.engagement.interaction_count,
            last_contacted_by: contact
                .engagement
                .last_contacted_by
                .map(|id| id.into_inner()),
            created_by: contact.created_by.map(|id| id.into_inner()),
            archived_at: contact.archived_at.map(timestamp),
            created_at: timestamp(contact.created_at),
            updated_at: timestamp(contact.updated_at),
            custom_fields: details.custom_fields,
            owner: details.view.owner.map(Into::into),
            company: details.view.company.map(Into::into),
            tags: None,
            possible_duplicates: None,
            contributed: details.view.contributed,
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub(crate) struct LinkCompanyRequest {
    pub(crate) company_id: String,
    pub(crate) role: Option<ContactCompanyRoleSchema>,
}

impl LinkCompanyRequest {
    pub(crate) fn into_command(self, ip: Option<String>) -> LinkCompanyCommand {
        LinkCompanyCommand {
            company_id: self.company_id,
            role: self.role.map(Into::into),
            ip,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct ContactCompanyLinkResponse {
    pub(crate) link_id: Option<String>,
    pub(crate) company: CompanyReferenceResponse,
    pub(crate) role: Option<ContactCompanyRoleSchema>,
    pub(crate) is_primary: bool,
    pub(crate) created_at: Option<String>,
}

impl From<ContactCompanyLink> for ContactCompanyLinkResponse {
    fn from(link: ContactCompanyLink) -> Self {
        Self {
            link_id: link.link_id.map(|id| id.to_string()),
            company: link.company.into(),
            role: link.role.map(Into::into),
            is_primary: link.is_primary,
            created_at: link.created_at.map(timestamp),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct ContactSummaryResponse {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) email: Option<String>,
    pub(crate) phone: Option<String>,
    pub(crate) mobile: Option<String>,
    pub(crate) job_title: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct CompanyContactResponse {
    pub(crate) link_id: Option<String>,
    pub(crate) contact: ContactSummaryResponse,
    pub(crate) role: Option<ContactCompanyRoleSchema>,
    pub(crate) is_primary: bool,
}

impl From<CompanyContactLink> for CompanyContactResponse {
    fn from(link: CompanyContactLink) -> Self {
        let contact = link.contact;
        Self {
            link_id: link.link_id.map(|id| id.to_string()),
            contact: ContactSummaryResponse {
                id: contact.id.to_string(),
                name: contact.name,
                email: contact.email,
                phone: contact.phone,
                mobile: contact.mobile,
                job_title: contact.job_title,
            },
            role: link.role.map(Into::into),
            is_primary: link.is_primary,
        }
    }
}

fn contact_body(values: Value) -> Value {
    let mut body = match values {
        Value::Object(map) => map,
        _ => Map::new(),
    };
    let has_first_name = body.get("first_name").is_some_and(|value| !value.is_null());
    if !has_first_name
        && let Some(name) = body
            .remove("name")
            .and_then(|name| name.as_str().map(str::to_owned))
    {
        if body.get("last_name").is_some_and(|value| !value.is_null()) {
            body.insert("first_name".to_owned(), Value::String(name));
        } else {
            let (first, last) = split_full_name(&name);
            body.insert("first_name".to_owned(), Value::String(first));
            if let Some(last) = last {
                body.insert("last_name".to_owned(), Value::String(last));
            }
        }
    } else {
        body.remove("name");
    }
    Value::Object(body)
}

pub(crate) struct ContactBehavior;

#[async_trait]
impl aphel_kernel::records::EntityWriteBehavior for ContactBehavior {
    async fn create(
        &self,
        ctx: &RecordCtx<'_>,
        tx: &mut MySqlTx,
        values: Value,
    ) -> Result<PendingCommit, AppError> {
        let command =
            values_request::<ContactRequest>(contact_body(values))?.into_command(None, None);
        let created =
            service::create_in_tx(&Ctx::from_state(ctx.state, ctx.scope).await?, tx, command)
                .await?;
        Ok(created.into_pending())
    }

    async fn update(
        &self,
        ctx: &RecordCtx<'_>,
        tx: &mut MySqlTx,
        id: Uuid,
        values: Value,
    ) -> Result<Option<PendingCommit>, AppError> {
        let command =
            values_request::<ContactRequest>(contact_body(values))?.into_command(None, None);
        let (_, pending) = service::update_in_tx(
            &Ctx::from_state(ctx.state, ctx.scope).await?,
            tx,
            &id.to_string(),
            command,
        )
        .await?;
        Ok(pending)
    }
}

fn split_full_name(name: &str) -> (String, Option<String>) {
    let name = name.trim();
    match name.split_once(char::is_whitespace) {
        Some((first, rest)) if !rest.trim().is_empty() => {
            (first.to_owned(), Some(rest.trim().to_owned()))
        }
        _ => (name.to_owned(), None),
    }
}
