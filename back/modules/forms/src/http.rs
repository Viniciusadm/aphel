use aphel_contract_crm::CrmRecordTypes;
use std::collections::HashMap;

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use chrono::Utc;
use serde_json::{Value, json};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;
use uuid::Uuid;

use aphel_kernel::Ctx;
use aphel_kernel::api::dto::common::{Data, ErrorEnvelope, Page, PageMeta};
use aphel_kernel::api::error::ApiError;
use aphel_kernel::api::extractors::{AuthContext, JsonBody};
use aphel_kernel::application::access::Authorizer;
use aphel_kernel::application::directory::DirectoryUseCases;
use aphel_kernel::application::organization_admin::ListWindow;
use aphel_kernel::application::shared::{Actor, AuditEntry, DomainEvent, FieldChange, RecordType};
use aphel_kernel::database::transaction::MySqlTx;
use aphel_kernel::errors::AppError;
use aphel_kernel::infrastructure::repositories::mysql_directory::MySqlUserDirectory;
use aphel_kernel::state::AppState;

use crate::model::{
    LeadFormDistributionRuleOption, LeadFormOptions, LeadFormRequest, LeadFormResponse,
    LeadFormSubmission, LeadFormSubmissionStatus, LeadFormTagOption, LeadFormUserOption,
};
use crate::store::{self, StoredForm};
use crate::validate;
use crate::{CREATED, DELETED, MANAGE, TAG, UPDATED, form_not_found, user_uuid};

pub(crate) const FORM_LIMIT_REACHED: &str = "lead_form_limit_reached";
pub(crate) const INVALID_QUERY: &str = "lead_form_invalid_query";
const MAX_FORMS: usize = 200;
const DEFAULT_PER_PAGE: u32 = 25;
const MAX_PER_PAGE: u32 = 100;

pub(crate) fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list_forms, create_form))
        .routes(routes!(get_form, update_form, delete_form))
        .routes(routes!(rotate_public_id))
        .routes(routes!(list_submissions))
        .routes(routes!(options))
}

fn parse_id(value: &str) -> Result<Uuid, AppError> {
    Uuid::parse_str(value).map_err(|_| form_not_found())
}

fn audit_changes(before: Option<&StoredForm>, after: Option<&StoredForm>) -> Vec<FieldChange> {
    let describe = |form: Option<&StoredForm>| {
        form.map(|form| {
            (
                json!(form.name),
                json!(form.is_active),
                json!(form.acting_user_id),
                serde_json::to_value(&form.definition).unwrap_or(Value::Null),
                json!(form.public_id),
            )
        })
        .unwrap_or((
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Null,
        ))
    };
    let (old_name, old_active, old_actor, old_definition, old_public) = describe(before);
    let (new_name, new_active, new_actor, new_definition, new_public) = describe(after);
    [
        ("name", old_name, new_name),
        ("is_active", old_active, new_active),
        ("acting_user_id", old_actor, new_actor),
        ("definition", old_definition, new_definition),
        ("public_id", old_public, new_public),
    ]
    .into_iter()
    .filter(|(_, old, new)| old != new)
    .map(|(field, old, new)| FieldChange::new(field, old, new))
    .collect()
}

async fn record_change(
    ctx: &Ctx<'_>,
    tx: &mut MySqlTx,
    event_type: &'static str,
    payload: Value,
    changes: Vec<FieldChange>,
) -> Result<(), AppError> {
    let actor = Actor::from_scope(ctx.scope);
    let now = Utc::now();
    ctx.events()
        .publish(
            tx,
            DomainEvent::new(actor.clone(), event_type, now).with_payload(payload),
        )
        .await?;
    ctx.audit()
        .record(
            tx,
            AuditEntry::new(actor, event_type, now).with_changes(changes),
        )
        .await
}

async fn save(
    ctx: &Ctx<'_>,
    event_type: &'static str,
    before: Option<&StoredForm>,
    form: &StoredForm,
) -> Result<(), AppError> {
    let mut tx = ctx.begin().await?;
    if before.is_some() {
        store::update_form(&mut tx, form).await?;
    } else {
        store::insert_form(&mut tx, form).await?;
    }
    record_change(
        ctx,
        &mut tx,
        event_type,
        json!({
            "form_id": form.id,
            "name": form.name,
            "is_active": form.is_active,
        }),
        audit_changes(before, Some(form)),
    )
    .await?;
    Ok(tx.commit().await?)
}

#[utoipa::path(
    get,
    path = "/forms/forms",
    tag = TAG,
    operation_id = "forms_list",
    security(("session_cookie" = [])),
    responses(
        (status = 200, description = "Lead forms, newest first, with submission counters", body = Page<LeadFormResponse>),
        (status = 403, description = "Missing `forms.manage` (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Module disabled (`module_disabled`)", body = ErrorEnvelope),
    )
)]
async fn list_forms(
    State(state): State<AppState>,
    tenant: AuthContext,
) -> Result<Json<Page<LeadFormResponse>>, ApiError> {
    let scope = &tenant.scope;
    scope.require(&MANAGE)?;
    let ctx = Ctx::from_state(&state, scope).await?;
    let forms = store::list_forms(ctx.pool()).await?;
    let total = forms.len();
    Ok(Json(Page {
        data: forms
            .into_iter()
            .map(|(form, stats)| store::response(form, stats))
            .collect(),
        meta: PageMeta {
            total: total as u64,
            page: 1,
            per_page: u32::try_from(total.max(1)).unwrap_or(u32::MAX),
        },
    }))
}

#[utoipa::path(
    post,
    path = "/forms/forms",
    tag = TAG,
    operation_id = "forms_create",
    security(("session_cookie" = [])),
    request_body = LeadFormRequest,
    responses(
        (status = 201, description = "Form created with a new opaque public id. Publishes `lead_form.created`", body = Data<LeadFormResponse>),
        (status = 403, description = "Missing `forms.manage` (`permission_denied`)", body = ErrorEnvelope),
        (status = 409, description = "The installation reached 200 forms (`lead_form_limit_reached`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid form (`lead_form_invalid` with `details.fields`) or body (`invalid_json`)", body = ErrorEnvelope),
    )
)]
async fn create_form(
    State(state): State<AppState>,
    tenant: AuthContext,
    JsonBody(request): JsonBody<LeadFormRequest>,
) -> Result<(StatusCode, Json<Data<LeadFormResponse>>), ApiError> {
    let scope = &tenant.scope;
    scope.require(&MANAGE)?;
    let ctx = Ctx::from_state(&state, scope).await?;
    let pool = ctx.pool().clone();
    if store::list_forms(&pool).await?.len() >= MAX_FORMS {
        return Err(AppError::conflict_code(
            FORM_LIMIT_REACHED,
            "the limit of 200 lead forms was reached",
        )
        .into());
    }
    let validated = validate::validate_request(&ctx, scope, request, user_uuid(scope)?).await?;
    let now = Utc::now().naive_utc();
    let form = StoredForm {
        id: Uuid::now_v7(),
        public_id: store::new_public_id(),
        name: validated.name,
        is_active: validated.is_active,
        acting_user_id: validated.acting_user_id,
        definition: validated.definition,
        created_by: user_uuid(scope)?,
        created_at: now,
        updated_at: now,
    };
    save(&ctx, CREATED, None, &form).await?;
    let (form, stats) = store::find_form(&pool, form.id).await?;
    Ok((
        StatusCode::CREATED,
        Json(Data {
            data: store::response(form, stats),
        }),
    ))
}

#[utoipa::path(
    get,
    path = "/forms/forms/{form_id}",
    tag = TAG,
    operation_id = "forms_get",
    security(("session_cookie" = [])),
    params(("form_id" = String, Path, description = "Form id")),
    responses(
        (status = 200, description = "The form", body = Data<LeadFormResponse>),
        (status = 403, description = "Missing `forms.manage` (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Form not found (`lead_form_not_found`)", body = ErrorEnvelope),
    )
)]
async fn get_form(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(form_id): Path<String>,
) -> Result<Json<Data<LeadFormResponse>>, ApiError> {
    let scope = &tenant.scope;
    scope.require(&MANAGE)?;
    let ctx = Ctx::from_state(&state, scope).await?;
    let (form, stats) = store::find_form(ctx.pool(), parse_id(&form_id)?).await?;
    Ok(Json(Data {
        data: store::response(form, stats),
    }))
}

#[utoipa::path(
    put,
    path = "/forms/forms/{form_id}",
    tag = TAG,
    operation_id = "forms_update",
    security(("session_cookie" = [])),
    params(("form_id" = String, Path, description = "Form id")),
    request_body = LeadFormRequest,
    responses(
        (status = 200, description = "Form replaced (the public id is kept). Publishes `lead_form.updated`", body = Data<LeadFormResponse>),
        (status = 403, description = "Missing `forms.manage` (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Form not found (`lead_form_not_found`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid form (`lead_form_invalid` with `details.fields`) or body (`invalid_json`)", body = ErrorEnvelope),
    )
)]
async fn update_form(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(form_id): Path<String>,
    JsonBody(request): JsonBody<LeadFormRequest>,
) -> Result<Json<Data<LeadFormResponse>>, ApiError> {
    let scope = &tenant.scope;
    scope.require(&MANAGE)?;
    let ctx = Ctx::from_state(&state, scope).await?;
    let pool = ctx.pool().clone();
    let (before, _) = store::find_form(&pool, parse_id(&form_id)?).await?;
    let validated = validate::validate_request(&ctx, scope, request, before.created_by).await?;
    let form = StoredForm {
        name: validated.name,
        is_active: validated.is_active,
        acting_user_id: validated.acting_user_id,
        definition: validated.definition,
        updated_at: Utc::now().naive_utc(),
        ..before.clone()
    };
    save(&ctx, UPDATED, Some(&before), &form).await?;
    let (form, stats) = store::find_form(&pool, form.id).await?;
    Ok(Json(Data {
        data: store::response(form, stats),
    }))
}

#[utoipa::path(
    post,
    path = "/forms/forms/{form_id}/rotate-public-id",
    tag = TAG,
    operation_id = "forms_rotate_public_id",
    security(("session_cookie" = [])),
    params(("form_id" = String, Path, description = "Form id")),
    responses(
        (status = 200, description = "New public id: the previous page, script and submission URLs stop working at once. Publishes `lead_form.updated`", body = Data<LeadFormResponse>),
        (status = 403, description = "Missing `forms.manage` (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Form not found (`lead_form_not_found`)", body = ErrorEnvelope),
    )
)]
async fn rotate_public_id(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(form_id): Path<String>,
) -> Result<Json<Data<LeadFormResponse>>, ApiError> {
    let scope = &tenant.scope;
    scope.require(&MANAGE)?;
    let ctx = Ctx::from_state(&state, scope).await?;
    let pool = ctx.pool().clone();
    let (before, _) = store::find_form(&pool, parse_id(&form_id)?).await?;
    let form = StoredForm {
        public_id: store::new_public_id(),
        updated_at: Utc::now().naive_utc(),
        ..before.clone()
    };
    save(&ctx, UPDATED, Some(&before), &form).await?;
    let (form, stats) = store::find_form(&pool, form.id).await?;
    Ok(Json(Data {
        data: store::response(form, stats),
    }))
}

#[utoipa::path(
    delete,
    path = "/forms/forms/{form_id}",
    tag = TAG,
    operation_id = "forms_delete",
    security(("session_cookie" = [])),
    params(("form_id" = String, Path, description = "Form id")),
    responses(
        (status = 204, description = "Form and its submission history deleted (people and deals created by it stay). Publishes `lead_form.deleted`"),
        (status = 403, description = "Missing `forms.manage` (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Form not found (`lead_form_not_found`)", body = ErrorEnvelope),
    )
)]
async fn delete_form(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(form_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let scope = &tenant.scope;
    scope.require(&MANAGE)?;
    let ctx = Ctx::from_state(&state, scope).await?;
    let (form, _) = store::find_form(ctx.pool(), parse_id(&form_id)?).await?;
    let mut tx = ctx.begin().await?;
    store::delete_form(&mut tx, form.id).await?;
    record_change(
        &ctx,
        &mut tx,
        DELETED,
        json!({ "form_id": form.id, "name": form.name }),
        audit_changes(Some(&form), None),
    )
    .await?;
    tx.commit().await.map_err(AppError::from)?;
    Ok(StatusCode::NO_CONTENT)
}

fn page_param(params: &HashMap<String, String>, key: &str, default: u32) -> Result<u32, AppError> {
    match params.get(key) {
        None => Ok(default),
        Some(value) => value
            .parse::<u32>()
            .ok()
            .filter(|value| *value >= 1)
            .ok_or_else(|| {
                AppError::validation_code(
                    INVALID_QUERY,
                    format!("{key} must be a positive integer"),
                )
            }),
    }
}

#[utoipa::path(
    get,
    path = "/forms/forms/{form_id}/submissions",
    tag = TAG,
    operation_id = "forms_submissions",
    security(("session_cookie" = [])),
    params(
        ("form_id" = String, Path, description = "Form id"),
        ("status" = Option<String>, Query, description = "accepted, spam or failed"),
        ("page" = Option<u32>, Query, description = "Page (from 1)"),
        ("per_page" = Option<u32>, Query, description = "Items per page (1 to 100, default 25)"),
    ),
    responses(
        (status = 200, description = "Submission history of the form, newest first, including spam and failures", body = Page<LeadFormSubmission>),
        (status = 403, description = "Missing `forms.manage` (`permission_denied`)", body = ErrorEnvelope),
        (status = 404, description = "Form not found (`lead_form_not_found`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid status or pagination (`lead_form_invalid_query`)", body = ErrorEnvelope),
    )
)]
async fn list_submissions(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(form_id): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<Page<LeadFormSubmission>>, ApiError> {
    let scope = &tenant.scope;
    scope.require(&MANAGE)?;
    let ctx = Ctx::from_state(&state, scope).await?;
    let pool = ctx.pool().clone();
    let (form, _) = store::find_form(&pool, parse_id(&form_id)?).await?;
    let status = match params.get("status").filter(|value| !value.is_empty()) {
        None => None,
        Some(value) => Some(LeadFormSubmissionStatus::parse(value).ok_or_else(|| {
            AppError::validation_code(INVALID_QUERY, "status must be accepted, spam or failed")
        })?),
    };
    let page = page_param(&params, "page", 1)?;
    let per_page = page_param(&params, "per_page", DEFAULT_PER_PAGE)?.min(MAX_PER_PAGE);
    let (data, total) = store::list_submissions(
        &pool,
        form.id,
        status,
        u64::from(page - 1) * u64::from(per_page),
        u64::from(per_page),
    )
    .await?;
    Ok(Json(Page {
        data,
        meta: PageMeta {
            total,
            page,
            per_page,
        },
    }))
}

async fn distribution_rules(
    ctx: &Ctx<'_>,
) -> Result<Vec<LeadFormDistributionRuleOption>, AppError> {
    if !ctx.app().is_enabled(assignment::KEY) {
        return Ok(Vec::new());
    }
    let listing = assignment::DistributionStore::of(ctx)
        .list(&assignment::RuleListFilter {
            record_type: Some(RecordType::CONTACT.as_str().to_owned()),
            active: Some(true),
            window: ListWindow {
                offset: 0,
                limit: 100,
            },
        })
        .await?;
    Ok(listing
        .items
        .into_iter()
        .map(|rule| LeadFormDistributionRuleOption {
            id: rule.id.to_string(),
            name: rule.name,
        })
        .collect())
}

#[utoipa::path(
    get,
    path = "/forms/options",
    tag = TAG,
    operation_id = "forms_options",
    security(("session_cookie" = [])),
    responses(
        (status = 200, description = "What the form editor can pick: writable contact fields (system and custom), active workflow and stages, contact tags, active users and active lead distribution rules for people. Lists the current user cannot read come back empty", body = Data<LeadFormOptions>),
        (status = 403, description = "Missing `forms.manage` (`permission_denied`)", body = ErrorEnvelope),
    )
)]
async fn options(
    State(state): State<AppState>,
    tenant: AuthContext,
) -> Result<Json<Data<LeadFormOptions>>, ApiError> {
    let scope = &tenant.scope;
    scope.require(&MANAGE)?;
    let ctx = Ctx::from_state(&state, scope).await?;
    let contact_fields = validate::target_fields(&ctx, scope).await?;
    let workflow = validate::workflow(&ctx, scope).await.unwrap_or_default();
    let tags = tags::service::list(&ctx, Some("people.person"))
        .await
        .map(|tags| {
            tags.into_iter()
                .take(100)
                .map(|view| LeadFormTagOption {
                    id: view.tag.id.to_string(),
                    name: view.tag.name,
                    color: view.tag.color,
                })
                .collect()
        })
        .unwrap_or_default();
    let users = DirectoryUseCases::new(MySqlUserDirectory::new(ctx.pool().clone()))
        .list_users(
            None,
            ListWindow {
                offset: 0,
                limit: 100,
            },
        )
        .await
        .map(|listing| {
            listing
                .items
                .into_iter()
                .map(|user| LeadFormUserOption {
                    id: user.id.as_str().to_owned(),
                    display_name: user.display_name,
                    email: user.email,
                })
                .collect()
        })
        .unwrap_or_default();
    let distribution_rules = distribution_rules(&ctx).await.unwrap_or_default();
    Ok(Json(Data {
        data: LeadFormOptions {
            contact_fields,
            workflow,
            tags,
            users,
            distribution_rules,
        },
    }))
}
