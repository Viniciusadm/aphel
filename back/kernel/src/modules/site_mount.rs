use axum::extract::Request;
use axum::response::Response;
use futures::future::BoxFuture;

use crate::api::state::AppState;

pub const SITE_RESERVED_SEGMENT: &str = "site";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SiteMountPrefix(pub String);

pub trait SiteRequestHandler: Send + Sync {
    fn serve<'a>(&'a self, state: &'a AppState, request: Request) -> BoxFuture<'a, Response>;

    fn serve_reserved<'a>(
        &'a self,
        state: &'a AppState,
        request: Request,
    ) -> BoxFuture<'a, Response>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublicAppPath(pub &'static str);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SiteSurface;

pub fn public_app_paths(state: &AppState) -> Vec<&'static str> {
    state
        .app
        .contributions::<PublicAppPath>()
        .into_iter()
        .map(|path| path.0)
        .collect()
}

pub fn site_enabled(app: &crate::modules::App) -> bool {
    !app.contributions::<dyn SiteRequestHandler>().is_empty()
}

pub fn public_tools_url(state: &AppState) -> String {
    state
        .auth_settings
        .app_base_url
        .tools_root(site_enabled(&state.app))
}
