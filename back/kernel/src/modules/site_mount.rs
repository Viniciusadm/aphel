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
