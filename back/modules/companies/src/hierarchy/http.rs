use axum::Json;
use axum::extract::{Path, State};
use rust_decimal::Decimal;
use serde::Serialize;
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::hierarchy::service;
use crate::hierarchy::service::{CompanyHierarchy, GroupTotals, HierarchyNode};
use crate::http::TAG;
use aphel_kernel::Ctx;
use aphel_kernel::api::dto::common::OwnerResponse;
use aphel_kernel::api::dto::common::{Data, ErrorEnvelope};
use aphel_kernel::api::error::ApiError;
use aphel_kernel::api::extractors::AuthContext;
use aphel_kernel::api::state::AppState;

const UNAUTHORIZED: &str = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)";

pub(crate) fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(get_company_hierarchy))
        .routes(routes!(get_company_group_totals))
}

#[utoipa::path(
    get,
    path = "/companies/{company_id}/hierarchy",
    tag = TAG,
    operation_id = "companies_hierarchy",
    security(("session_cookie" = [])),
    description = "Company group of a company (permission `companies.read`): `root_id` is the top company of the group and `nodes` lists the group depth-first (siblings by name), each with `parent_id` and `depth` (0 = root). Companies the caller cannot read are left out, except when they sit on the path between the root and a readable company: then they appear with `accessible = false` and no data, so the tree keeps its shape. `contacts_count`, `open_deals_count` and `open_deals_amount` (sum of `amount_base`, in the base `currency` of the organization) count only records the caller can read; they are null without `contacts.read`/`deals.read` (or when `amount_base` is hidden). `subsidiaries` counts the direct subsidiaries listed. Up to 1000 companies (`truncated` tells when there are more). `max_depth` is the deepest group allowed when setting `parent_company_id`.",
    params(("company_id" = String, Path, description = "Company id")),
    responses(
        (status = 200, description = "Company group", body = Data<CompanyHierarchyResponse>),
        (status = 401, description = UNAUTHORIZED, body = ErrorEnvelope),
        (status = 403, description = "Missing `companies.read` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Company not found or not visible (`company_not_found`)", body = ErrorEnvelope),
    )
)]
async fn get_company_hierarchy(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(company_id): Path<String>,
) -> Result<Json<Data<CompanyHierarchyResponse>>, ApiError> {
    let hierarchy =
        service::hierarchy(&Ctx::from_state(&state, &tenant.scope).await?, &company_id).await?;
    Ok(Json(Data::new(hierarchy.into())))
}

#[utoipa::path(
    get,
    path = "/companies/{company_id}/group-totals",
    tag = TAG,
    operation_id = "companies_group_totals",
    security(("session_cookie" = [])),
    description = "Totals of a company and all its subsidiaries, at any level (permission `companies.read`). Only records the caller can read count: `companies_count` (readable companies of the group, the company included), `contacts_count` (contacts whose company is in the group; null without `contacts.read`), `open_deals_count`/`open_deals_amount` and `won_deals_count`/`won_amount` (deals whose company is in the group, amounts summed over `amount_base` in the base `currency` of the organization; null without `deals.read`, amounts null when `amount_base` is hidden). Archived contacts and deals are left out.",
    params(("company_id" = String, Path, description = "Company id")),
    responses(
        (status = 200, description = "Totals of the group", body = Data<CompanyGroupTotalsResponse>),
        (status = 401, description = UNAUTHORIZED, body = ErrorEnvelope),
        (status = 403, description = "Missing `companies.read` (`permission_denied`, `insufficient_scope`)", body = ErrorEnvelope),
        (status = 404, description = "Company not found or not visible (`company_not_found`)", body = ErrorEnvelope),
    )
)]
async fn get_company_group_totals(
    State(state): State<AppState>,
    tenant: AuthContext,
    Path(company_id): Path<String>,
) -> Result<Json<Data<CompanyGroupTotalsResponse>>, ApiError> {
    let totals =
        service::group_totals(&Ctx::from_state(&state, &tenant.scope).await?, &company_id).await?;
    Ok(Json(Data::new(totals.into())))
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct CompanyHierarchyNodeResponse {
    pub(crate) id: String,
    pub(crate) parent_id: Option<String>,
    #[schema(example = 0)]
    pub(crate) depth: u32,
    pub(crate) accessible: bool,
    #[schema(example = "Acme Holding")]
    pub(crate) name: Option<String>,
    pub(crate) owner: Option<OwnerResponse>,
    pub(crate) archived: bool,
    pub(crate) subsidiaries: u32,
    pub(crate) contacts_count: Option<u64>,
    pub(crate) open_deals_count: Option<u64>,
    #[schema(example = "15000.00")]
    pub(crate) open_deals_amount: Option<String>,
}

impl From<HierarchyNode> for CompanyHierarchyNodeResponse {
    fn from(node: HierarchyNode) -> Self {
        Self {
            id: node.id.to_string(),
            parent_id: node.parent_id.map(|id| id.to_string()),
            depth: node.depth,
            accessible: node.accessible,
            name: node.name,
            owner: node.owner.map(Into::into),
            archived: node.archived,
            subsidiaries: node.subsidiaries,
            contacts_count: node.contacts_count,
            open_deals_count: node.open_deals_count,
            open_deals_amount: node.open_deals_amount.map(money),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct CompanyHierarchyResponse {
    pub(crate) company_id: String,
    pub(crate) root_id: String,
    #[schema(example = 10)]
    pub(crate) max_depth: usize,
    pub(crate) truncated: bool,
    #[schema(example = "BRL")]
    pub(crate) currency: Option<String>,
    pub(crate) nodes: Vec<CompanyHierarchyNodeResponse>,
}

impl From<CompanyHierarchy> for CompanyHierarchyResponse {
    fn from(hierarchy: CompanyHierarchy) -> Self {
        Self {
            company_id: hierarchy.company_id.to_string(),
            root_id: hierarchy.root_id.to_string(),
            max_depth: hierarchy.max_depth,
            truncated: hierarchy.truncated,
            currency: hierarchy.currency,
            nodes: hierarchy.nodes.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct CompanyGroupTotalsResponse {
    pub(crate) company_id: String,
    pub(crate) companies_count: u64,
    #[schema(example = "BRL")]
    pub(crate) currency: Option<String>,
    pub(crate) contacts_count: Option<u64>,
    pub(crate) open_deals_count: Option<u64>,
    #[schema(example = "15000.00")]
    pub(crate) open_deals_amount: Option<String>,
    pub(crate) won_deals_count: Option<u64>,
    #[schema(example = "42000.00")]
    pub(crate) won_amount: Option<String>,
}

impl From<GroupTotals> for CompanyGroupTotalsResponse {
    fn from(totals: GroupTotals) -> Self {
        Self {
            company_id: totals.company_id.to_string(),
            companies_count: totals.companies_count,
            currency: totals.currency,
            contacts_count: totals.contacts_count,
            open_deals_count: totals.open_deals_count,
            open_deals_amount: totals.open_deals_amount.map(money),
            won_deals_count: totals.won_deals_count,
            won_amount: totals.won_amount.map(money),
        }
    }
}

fn money(value: Decimal) -> String {
    format!("{:.2}", value.round_dp(2))
}
