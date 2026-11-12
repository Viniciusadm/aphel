use aphel_contract_site::SiteContentSource;
use aphel_kernel::context::Ctx;
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::api::state::AppState;
use aphel_sdk::platform::application::organization_profile::OrganizationRepository;
use aphel_sdk::platform::domain::access::AccessScope;
use aphel_sdk::platform::infrastructure::repositories::mysql_organization_profile::MySqlOrganizationRepository;
use axum::body::Body;
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};

pub(crate) const LOGO_PATH: &str = "/logo";
const LOGO_CACHE_CONTROL: &str = "public, max-age=86400";
const THEME_CACHE_CONTROL: &str = "public, max-age=3600";

fn with_headers(
    mut response: Response,
    content_type: &str,
    cache_control: &'static str,
) -> Response {
    let headers = response.headers_mut();
    if let Ok(value) = HeaderValue::from_str(content_type) {
        headers.insert(header::CONTENT_TYPE, value);
    }
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static(cache_control),
    );
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    response
}

async fn load_logo(state: &AppState) -> Result<Option<Response>, AppError> {
    let organization = MySqlOrganizationRepository::new(state.db()?).find().await?;
    let Some(file_id) = organization.and_then(|organization| organization.logo_file_id) else {
        return Ok(None);
    };
    let scope = AccessScope::system().acting_as("site", "public");
    let download = Ctx::from_state(state, &scope)
        .await?
        .files()
        .await?
        .download(&scope, &file_id)
        .await?;
    if !download.file.mime_type.starts_with("image/") {
        return Ok(None);
    }
    Ok(Some(with_headers(
        (StatusCode::OK, Body::from_stream(download.content)).into_response(),
        &download.file.mime_type,
        LOGO_CACHE_CONTROL,
    )))
}

pub(crate) async fn logo(state: &AppState) -> Response {
    match load_logo(state).await {
        Ok(Some(response)) => response,
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(error) => {
            tracing::warn!(%error, "could not serve the site logo");
            StatusCode::NOT_FOUND.into_response()
        }
    }
}

pub(crate) async fn theme_file(state: &AppState, rest: &str) -> Response {
    let Some((owner, path)) = rest.split_once('/') else {
        return StatusCode::NOT_FOUND.into_response();
    };
    for source in state.app.contributions::<dyn SiteContentSource>() {
        match source.asset(owner, path).await {
            Ok(Some((content_type, content))) => {
                return with_headers(
                    (StatusCode::OK, content).into_response(),
                    &content_type,
                    THEME_CACHE_CONTROL,
                );
            }
            Ok(None) => {}
            Err(error) => {
                tracing::warn!(%error, owner, path, "could not serve a theme file");
                return StatusCode::NOT_FOUND.into_response();
            }
        }
    }
    StatusCode::NOT_FOUND.into_response()
}

pub(crate) async fn appearance_file(state: &AppState, key: &str) -> Response {
    match load_appearance_file(state, key).await {
        Ok(Some(response)) => response,
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(error) => {
            tracing::warn!(%error, "could not serve appearance image");
            StatusCode::SERVICE_UNAVAILABLE.into_response()
        }
    }
}

async fn load_appearance_file(state: &AppState, key: &str) -> Result<Option<Response>, AppError> {
    let parts: Vec<&str> = key.split('/').collect();
    let [entity, id, key] = parts.as_slice() else {
        return Ok(None);
    };
    if !["logo", "dark_logo", "icon", "opening_image"].contains(key)
        || ![
            aphel_contract_site::APPEARANCE_ENTITY,
            aphel_contract_site::PRESENTATION_ENTITY,
        ]
        .contains(entity)
        || !state.app.is_enabled("publication")
    {
        return Ok(None);
    }
    let scope = AccessScope::system().acting_as("site", "public");
    let id = uuid::Uuid::parse_str(id)
        .map_err(|_| AppError::Validation("invalid appearance record".into()))?;
    let Some(record) = state
        .registry()
        .await?
        .handler(entity)?
        .load(&aphel_sdk::records::RecordCtx::new(&scope, state), id)
        .await?
    else {
        return Ok(None);
    };
    if record[crate::appearance::STATE_FIELD] != "published" {
        return Ok(None);
    }
    let Some(file) = record[*key].as_str() else {
        return Ok(None);
    };
    let download = Ctx::from_state(state, &scope)
        .await?
        .files()
        .await?
        .download(&scope, file)
        .await?;
    if ![
        "image/png",
        "image/jpeg",
        "image/webp",
        "image/gif",
        "image/x-icon",
        "image/vnd.microsoft.icon",
    ]
    .contains(&download.file.mime_type.as_str())
    {
        return Ok(None);
    }
    Ok(Some(with_headers(
        (StatusCode::OK, Body::from_stream(download.content)).into_response(),
        &download.file.mime_type,
        "private, no-store",
    )))
}
