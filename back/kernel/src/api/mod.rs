pub(crate) mod api_token_limit;
pub mod automation_operations;
pub(crate) mod client_address;
pub(crate) mod csrf;
pub(crate) mod demo_guard;
pub(crate) mod docs;
pub mod dto;
pub mod error;
pub mod extractors;
pub(crate) mod idempotency;
pub mod payload_limit;
pub mod rate_limit;
pub(crate) mod records;
pub mod routes;
pub mod session_cookie;
pub mod sso;
pub mod state;
pub(crate) mod telemetry;
pub(crate) mod web;

use axum::Router;
use axum::middleware;
use tower_http::cors::CorsLayer;
use utoipa_axum::router::OpenApiRouter;

use state::AppState;

pub fn router(state: AppState) -> Router {
    crate::state::install_runtime_state(state.clone());
    let (api_router, mut openapi) = openapi_router(&state).split_for_parts();
    let api_router = state.app.apply_overrides(api_router, &state);
    docs::add_api_token_security(&mut openapi);
    idempotency::document(&mut openapi);

    let app = api_router
        .layer(middleware::from_fn(payload_limit::envelope))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            idempotency::guard,
        ))
        .layer(middleware::from_fn(csrf::require_header))
        .layer(middleware::from_fn(demo_guard::guard))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            api_token_limit::limit,
        ))
        .merge(docs::router(openapi))
        .fallback(web::serve_web)
        .with_state(state)
        .layer(CorsLayer::permissive());

    telemetry::apply(routes::health::track_responses(app))
}

pub fn openapi() -> utoipa::openapi::OpenApi {
    let mut document = openapi_router(&AppState::new(None)).into_openapi();
    docs::add_api_token_security(&mut document);
    idempotency::document(&mut document);
    document
}

pub fn openapi_json() -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(&openapi()).map(|json| json + "\n")
}

fn openapi_router(state: &AppState) -> OpenApiRouter<AppState> {
    OpenApiRouter::with_openapi(docs::base_document(&state.app)).merge(state.app.router(state))
}
