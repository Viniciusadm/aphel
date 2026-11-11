use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use axum::extract::{Request, State};
use axum::http::{StatusCode, header};
use axum::response::{Html, IntoResponse, Response};
use tokio::task::JoinHandle;

use crate::api::error::app_error;
use crate::config::AppConfig;
use crate::errors::AppError;
use crate::tenancy::{ControlPlane, TenantDirectory, TenantRuntimes, normalize_host};
use tower::ServiceExt;

const DIRECTORY_REFRESH_INTERVAL: Duration = Duration::from_secs(5);

struct Dispatcher {
    directory: Arc<TenantDirectory>,
    runtimes: Arc<TenantRuntimes>,
    console: Option<(String, Router)>,
}

pub fn router(
    control: &ControlPlane,
    config: &AppConfig,
    directory: Arc<TenantDirectory>,
    runtimes: Arc<TenantRuntimes>,
) -> Router {
    let console = config
        .console_host
        .as_deref()
        .and_then(normalize_host)
        .map(|host| {
            let console = crate::tenancy::console::router(
                control.clone(),
                config.clone(),
                directory.clone(),
                runtimes.clone(),
                &host,
            );
            (host, console)
        });
    let dispatcher = Arc::new(Dispatcher {
        directory,
        runtimes,
        console,
    });
    crate::api::with_process_layers(Router::new().fallback(dispatch).with_state(dispatcher))
}

pub fn spawn_directory_refresh(
    directory: Arc<TenantDirectory>,
    runtimes: Arc<TenantRuntimes>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(DIRECTORY_REFRESH_INTERVAL);
        loop {
            interval.tick().await;
            match directory.refresh().await {
                Ok(()) => runtimes.retain(&directory),
                Err(error) => tracing::warn!("could not refresh the tenant directory: {error}"),
            }
        }
    })
}

async fn dispatch(State(dispatcher): State<Arc<Dispatcher>>, request: Request) -> Response {
    let path = request.uri().path().to_owned();
    let Some(host) = request_host(&request) else {
        return unknown_host(&path);
    };
    if let Some((console_host, console)) = &dispatcher.console
        && *console_host == host
    {
        return match console.clone().oneshot(request).await {
            Ok(response) => response,
            Err(infallible) => match infallible {},
        };
    }
    let Some(tenant) = dispatcher.directory.resolve(&host) else {
        return unknown_host(&path);
    };
    if !tenant.status.serves_requests() {
        return tenant_unavailable(&path);
    }
    match dispatcher.runtimes.get(&tenant).await {
        Ok(runtime) => runtime.serve(request).await,
        Err(error) => {
            tracing::error!(
                tenant = tenant.slug,
                "could not start the tenant runtime: {error}"
            );
            tenant_unavailable(&path)
        }
    }
}

fn request_host(request: &Request) -> Option<String> {
    request
        .uri()
        .authority()
        .map(|authority| authority.as_str().to_owned())
        .or_else(|| {
            request
                .headers()
                .get(header::HOST)
                .and_then(|value| value.to_str().ok())
                .map(str::to_owned)
        })
        .and_then(|host| normalize_host(&host))
}

fn is_api(path: &str) -> bool {
    path == "/api" || path.starts_with("/api/")
}

fn unknown_host(path: &str) -> Response {
    if is_api(path) {
        return app_error(AppError::not_found_code(
            "tenant_not_found",
            "no organization is served at this address",
        ));
    }
    plain_page(
        StatusCode::NOT_FOUND,
        "Address not found",
        "No organization is served at this address.",
    )
}

fn tenant_unavailable(path: &str) -> Response {
    if is_api(path) {
        return app_error(AppError::unavailable_code(
            "tenant_unavailable",
            "this organization is temporarily unavailable",
        ));
    }
    plain_page(
        StatusCode::SERVICE_UNAVAILABLE,
        "Temporarily unavailable",
        "This organization is temporarily unavailable. Please try again later.",
    )
}

fn plain_page(status: StatusCode, title: &str, message: &str) -> Response {
    let body = format!(
        "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width, initial-scale=1\"><title>{title}</title></head><body style=\"font-family: system-ui, sans-serif; max-width: 32rem; margin: 20vh auto; padding: 0 1rem; color: #222\"><h1 style=\"font-size: 1.25rem\">{title}</h1><p>{message}</p></body></html>"
    );
    (status, [(header::CACHE_CONTROL, "no-store")], Html(body)).into_response()
}
