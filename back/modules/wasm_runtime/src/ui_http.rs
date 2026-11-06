use std::collections::BTreeMap;

use async_trait::async_trait;
use axum::Json;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use aphel_kernel::errors::AppError;
use aphel_sdk::platform::api::dto::common::{Data, ErrorEnvelope};
use aphel_sdk::platform::api::error::ApiError;
use aphel_sdk::platform::api::extractors::AuthContext;
use aphel_sdk::platform::api::state::AppState;
use aphel_sdk::platform::application::organization_profile::UiPluginDirectory;
use aphel_sdk::platform::infrastructure::organization_profile::find_ui_plugin_default;
use aphel_sdk::platform::state::runtime_pool;

use crate::http::TAG;
use crate::store::{self, InstalledPlugin};

pub const DEFAULT_CSS: &str = "default";

pub(crate) struct PluginUiDirectory;

#[async_trait]
impl UiPluginDirectory for PluginUiDirectory {
    async fn is_available(&self, key: &str) -> Result<bool, AppError> {
        let pool = runtime_pool()?;
        Ok(store::find(&pool, key)
            .await?
            .is_some_and(|plugin| plugin.enabled && plugin.manifest.ui.is_some()))
    }
}

pub(crate) fn public_router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new().routes(routes!(ui_stylesheet))
}

pub(crate) fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new().routes(routes!(list_ui_plugins))
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct UiPluginResponse {
    key: String,
    label: String,
    description: Option<String>,
    has_screens: bool,
    has_colors: bool,
    preview: BTreeMap<String, String>,
}

fn response(plugin: &InstalledPlugin) -> Option<UiPluginResponse> {
    let ui = plugin.manifest.ui.as_ref()?;
    Some(UiPluginResponse {
        key: plugin.key.clone(),
        label: ui.label.clone(),
        description: ui.description.clone(),
        has_screens: plugin.manifest.web.is_some(),
        has_colors: ui.has_tokens(),
        preview: ui.preview(),
    })
}

#[utoipa::path(
    get,
    path = "/ui-plugins",
    tag = TAG,
    operation_id = "ui_plugins_list",
    description = "Lists the enabled plugins that declare an interface (`[ui]`), which any signed-in user can choose in their preferences. Returns only the label, a short description, whether the plugin ships screens or colors, and a few preview colors, never the full token set.",
    security(("session_cookie" = [])),
    responses(
        (status = 200, description = "Interface plugins that can be chosen", body = Data<Vec<UiPluginResponse>>),
        (status = 401, description = "Missing or invalid session", body = ErrorEnvelope),
    )
)]
async fn list_ui_plugins(
    State(state): State<AppState>,
    _auth: AuthContext,
) -> Result<Json<Data<Vec<UiPluginResponse>>>, ApiError> {
    let plugins = store::list(&state.db()?).await?;
    Ok(Json(Data::new(
        plugins
            .iter()
            .filter(|plugin| plugin.enabled)
            .filter_map(response)
            .collect(),
    )))
}

#[utoipa::path(
    get,
    path = "/public/ui/{file}",
    tag = TAG,
    operation_id = "ui_plugin_stylesheet",
    description = "Serves the colors and fonts of an interface plugin as a stylesheet, from the tokens declared in its manifest (`<key>.css`). `default.css` serves the plugin chosen as the organization default, or an empty stylesheet when there is none. Public, because it is loaded before sign-in. The organization brand color still wins over these tokens.",
    params(("file" = String, Path, description = "`<plugin key>.css` or `default.css`")),
    responses(
        (status = 200, description = "Stylesheet", content_type = "text/css", body = String),
        (status = 404, description = "No enabled interface plugin with that key", body = ErrorEnvelope),
    )
)]
async fn ui_stylesheet(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(file): Path<String>,
) -> Result<Response, ApiError> {
    let name = file
        .strip_suffix(".css")
        .ok_or_else(|| AppError::NotFound("stylesheet not found".to_owned()))?;
    let pool = state.db()?;
    let plugin = if name == "core" {
        None
    } else if name == DEFAULT_CSS {
        match find_ui_plugin_default(&pool).await? {
            Some(key) => store::find(&pool, &key).await?,
            None => None,
        }
    } else {
        Some(
            store::find(&pool, name)
                .await?
                .ok_or_else(|| AppError::NotFound("stylesheet not found".to_owned()))?,
        )
    };
    let (css, tag) = match plugin.as_ref().filter(|plugin| plugin.enabled) {
        Some(plugin) => match plugin.manifest.ui.as_ref() {
            Some(ui) => (ui.css(), format!("\"ui-{}\"", &plugin.checksum[..12])),
            None => return Err(AppError::NotFound("stylesheet not found".to_owned()).into()),
        },
        None if name == DEFAULT_CSS || name == "core" => (String::new(), "\"ui-none\"".to_owned()),
        None => return Err(AppError::NotFound("stylesheet not found".to_owned()).into()),
    };
    let mut response_headers = HeaderMap::new();
    response_headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/css; charset=utf-8"),
    );
    response_headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-cache"));
    if let Ok(value) = HeaderValue::from_str(&tag) {
        if headers
            .get(header::IF_NONE_MATCH)
            .is_some_and(|candidate| *candidate == value)
        {
            return Ok((StatusCode::NOT_MODIFIED, response_headers).into_response());
        }
        response_headers.insert(header::ETAG, value);
    }
    Ok((response_headers, css).into_response())
}
