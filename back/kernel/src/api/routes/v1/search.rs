use axum::Json;
use axum::extract::{Query, State};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::api::dto::common::{Data, ErrorEnvelope};
use crate::api::dto::search::{SearchParams, SearchResponse};
use crate::api::error::ApiError;
use crate::api::extractors::AuthContext;
use crate::api::state::AppState;
use crate::application::search::SearchUseCases;
use crate::infrastructure::search::MySqlSearchPorts;

pub const TAG: &str = "search";

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new().routes(routes!(search))
}

#[utoipa::path(
    get,
    path = "/search",
    tag = TAG,
    operation_id = "search",
    security(("session_cookie" = [])),
    description = "Unified search over every registered record type (contacts, companies, deals, activities and any type added to the record type registry). `q` must have between 2 and 100 characters; it is folded to lowercase without accents and split into words of at least 2 characters, and every word must match (substring match through an n-gram index) one of the indexed fields of the record. A term that looks like a phone number (only digits, spaces and `+()-./`, 6 to 15 digits) is searched by its digits, so `11 99999-8888`, `999998888` and `+55 11 99999-8888` all find `+5511999998888`. Indexed fields: searchable system fields and custom fields flagged searchable of the text, long text, e-mail, phone and URL types; sensitive fields are never indexed. Each type is searched only when the caller has its read permission, with the own/team/organization scope of that permission, the pipeline restriction of deals, soft delete, archive and merges applied by revalidating the candidates through the record query engine; fields hidden from the role are neither matched nor used for `title`/`subtitle`. `types` (comma separated) limits and orders the groups (unknown types answer 422); without it groups come by best relevance. `limit` is per type (default 5, max 20) and items are ordered by relevance (`score`). `total` counts the visible matches among the first 100 candidates of the type. Types without matches are omitted. The index is updated by the `search.index` consumer and rebuilt with `aphel search reindex`.",
    params(SearchParams),
    responses(
        (status = 200, description = "Search results grouped by record type", body = Data<SearchResponse>),
        (status = 401, description = "Missing or invalid credentials (`unauthorized`, `invalid_api_token`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid `q`, `types` or `limit` (`validation_error` with `details.fields`)", body = ErrorEnvelope),
    )
)]
async fn search(
    State(state): State<AppState>,
    tenant: AuthContext,
    Query(params): Query<SearchParams>,
) -> Result<Json<Data<SearchResponse>>, ApiError> {
    let ports = MySqlSearchPorts::new(state.db()?, state.registry().await?);
    let result = SearchUseCases::new(ports)
        .search(&tenant.scope, params.into())
        .await?;
    Ok(Json(Data::new(result.into())))
}
