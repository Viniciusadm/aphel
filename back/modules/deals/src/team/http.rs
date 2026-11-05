use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::{IntoParams, ToSchema};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::domain::instant;
use crate::http::DecimalInputSchema;
use crate::team::domain::{
    AddDealMemberCommand, CreateDealMemberRoleCommand, DealMember, DealMemberRole,
    DealMemberRoleRef, DealTeam, UpdateDealMemberCommand, UpdateDealMemberRoleCommand,
    percent_text,
};
use crate::team::service;
use aphel_kernel::Ctx;
use aphel_kernel::domain::UserId;
use aphel_sdk::platform::api::dto::common::{Data, ErrorEnvelope, decimal_text, double_option};
use aphel_sdk::platform::api::dto::files::UserReferenceResponse;
use aphel_sdk::platform::api::error::ApiError;
use aphel_sdk::platform::api::extractors::{AuthContext, ClientInfo, IfMatch, JsonBody};
use aphel_sdk::platform::api::state::AppState;

pub(crate) const TAG: &str = "deal-members";

const UNAUTHORIZED: &str = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)";

pub(crate) fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(list_deal_members, add_deal_member))
        .routes(routes!(update_deal_member, remove_deal_member))
        .routes(routes!(list_roles, create_role))
        .routes(routes!(reorder_roles))
        .routes(routes!(update_role, delete_role))
}

#[utoipa::path(
    get,
    path = "/deals/{deal_id}/members",
    tag = TAG,
    operation_id = "deal_members_list",
    security(("session_cookie" = [])),
    description = "Team of a deal (permission `deals.read` on the deal, including the pipeline restriction by team): the members with role and revenue credit, and the owner with the credit left to them. The credit of the members adds up to at most 100%; the owner receives the rest (`remaining_percent`), plus their own line when they are also a member. Members see the deal even with the `own` scope of `deals.read` (read only; editing follows the owner rule).",
    params(("deal_id" = String, Path, description = "Deal id")),
    responses(
        (status = 200, description = "Team of the deal", body = Data<DealTeamResponse>),
        (status = 401, description = UNAUTHORIZED, body = ErrorEnvelope),
        (status = 403, description = "Missing `deals.read` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Deal not found or not visible (`deal_not_found`)", body = ErrorEnvelope),
    )
)]
async fn list_deal_members(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(deal_id): Path<String>,
) -> Result<Json<Data<DealTeamResponse>>, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let team = service::team(&ctx, &deal_id).await?;
    Ok(Json(Data::new(team.into())))
}

#[utoipa::path(
    post,
    path = "/deals/{deal_id}/members",
    tag = TAG,
    operation_id = "deal_members_create",
    security(("session_cookie" = [])),
    description = "Adds a user to the team of a deal (permission `deals.update` on the deal) and publishes `deal.member_added`. `user_id` is an active member of the organization (the owner may be added too); `role_id` is an active deal member role (optional); `credit_percent` (0–100, 2 decimals, default 0) is taken from the share of the owner, and the members together cannot pass 100%. At most 50 members per deal.",
    params(("deal_id" = String, Path, description = "Deal id")),
    request_body = AddDealMemberRequest,
    responses(
        (status = 201, description = "Member added", body = Data<DealMemberResponse>),
        (status = 401, description = UNAUTHORIZED, body = ErrorEnvelope),
        (status = 403, description = "Missing `deals.update` on the deal (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Deal not found or not visible (`deal_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "The user is already a member (`deal_member_exists`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid user, role or credit (`validation_error`), credit above the available share (`deal_credit_exceeds_total`, with `allocated_percent` and `available_percent` in `details`) or member limit reached (`deal_members_limit_reached`)", body = ErrorEnvelope),
    )
)]
async fn add_deal_member(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(deal_id): Path<String>,
    JsonBody(request): JsonBody<AddDealMemberRequest>,
) -> Result<(StatusCode, Json<Data<DealMemberResponse>>), ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let member = service::add_member(&ctx, &deal_id, request.into_command(client.0.ip)).await?;
    Ok((StatusCode::CREATED, Json(Data::new(member.into()))))
}

#[utoipa::path(
    patch,
    path = "/deals/{deal_id}/members/{member_id}",
    tag = TAG,
    operation_id = "deal_members_update",
    security(("session_cookie" = [])),
    description = "Changes the role or the credit of a member (permission `deals.update` on the deal) and publishes `deal.member_updated` with the changes; omitted properties stay unchanged and `role_id: null` clears the role.",
    params(
        ("deal_id" = String, Path, description = "Deal id"),
        ("member_id" = String, Path, description = "Member id"),
        ("If-Match" = Option<String>, Header, description = "`updated_at` of the member as read (ISO-8601); the update fails with 409 `version_conflict` when it changed since"),
    ),
    request_body = UpdateDealMemberRequest,
    responses(
        (status = 200, description = "Member updated", body = Data<DealMemberResponse>),
        (status = 401, description = UNAUTHORIZED, body = ErrorEnvelope),
        (status = 403, description = "Missing `deals.update` on the deal (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Deal or member not found (`deal_not_found`, `deal_member_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "The member changed since the `If-Match` version (`version_conflict`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid role or credit (`validation_error`) or credit above the available share (`deal_credit_exceeds_total`)", body = ErrorEnvelope),
    )
)]
async fn update_deal_member(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    IfMatch(expected): IfMatch,
    Path((deal_id, member_id)): Path<(String, String)>,
    JsonBody(request): JsonBody<UpdateDealMemberRequest>,
) -> Result<Json<Data<DealMemberResponse>>, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let member = service::update_member(
        &ctx,
        &deal_id,
        &member_id,
        request.into_command(client.0.ip, expected),
    )
    .await?;
    Ok(Json(Data::new(member.into())))
}

#[utoipa::path(
    delete,
    path = "/deals/{deal_id}/members/{member_id}",
    tag = TAG,
    operation_id = "deal_members_delete",
    security(("session_cookie" = [])),
    description = "Removes a member from the team (permission `deals.update` on the deal) and publishes `deal.member_removed`; their credit returns to the owner.",
    params(
        ("deal_id" = String, Path, description = "Deal id"),
        ("member_id" = String, Path, description = "Member id"),
    ),
    responses(
        (status = 204, description = "Member removed"),
        (status = 401, description = UNAUTHORIZED, body = ErrorEnvelope),
        (status = 403, description = "Missing `deals.update` on the deal (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Deal or member not found (`deal_not_found`, `deal_member_not_found`)", body = ErrorEnvelope),
    )
)]
async fn remove_deal_member(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path((deal_id, member_id)): Path<(String, String)>,
) -> Result<StatusCode, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    service::remove_member(&ctx, &deal_id, &member_id, client.0.ip).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    get,
    path = "/deal-member-roles",
    tag = TAG,
    operation_id = "deal_member_roles_list",
    security(("session_cookie" = [])),
    params(DealMemberRoleListParams),
    responses(
        (status = 200, description = "Roles of deal team members in their order (active only unless `include_inactive=true`). `sales`, `pre_sales`, `specialist` and `manager` are created with the organization", body = Data<Vec<DealMemberRoleResponse>>),
        (status = 401, description = UNAUTHORIZED, body = ErrorEnvelope),
        (status = 403, description = "Missing `deals.read` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
    )
)]
async fn list_roles(
    State(state): State<AppState>,
    tenant: AuthContext,
    Query(params): Query<DealMemberRoleListParams>,
) -> Result<Json<Data<Vec<DealMemberRoleResponse>>>, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let roles = service::list_roles(&ctx, params.include_inactive.unwrap_or(false)).await?;
    Ok(Json(Data::new(roles.into_iter().map(Into::into).collect())))
}

#[utoipa::path(
    post,
    path = "/deal-member-roles",
    tag = TAG,
    operation_id = "deal_member_roles_create",
    security(("session_cookie" = [])),
    request_body = CreateDealMemberRoleRequest,
    responses(
        (status = 201, description = "Role created at the end of the list (`deal_member_role.created`); `key` defaults to one derived from the name", body = Data<DealMemberRoleResponse>),
        (status = 401, description = UNAUTHORIZED, body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 409, description = "Another role uses the key (`deal_member_role_key_taken`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid name or key, or the limit of 50 roles was reached (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn create_role(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    JsonBody(request): JsonBody<CreateDealMemberRoleRequest>,
) -> Result<(StatusCode, Json<Data<DealMemberRoleResponse>>), ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let role = service::create_role(&ctx, request.into_command(client.0.ip)).await?;
    Ok((StatusCode::CREATED, Json(Data::new(role.into()))))
}

#[utoipa::path(
    patch,
    path = "/deal-member-roles/{role_id}",
    tag = TAG,
    operation_id = "deal_member_roles_update",
    security(("session_cookie" = [])),
    params(
        ("role_id" = String, Path, description = "Role id"),
        ("If-Match" = Option<String>, Header, description = "`updated_at` of the role as last read (ISO-8601); when it changed since, the update is refused with 409 `version_conflict`"),
    ),
    request_body = UpdateDealMemberRoleRequest,
    responses(
        (status = 200, description = "Role updated (`deal_member_role.updated`); the key never changes. Inactive roles stay on the members that have them but cannot be picked again", body = Data<DealMemberRoleResponse>),
        (status = 401, description = UNAUTHORIZED, body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Role not found (`deal_member_role_not_found`)", body = ErrorEnvelope),
        (status = 409, description = "The role changed after the `If-Match` version (`version_conflict`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid name (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn update_role(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    IfMatch(expected): IfMatch,
    Path(role_id): Path<String>,
    JsonBody(request): JsonBody<UpdateDealMemberRoleRequest>,
) -> Result<Json<Data<DealMemberRoleResponse>>, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let role =
        service::update_role(&ctx, &role_id, request.into_command(client.0.ip, expected)).await?;
    Ok(Json(Data::new(role.into())))
}

#[utoipa::path(
    delete,
    path = "/deal-member-roles/{role_id}",
    tag = TAG,
    operation_id = "deal_member_roles_delete",
    security(("session_cookie" = [])),
    params(("role_id" = String, Path, description = "Role id")),
    responses(
        (status = 204, description = "Role deleted (`deal_member_role.deleted`); the members that had it keep their credit and stay without role"),
        (status = 401, description = UNAUTHORIZED, body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Role not found (`deal_member_role_not_found`)", body = ErrorEnvelope),
    )
)]
async fn delete_role(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    Path(role_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    service::delete_role(&ctx, &role_id, client.0.ip).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    put,
    path = "/deal-member-roles/order",
    tag = TAG,
    operation_id = "deal_member_roles_reorder",
    security(("session_cookie" = [])),
    request_body = ReorderDealMemberRolesRequest,
    responses(
        (status = 200, description = "Roles in the new order (`deal_member_role.reordered`); `ids` lists every role, active or not, exactly once", body = Data<Vec<DealMemberRoleResponse>>),
        (status = 401, description = UNAUTHORIZED, body = ErrorEnvelope),
        (status = 403, description = "Missing `settings.manage` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 422, description = "The list does not match the roles of the organization (`validation_error`)", body = ErrorEnvelope),
    )
)]
async fn reorder_roles(
    State(state): State<AppState>,
    tenant: AuthContext,
    client: ClientInfo,
    JsonBody(request): JsonBody<ReorderDealMemberRolesRequest>,
) -> Result<Json<Data<Vec<DealMemberRoleResponse>>>, ApiError> {
    let ctx = Ctx::from_state(&state, &tenant.scope).await?;
    let roles = service::reorder_roles(&ctx, &request.ids, client.0.ip).await?;
    Ok(Json(Data::new(roles.into_iter().map(Into::into).collect())))
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct DealMemberRoleSummaryResponse {
    pub(crate) id: String,
    #[schema(example = "pre_sales")]
    pub(crate) key: String,
    #[schema(example = "Pre-sales")]
    pub(crate) name: String,
}

impl From<DealMemberRoleRef> for DealMemberRoleSummaryResponse {
    fn from(role: DealMemberRoleRef) -> Self {
        Self {
            id: role.id.to_string(),
            key: role.key,
            name: role.name,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct DealMemberResponse {
    pub(crate) id: String,
    pub(crate) deal_id: String,
    pub(crate) user: UserReferenceResponse,
    pub(crate) user_active: bool,
    pub(crate) role: Option<DealMemberRoleSummaryResponse>,
    #[schema(example = "30.00")]
    pub(crate) credit_percent: String,
    pub(crate) added_by: Option<String>,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

impl From<DealMember> for DealMemberResponse {
    fn from(member: DealMember) -> Self {
        Self {
            id: member.id.to_string(),
            deal_id: member.deal_id.to_string(),
            user: member.user.into(),
            user_active: member.user_active,
            role: member.role.map(Into::into),
            credit_percent: percent_text(member.credit_percent),
            added_by: member.added_by.map(UserId::into_inner),
            created_at: instant(member.created_at),
            updated_at: instant(member.updated_at),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct DealTeamOwnerResponse {
    pub(crate) id: String,
    pub(crate) display_name: String,
    #[schema(example = "70.00")]
    pub(crate) credit_percent: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct DealTeamResponse {
    pub(crate) deal_id: String,
    pub(crate) owner: Option<DealTeamOwnerResponse>,
    #[schema(example = "30.00")]
    pub(crate) allocated_percent: String,
    #[schema(example = "70.00")]
    pub(crate) remaining_percent: String,
    pub(crate) members: Vec<DealMemberResponse>,
}

impl From<DealTeam> for DealTeamResponse {
    fn from(team: DealTeam) -> Self {
        let owner_percent = team.owner_percent.unwrap_or_default();
        Self {
            deal_id: team.deal_id.to_string(),
            owner: team.owner.map(|owner| DealTeamOwnerResponse {
                id: owner.id.into_inner(),
                display_name: owner.display_name,
                credit_percent: percent_text(owner_percent),
            }),
            allocated_percent: percent_text(team.allocated_percent),
            remaining_percent: percent_text(team.remaining_percent),
            members: team.members.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct AddDealMemberRequest {
    pub(crate) user_id: String,
    pub(crate) role_id: Option<String>,
    #[schema(value_type = Option<DecimalInputSchema>, example = "30")]
    pub(crate) credit_percent: Option<Value>,
}

impl AddDealMemberRequest {
    pub(crate) fn into_command(self, ip: Option<String>) -> AddDealMemberCommand {
        AddDealMemberCommand {
            user_id: self.user_id,
            role_id: self.role_id,
            credit_percent: self.credit_percent.and_then(decimal_text),
            ip,
        }
    }
}

#[derive(Debug, Default, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct UpdateDealMemberRequest {
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<String>)]
    pub(crate) role_id: Option<Option<String>>,
    #[schema(value_type = Option<DecimalInputSchema>, example = "25.5")]
    pub(crate) credit_percent: Option<Value>,
}

impl UpdateDealMemberRequest {
    pub(crate) fn into_command(
        self,
        ip: Option<String>,
        expected_updated_at: Option<DateTime<Utc>>,
    ) -> UpdateDealMemberCommand {
        UpdateDealMemberCommand {
            role_id: self.role_id,
            credit_percent: self
                .credit_percent
                .map(|value| decimal_text(value).unwrap_or_default()),
            expected_updated_at,
            ip,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct DealMemberRoleResponse {
    pub(crate) id: String,
    #[schema(example = "pre_sales")]
    pub(crate) key: String,
    #[schema(example = "Pre-sales")]
    pub(crate) name: String,
    pub(crate) is_active: bool,
    pub(crate) position: i32,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

impl From<DealMemberRole> for DealMemberRoleResponse {
    fn from(role: DealMemberRole) -> Self {
        Self {
            id: role.id.to_string(),
            key: role.key,
            name: role.name,
            is_active: role.is_active,
            position: role.position,
            created_at: instant(role.created_at),
            updated_at: instant(role.updated_at),
        }
    }
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct DealMemberRoleListParams {
    pub(crate) include_inactive: Option<bool>,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct CreateDealMemberRoleRequest {
    #[schema(example = "solution_architect")]
    pub(crate) key: Option<String>,
    #[schema(example = "Solutions architect")]
    pub(crate) name: String,
}

impl CreateDealMemberRoleRequest {
    pub(crate) fn into_command(self, ip: Option<String>) -> CreateDealMemberRoleCommand {
        CreateDealMemberRoleCommand {
            key: self.key,
            name: self.name,
            ip,
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct UpdateDealMemberRoleRequest {
    pub(crate) name: Option<String>,
    pub(crate) is_active: Option<bool>,
}

impl UpdateDealMemberRoleRequest {
    pub(crate) fn into_command(
        self,
        ip: Option<String>,
        expected_updated_at: Option<DateTime<Utc>>,
    ) -> UpdateDealMemberRoleCommand {
        UpdateDealMemberRoleCommand {
            name: self.name,
            is_active: self.is_active,
            expected_updated_at,
            ip,
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct ReorderDealMemberRolesRequest {
    pub(crate) ids: Vec<String>,
}
