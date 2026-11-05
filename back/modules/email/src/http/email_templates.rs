use axum::Json;
use axum::extract::rejection::QueryRejection;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::dto::email_outbound::{
    CreateEmailTemplateRequest, EmailTemplateResponse, EmailTemplateVariableResponse,
    EmailTemplateVariablesQuery, EmailTemplatesQuery, UpdateEmailTemplateRequest,
};
use crate::service;
use aphel_kernel::Ctx;
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::api::dto::common::{Data, ErrorEnvelope};
use aphel_sdk::platform::api::error::ApiError;
use aphel_sdk::platform::api::extractors::{AuthContext, ClientInfo, JsonBody};
use aphel_sdk::platform::api::state::AppState;

pub(crate) const TAG: &str = "email templates";

pub(crate) fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list_templates, create_template))
        .routes(routes!(template_variables))
        .routes(routes!(get_template, update_template, delete_template))
}

#[utoipa::path(
    get,
    path = "/email-templates",
    tag = TAG,
    operation_id = "email_templates_list",
    security(("session_cookie" = [])),
    params(EmailTemplatesQuery),
    responses(
        (status = 200, description = "Organization templates and the user's personal templates (`scope=all`, default), only one of them (`organization`, `personal`); `record_type` keeps the templates of that type and the generic ones", body = Data<Vec<EmailTemplateResponse>>),
        (status = 401, description = "Missing or invalid credentials", body = ErrorEnvelope),
        (status = 403, description = "Missing `emails.send` (`permission_denied`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid filter (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn list_templates(
    State(state): State<AppState>,
    tenant: AuthContext,
    query: Result<Query<EmailTemplatesQuery>, QueryRejection>,
) -> Result<Json<Data<Vec<EmailTemplateResponse>>>, ApiError> {
    let Query(params) = query.map_err(|rejection| AppError::Validation(rejection.body_text()))?;
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let templates =
        service::templates::list(&ctx, params.scope.as_deref(), params.record_type.as_deref())
            .await?;
    Ok(Json(Data::new(
        templates.into_iter().map(Into::into).collect(),
    )))
}

#[utoipa::path(
    post,
    path = "/email-templates",
    tag = TAG,
    operation_id = "email_templates_create",
    description = "Creates an organization template (`scope = organization`, needs `settings.manage`) or a personal one (default; only its owner sees it). `key` is stable (snake_case, unique among organization templates or per owner) and is generated from the name when omitted. The HTML is sanitized. Variables use `{{contact.first_name}}`, `{{company.name}}`, `{{deal.title}}`, `{{record.<field>}}`, `{{user.display_name}}`, `{{user.email}}`.",
    security(("session_cookie" = [])),
    request_body = CreateEmailTemplateRequest,
    responses(
        (status = 201, description = "Template created (`email_template.created` with audit)", body = Data<EmailTemplateResponse>),
        (status = 401, description = "Missing or invalid credentials", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` or `emails.send` (`permission_denied`)", body = ErrorEnvelope),
        (status = 409, description = "Key already used (`email_template_key_taken`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid input (`validation_error` with `details.fields`)", body = ErrorEnvelope),
    )
)]
async fn create_template(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    JsonBody(request): JsonBody<CreateEmailTemplateRequest>,
) -> Result<(StatusCode, Json<Data<EmailTemplateResponse>>), ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let template = service::templates::create(&ctx, request.into_command(client.0.ip)).await?;
    Ok((StatusCode::CREATED, Json(Data::new(template.into()))))
}

#[utoipa::path(
    get,
    path = "/email-templates/variables",
    tag = TAG,
    operation_id = "email_templates_variables",
    security(("session_cookie" = [])),
    params(EmailTemplateVariablesQuery),
    responses(
        (status = 200, description = "Variables available for the record type (its fields and those of related contact/company), limited to the fields the user can read, plus the sender variables", body = Data<Vec<EmailTemplateVariableResponse>>),
        (status = 401, description = "Missing or invalid credentials", body = ErrorEnvelope),
        (status = 403, description = "Missing `emails.send` (`permission_denied`)", body = ErrorEnvelope),
        (status = 422, description = "Unsupported record type (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn template_variables(
    State(state): State<AppState>,
    tenant: AuthContext,
    query: Result<Query<EmailTemplateVariablesQuery>, QueryRejection>,
) -> Result<Json<Data<Vec<EmailTemplateVariableResponse>>>, ApiError> {
    let Query(params) = query.map_err(|rejection| AppError::Validation(rejection.body_text()))?;
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let variables = service::templates::variables(&ctx, params.record_type.as_deref()).await?;
    Ok(Json(Data::new(
        variables.into_iter().map(Into::into).collect(),
    )))
}

#[utoipa::path(
    get,
    path = "/email-templates/{template_id}",
    tag = TAG,
    operation_id = "email_templates_get",
    security(("session_cookie" = [])),
    params(("template_id" = String, Path, description = "Template id")),
    responses(
        (status = 200, description = "The template", body = Data<EmailTemplateResponse>),
        (status = 401, description = "Missing or invalid credentials", body = ErrorEnvelope),
        (status = 403, description = "Missing `emails.send` (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Unknown template or personal template of another user (`email_template_not_found`)", body = ErrorEnvelope),
    )
)]
async fn get_template(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(template_id): Path<String>,
) -> Result<Json<Data<EmailTemplateResponse>>, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let template = service::templates::get(&ctx, &template_id).await?;
    Ok(Json(Data::new(template.into())))
}

#[utoipa::path(
    patch,
    path = "/email-templates/{template_id}",
    tag = TAG,
    operation_id = "email_templates_update",
    security(("session_cookie" = [])),
    params(("template_id" = String, Path, description = "Template id")),
    request_body = UpdateEmailTemplateRequest,
    responses(
        (status = 200, description = "Template updated (`email_template.updated` with audit); the scope cannot change", body = Data<EmailTemplateResponse>),
        (status = 401, description = "Missing or invalid credentials", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` for organization templates (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Unknown template (`email_template_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "Key already used (`email_template_key_taken`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid input (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn update_template(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(template_id): Path<String>,
    JsonBody(request): JsonBody<UpdateEmailTemplateRequest>,
) -> Result<Json<Data<EmailTemplateResponse>>, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let template =
        service::templates::update(&ctx, &template_id, request.into_command(client.0.ip)).await?;
    Ok(Json(Data::new(template.into())))
}

#[utoipa::path(
    delete,
    path = "/email-templates/{template_id}",
    tag = TAG,
    operation_id = "email_templates_delete",
    security(("session_cookie" = [])),
    params(("template_id" = String, Path, description = "Template id")),
    responses(
        (status = 204, description = "Template deleted (`email_template.deleted` with audit)"),
        (status = 401, description = "Missing or invalid credentials", body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` for organization templates (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Unknown template (`email_template_not_found`)", body = ErrorEnvelope),
    )
)]
async fn delete_template(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(template_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    service::templates::delete(&ctx, &template_id, client.0.ip).await?;
    Ok(StatusCode::NO_CONTENT)
}
