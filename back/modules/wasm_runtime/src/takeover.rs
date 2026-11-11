use axum::body::to_bytes;
use axum::extract::{FromRequestParts, OriginalUri, Request};
use axum::response::{IntoResponse, Response};
use futures::future::BoxFuture;

use aphel_sdk::platform::api::extractors::AuthContext;
use aphel_sdk::platform::api::state::AppState;
use aphel_sdk::platform::modules::RouteTakeover;

use crate::http::{RouteCall, serve_route};
use crate::registry;

const API_PREFIX: &str = "/api/v1";
const MAX_BODY_BYTES: usize = 10 * 1024 * 1024;

pub struct AliasTakeover;

fn mapped_path(manifest: &crate::manifest::PluginManifest, path: &str) -> Option<String> {
    manifest.aliases.iter().find_map(|(native, plugin)| {
        let rest = path.strip_prefix(native.as_str())?;
        if !rest.is_empty() && !rest.starts_with('/') {
            return None;
        }
        Some(format!("{plugin}{rest}"))
    })
}

impl RouteTakeover for AliasTakeover {
    fn take<'a>(
        &'a self,
        state: &'a AppState,
        module: &'static str,
        request: Request,
    ) -> BoxFuture<'a, Result<Response, Request>> {
        Box::pin(async move {
            let Ok(pool) = state.db() else {
                return Err(request);
            };
            let Ok(snapshot) = registry::current(&pool).await else {
                return Err(request);
            };
            let full = request
                .extensions()
                .get::<OriginalUri>()
                .map(|original| original.0.path().to_owned())
                .unwrap_or_else(|| request.uri().path().to_owned());
            let Some(path) = full.strip_prefix(API_PREFIX) else {
                return Err(request);
            };
            let Some((plugin, target)) = snapshot.plugins.iter().find_map(|plugin| {
                if plugin.manifest.aliases.is_empty() {
                    return None;
                }
                if !crate::modules::held_modules(&plugin.manifest)
                    .iter()
                    .any(|held| held == module)
                {
                    return None;
                }
                mapped_path(&plugin.manifest, path).map(|target| (plugin.clone(), target))
            }) else {
                return Err(request);
            };
            let (mut parts, body) = request.into_parts();
            let tenant = match AuthContext::from_request_parts(&mut parts, state).await {
                Ok(tenant) => tenant,
                Err(error) => return Ok(error.into_response()),
            };
            let Ok(bytes) = to_bytes(body, MAX_BODY_BYTES).await else {
                return Ok(aphel_sdk::platform::api::error::app_error(
                    aphel_sdk::AppError::Validation("the request body is too large".to_owned()),
                ));
            };
            let call = RouteCall {
                method: parts.method.clone(),
                key: plugin.key.clone(),
                path: target.trim_start_matches('/').to_owned(),
                query: parts.uri.query().map(str::to_owned),
                headers: parts.headers.clone(),
                body: bytes,
            };
            Ok(
                match serve_route(
                    state,
                    call,
                    Some(&tenant.scope),
                    crate::manifest::RouteAudience::Private,
                )
                .await
                {
                    Ok(response) => response,
                    Err(error) => aphel_sdk::platform::api::error::app_error(error),
                },
            )
        })
    }
}
