use std::io::Write;
use std::sync::LazyLock;
use std::time::Duration;

use axum::Json;
use axum::extract::multipart::MultipartRejection;
use axum::extract::{DefaultBodyLimit, Multipart, State};
use axum::http::StatusCode;
use serde::Serialize;
use utoipa::ToSchema;
use utoipa_axum::router::{OpenApiRouter, UtoipaMethodRouterExt};
use utoipa_axum::routes;

use crate::service::restore_installation;
use crate::{IdStrategy, RestoreCommand, RestoreEnv, TempFile};
use aphel_kernel::api::dto::common::{Data, ErrorEnvelope};
use aphel_kernel::api::error::ApiError;
use aphel_kernel::api::extractors::ClientInfo;
use aphel_kernel::api::payload_limit::payload_too_large;
use aphel_kernel::api::rate_limit::SharedRateLimiter;
use aphel_kernel::api::state::AppState;
use aphel_kernel::application::installer::InstallerUseCases;
use aphel_kernel::application::search::reindex_job;
use aphel_kernel::application::shared::JobQueue;
use aphel_kernel::errors::AppError;
use aphel_kernel::infrastructure::installer::MySqlInstallerPorts;

pub(crate) const TAG: &str = "installer";
const FILE_FIELD: &str = "file";
pub(crate) const MAX_RESTORE_UPLOAD_BYTES: usize = 2 * 1024 * 1024 * 1024;
const HOUR: Duration = Duration::from_secs(60 * 60);
const RATE_LIMIT_MESSAGE: &str = "too many restore attempts; wait a little and try again";

static RESTORES_PER_ADDRESS: LazyLock<SharedRateLimiter> =
    LazyLock::new(|| SharedRateLimiter::new("installer.restore_address", 5, HOUR));
static RESTORES: LazyLock<SharedRateLimiter> =
    LazyLock::new(|| SharedRateLimiter::new("installer.restore", 20, HOUR));

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct InstallerRestoreResponse {
    pub(crate) restored: bool,
}

#[derive(ToSchema)]
pub(crate) struct InstallerRestoreForm {
    #[schema(value_type = String, format = Binary)]
    pub(crate) file: TempFile,
}

pub(crate) fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(restore_backup).layer(DefaultBodyLimit::max(MAX_RESTORE_UPLOAD_BYTES)))
}

#[utoipa::path(
    post,
    path = "/installer/restore",
    tag = TAG,
    operation_id = "installer_restore",
    description = "Restores a full backup (`aphel.organization_export` zip) into an empty installation, as an alternative to `POST /installer`: creates the organization and its members (with their passwords), installs the configuration and restores records, relationships, history and files. Available only while the installer is (`GET /installer`). Sign in afterwards with an administrator of the backup. The upload is limited to 2 GiB (use `aphel org import` for larger backups), the archive to 500,000 entries and 64 GiB uncompressed, and the route to 5 attempts per hour per IP address and 20 per hour in total.",
    request_body(content = InstallerRestoreForm, content_type = "multipart/form-data"),
    responses(
        (status = 201, description = "Backup restored", body = Data<InstallerRestoreResponse>),
        (status = 404, description = "The instance is already installed (`not_found`)", body = ErrorEnvelope),
        (status = 409, description = "The installation is not empty (`organization_not_empty`)", body = ErrorEnvelope),
        (status = 413, description = "The upload is larger than 2 GiB (`payload_too_large`)", body = ErrorEnvelope),
        (status = 422, description = "Invalid form or invalid backup (`invalid_organization_export`)", body = ErrorEnvelope),
        (status = 429, description = "Too many restore attempts (`rate_limited`, with `Retry-After`)", body = ErrorEnvelope),
        (status = 503, description = "Database is not configured", body = ErrorEnvelope),
    )
)]
async fn restore_backup(
    State(state): State<AppState>,
    client: ClientInfo,
    multipart: Result<Multipart, MultipartRejection>,
) -> Result<(StatusCode, Json<Data<InstallerRestoreResponse>>), ApiError> {
    installer(&state)?.status().await?;
    RESTORES_PER_ADDRESS
        .check(client.ip_or_unknown(), RATE_LIMIT_MESSAGE)
        .await?;
    RESTORES.check("installer", RATE_LIMIT_MESSAGE).await?;
    let archive = receive_form(multipart).await?.file;
    let env = RestoreEnv::new(
        state.db()?,
        state.record_types.clone(),
        state.storage.clone(),
        state.events.clone(),
        state.audit.clone(),
    );
    let result = restore_installation(
        &env,
        RestoreCommand {
            archive: archive.path().to_path_buf(),
            ids: IdStrategy::Preserve,
        },
    )
    .await;
    drop(archive);
    state.record_types.invalidate();
    let report = result?;
    tracing::info!(
        source = %report.source_organization,
        export_id = %report.export_id,
        "installation restored from a backup"
    );
    if let Err(error) = state.jobs.enqueue_now(reindex_job(None, None)).await {
        tracing::warn!(%error, "failed to queue the search reindex after a restore");
    }
    Ok((
        StatusCode::CREATED,
        Json(Data::new(InstallerRestoreResponse { restored: true })),
    ))
}

fn installer(state: &AppState) -> Result<InstallerUseCases<MySqlInstallerPorts>, AppError> {
    Ok(InstallerUseCases::new(MySqlInstallerPorts::new(
        state.db()?,
        state.record_types.builtin(),
        state.events.clone(),
        state.audit.clone(),
        state.secrets.clone(),
    )))
}

async fn receive_form(
    multipart: Result<Multipart, MultipartRejection>,
) -> Result<InstallerRestoreForm, AppError> {
    let mut multipart =
        multipart.map_err(|rejection| AppError::Validation(rejection.body_text()))?;
    let mut archive: Option<TempFile> = None;
    while let Some(mut field) = multipart.next_field().await.map_err(invalid_form)? {
        if field.name() != Some(FILE_FIELD) || archive.is_some() {
            return Err(AppError::Validation(
                "unexpected or repeated form field".to_owned(),
            ));
        }
        let temp = TempFile::new("aphel-installer-restore", "zip");
        let mut file = temp.create()?;
        while let Some(chunk) = field.chunk().await.map_err(invalid_form)? {
            file.write_all(&chunk).map_err(|error| {
                AppError::Internal(format!("failed to store the uploaded backup: {error}"))
            })?;
        }
        archive = Some(temp);
    }
    let file =
        archive.ok_or_else(|| AppError::Validation("the form must include a file".to_owned()))?;
    Ok(InstallerRestoreForm { file })
}

fn invalid_form(error: axum::extract::multipart::MultipartError) -> AppError {
    if error.status() == StatusCode::PAYLOAD_TOO_LARGE {
        return payload_too_large();
    }
    AppError::Validation(format!("invalid multipart form: {}", error.body_text()))
}
