use std::sync::atomic::{AtomicU64, Ordering};

use axum::Json;
use axum::Router;
use axum::extract::{Request, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use chrono::Utc;
use sha2::{Digest, Sha256};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::api::dto::common::ErrorEnvelope;
use crate::api::dto::system::ReadinessResponse;
use crate::api::error::app_error;
use crate::api::extractors::auth::bearer_token;
use crate::api::state::AppState;
use crate::application::system::health::age_seconds;
use crate::application::system::metrics::{MetricsSnapshot, render_prometheus};
use crate::application::system::{APHEL_VERSION, Readiness, ReadinessUseCases, WORKER_ROLE};
use crate::errors::AppError;
use crate::infrastructure::auth::SystemClock;
use crate::infrastructure::system::{SystemHealthProbe, SystemSettings, monitor, probe};

pub const TAG: &str = "health";

const STATUS_CLASSES: [&str; 5] = ["1xx", "2xx", "3xx", "4xx", "5xx"];
static HTTP_RESPONSES: [AtomicU64; 5] = [
    AtomicU64::new(0),
    AtomicU64::new(0),
    AtomicU64::new(0),
    AtomicU64::new(0),
    AtomicU64::new(0),
];

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(health_check))
        .routes(routes!(readiness_check))
        .routes(routes!(metrics))
}

pub fn track_responses(router: Router) -> Router {
    router.layer(middleware::from_fn(count_response))
}

async fn count_response(request: Request, next: Next) -> Response {
    let response = next.run(request).await;
    let class = usize::from(response.status().as_u16() / 100).saturating_sub(1);
    if let Some(counter) = HTTP_RESPONSES.get(class) {
        counter.fetch_add(1, Ordering::Relaxed);
    }
    response
}

#[utoipa::path(
    get,
    path = "/_aphel/health",
    tag = TAG,
    operation_id = "health_check",
    responses(
        (status = 200, description = "Service is up", body = String, content_type = "text/plain", example = "ok"),
    )
)]
async fn health_check() -> &'static str {
    "ok"
}

pub async fn readiness(state: &AppState) -> Readiness {
    let settings = SystemSettings::from_env();
    ReadinessUseCases::new(
        SystemHealthProbe::new(state.db.clone(), state.storage.clone()),
        SystemClock,
        settings.readiness,
    )
    .check()
    .await
}

#[utoipa::path(
    get,
    path = "/_aphel/health/ready",
    tag = TAG,
    operation_id = "readiness_check",
    responses(
        (status = 200, description = "Every check passed: database, storage, worker heartbeat, scheduler and queue delay", body = ReadinessResponse),
        (status = 503, description = "At least one check failed", body = ReadinessResponse),
    )
)]
async fn readiness_check(State(state): State<AppState>) -> Response {
    let readiness = readiness(&state).await;
    let status = if readiness.is_ready() {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    let mut response = (status, Json(ReadinessResponse::from(readiness))).into_response();
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

#[utoipa::path(
    get,
    path = "/_aphel/metrics",
    tag = TAG,
    operation_id = "metrics",
    description = "Disabled unless `METRICS_TOKEN` is set; requires `Authorization: Bearer <METRICS_TOKEN>`.",
    responses(
        (status = 200, description = "Prometheus text exposition format (version 0.0.4)", body = String, content_type = "text/plain"),
        (status = 401, description = "Missing or wrong `Authorization: Bearer <METRICS_TOKEN>` (`metrics_unauthorized`)", body = ErrorEnvelope),
        (status = 404, description = "Metrics are disabled (`METRICS_TOKEN` not set)", body = ErrorEnvelope),
    )
)]
async fn metrics(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let settings = SystemSettings::from_env();
    let Some(expected) = settings.metrics_token.as_deref() else {
        return app_error(AppError::not_found_code(
            "metrics_disabled",
            "metrics are disabled",
        ));
    };
    let authorized = bearer_token(&headers).is_some_and(|token| same_secret(&token, expected));
    if !authorized {
        return app_error(AppError::unauthorized_code(
            "metrics_unauthorized",
            "a valid metrics token is required",
        ));
    }
    let snapshot = metrics_snapshot(&state, &settings).await;
    let mut response = render_prometheus(&snapshot).into_response();
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/plain; version=0.0.4; charset=utf-8"),
    );
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

fn same_secret(provided: &str, expected: &str) -> bool {
    let provided = Sha256::digest(provided.as_bytes());
    let expected = Sha256::digest(expected.as_bytes());
    provided
        .iter()
        .zip(expected.iter())
        .fold(0u8, |difference, (left, right)| difference | (left ^ right))
        == 0
}

async fn metrics_snapshot(state: &AppState, settings: &SystemSettings) -> MetricsSnapshot {
    let http_responses = STATUS_CLASSES
        .iter()
        .zip(HTTP_RESPONSES.iter())
        .map(|(class, counter)| (*class, counter.load(Ordering::Relaxed)))
        .collect();
    let mut snapshot = MetricsSnapshot {
        version: APHEL_VERSION.to_owned(),
        http_responses,
        ..MetricsSnapshot::default()
    };
    let Some(pool) = state.db.as_ref() else {
        return snapshot;
    };
    let now = Utc::now();
    snapshot.database_up = probe::database_up(pool).await;
    if !snapshot.database_up {
        return snapshot;
    }
    if let Ok(jobs) = monitor::job_counts(pool, now).await {
        snapshot.jobs = jobs;
    }
    if let Ok(oldest) = probe::oldest_ready_job(pool, now).await {
        snapshot.oldest_ready_age_seconds =
            Some(oldest.map_or(0, |run_at| age_seconds(now, run_at)));
    }
    if let Ok(Some(next_run_at)) = probe::oldest_due_schedule(pool).await {
        snapshot.scheduler_overdue_seconds = Some(age_seconds(now, next_run_at));
    }
    if let Ok(heartbeats) = monitor::heartbeats(pool).await {
        let limit = settings.readiness.worker_max_silence.as_secs() as i64;
        let workers: Vec<_> = heartbeats
            .iter()
            .filter(|heartbeat| heartbeat.role == WORKER_ROLE)
            .collect();
        snapshot.workers_alive = workers
            .iter()
            .filter(|heartbeat| age_seconds(now, heartbeat.last_seen_at) <= limit)
            .count() as u64;
        snapshot.worker_heartbeat_age_seconds = workers
            .iter()
            .map(|heartbeat| age_seconds(now, heartbeat.last_seen_at))
            .min();
    }
    snapshot
}
