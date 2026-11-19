use std::sync::Arc;

use axum::Router;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use serde::Deserialize;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::tenancy::{TenantDirectory, normalize_host};

#[derive(Deserialize)]
struct DomainQuery {
    domain: String,
}

fn api() -> OpenApiRouter<Arc<TenantDirectory>> {
    OpenApiRouter::new().routes(routes!(authorize))
}

pub(super) fn router(directory: Arc<TenantDirectory>) -> Router {
    Router::from(api()).with_state(directory)
}

pub(super) fn openapi() -> utoipa::openapi::OpenApi {
    api().into_openapi()
}

#[utoipa::path(
    get,
    path = "/_aphel/tenancy/tls/authorize",
    tag = "tenancy",
    operation_id = "tenancy_authorize_tls",
    description = "Reverse proxy callback for on-demand TLS in multi-tenant mode. Available before host dispatch without an operator session. Allows only exact registered hosts of active tenants, using the in-memory directory. Unknown, suspended and deleting hosts are denied. No certificate is issued by this endpoint.",
    params(("domain" = String, Query, description = "The TLS server name requested by the proxy")),
    responses(
        (status = 204, description = "The proxy may manage a certificate for this host"),
        (status = 400, description = "Missing domain query parameter"),
        (status = 403, description = "Certificate management is not authorized for this host"),
    )
)]
async fn authorize(
    State(directory): State<Arc<TenantDirectory>>,
    Query(query): Query<DomainQuery>,
) -> StatusCode {
    let allowed = normalize_host(&query.domain)
        .filter(|host| *host == query.domain.to_ascii_lowercase())
        .and_then(|host| directory.resolve(&host))
        .is_some_and(|tenant| tenant.status.serves_requests());
    if allowed {
        StatusCode::NO_CONTENT
    } else {
        StatusCode::FORBIDDEN
    }
}
