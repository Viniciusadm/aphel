use std::sync::Arc;

use axum::extract::{Request, State};
use axum::middleware::{Next, from_fn_with_state};
use axum::response::Response;
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
}
