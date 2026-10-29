use std::collections::HashMap;
use std::sync::Arc;

use axum::Router;
use axum::extract::{MatchedPath, Request, State};
use axum::http::Method;
use axum::middleware::{Next, from_fn_with_state};
use axum::response::{IntoResponse, Response};
use tower::ServiceExt;
use utoipa_axum::router::OpenApiRouter;

use crate::api::error::app_error;
use crate::api::state::AppState;
use crate::errors::AppError;
use crate::modules::builder::{App, RouteMount};
use crate::modules::enabled::EnabledModules;

pub const MODULE_DISABLED: &str = "module_disabled";

pub fn module_disabled() -> AppError {
    AppError::not_found_code(MODULE_DISABLED, "this module is disabled")
}

#[derive(Clone)]
struct ModuleGate {
    enabled: Arc<EnabledModules>,
    module: &'static str,
}

async fn gate(State(gate): State<ModuleGate>, request: Request, next: Next) -> Response {
    if !gate.enabled.is_enabled(gate.module) {
        return app_error(module_disabled());
    }
    next.run(request).await
}

type OverrideTargets = HashMap<(Method, String), (&'static str, Router)>;

#[derive(Clone)]
struct Overrides {
    enabled: Arc<EnabledModules>,
    targets: Arc<OverrideTargets>,
}

async fn overridden(State(overrides): State<Overrides>, request: Request, next: Next) -> Response {
    let target = request
        .extensions()
        .get::<MatchedPath>()
        .and_then(|path| {
            overrides
                .targets
                .get(&(request.method().clone(), path.as_str().to_owned()))
        })
        .filter(|(module, _)| overrides.enabled.is_enabled(module))
        .map(|(_, router)| router.clone());
    match target {
        Some(router) => match router.oneshot(request).await {
            Ok(response) => response.into_response(),
            Err(never) => match never {},
        },
        None => next.run(request).await,
    }
}

impl App {
    pub fn module_router(
        &self,
        module: &'static str,
        router: OpenApiRouter<AppState>,
    ) -> OpenApiRouter<AppState> {
        if self.enabled().is_required(module) {
            return router;
        }
        router.layer(from_fn_with_state(
            ModuleGate {
                enabled: self.enabled().clone(),
                module,
            },
            gate,
        ))
    }

    pub fn router(&self, state: &AppState) -> OpenApiRouter<AppState> {
        let mut api = OpenApiRouter::new();
        let mut root = OpenApiRouter::new();
        for contribution in self.route_contributions() {
            let router =
                self.module_router(contribution.module, (contribution.item.factory)(state));
            match contribution.item.mount {
                RouteMount::Api | RouteMount::Public => api = api.merge(router),
                RouteMount::Root => root = root.merge(router),
            }
        }
        root.nest(RouteMount::Api.prefix(), api)
    }

    pub fn apply_overrides(&self, router: Router<AppState>, state: &AppState) -> Router<AppState> {
        let overrides = self.route_overrides();
        if overrides.is_empty() {
            return router;
        }
        let mut targets = OverrideTargets::new();
        for contribution in overrides {
            let item = &contribution.item;
            tracing::info!(
                module = contribution.module,
                method = %item.method,
                path = %item.path,
                "route overridden by module"
            );
            let target = Router::new()
                .route(&item.path, item.handler.clone())
                .with_state(state.clone());
            targets.insert(
                (item.method.clone(), item.path.clone()),
                (contribution.module, target),
            );
        }
        router.layer(from_fn_with_state(
            Overrides {
                enabled: self.enabled().clone(),
                targets: Arc::new(targets),
            },
            overridden,
        ))
    }
}
