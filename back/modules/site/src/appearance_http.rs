use aphel_contract_site::{
    APPEARANCE_ENTITY, PRESENTATION_ENTITY, appearance_presets, appearance_properties,
};
use aphel_sdk::AppError;
use aphel_sdk::platform::api::dto::common::{Data, ErrorEnvelope};
use aphel_sdk::platform::api::error::ApiError;
use aphel_sdk::platform::api::extractors::AuthContext;
use aphel_sdk::platform::api::state::AppState;
use axum::http::{HeaderValue, header};
use axum::response::{IntoResponse, Response};
use axum::{Json, extract::State};
use base64::Engine;
use serde::Deserialize;
use serde_json::{Value, json};
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};

pub(crate) fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(catalog))
        .routes(routes!(render))
}

#[utoipa::path(get, path = "/site/appearance", tag = "site", operation_id = "site_appearance_catalog",
    security(("session_cookie" = [])), responses((status = 200, body = Data<Value>), (status = 403, body = ErrorEnvelope)))]
async fn catalog(
    State(state): State<AppState>,
    tenant: AuthContext,
) -> Result<Json<Data<Value>>, ApiError> {
    let global = crate::appearance::require(&tenant.scope, APPEARANCE_ENTITY).is_ok();
    let collection = crate::appearance::require(&tenant.scope, PRESENTATION_ENTITY).is_ok();
    if !global && !collection {
        crate::appearance::require(&tenant.scope, APPEARANCE_ENTITY)?;
    }
    let runtime = crate::runtime::active(&state).await?;
    let mut targets = Vec::new();
    if global {
        targets.push(
            json!({ "entity": APPEARANCE_ENTITY, "target": "site", "label": null, "path": "/" }),
        );
    }
    if collection {
        for route in &runtime.prefixed {
            if let Some(prefix) = &route.page.prefix {
                for record in
                    crate::prefixes::candidates(&state, &runtime, &route.owner, prefix).await?
                {
                    let Some(id) = record["id"].as_str() else {
                        continue;
                    };
                    let target = format!("{}:{id}", prefix.entity);
                    if targets.iter().any(|item| item["target"] == target) {
                        continue;
                    }
                    let path = record[&prefix.field].as_str().unwrap_or_default();
                    targets.push(json!({ "entity": PRESENTATION_ENTITY, "target": target, "label": record.get("name").or_else(|| record.get("title")), "path": format!("/{path}") }));
                }
            }
        }
    }
    use aphel_sdk::platform::application::organization_profile::OrganizationRepository;
    let organization = aphel_sdk::platform::infrastructure::repositories::mysql_organization_profile::MySqlOrganizationRepository::new(state.db()?).find().await?;
    let locale = organization
        .map(|organization| organization.locale)
        .unwrap_or_else(|| "en".into());
    let settings = crate::settings::site_settings(&state.organization_config.snapshot());
    let mut properties = appearance_properties();
    if let Some(property) = properties
        .iter_mut()
        .find(|property| property.key == "theme")
    {
        property.kind = "select".into();
        property.options = std::iter::once(String::new())
            .chain(runtime.themes.iter().map(|theme| theme.key.clone()))
            .collect();
    }
    let mut theme_catalogs = serde_json::Map::new();
    for key in std::iter::once("").chain(runtime.themes.iter().map(|theme| theme.key.as_str())) {
        let selected =
            crate::runtime::current(&state.app, (!key.is_empty()).then_some(key)).await?;
        theme_catalogs.insert(key.to_owned(), json!({ "properties": selected.appearance.properties, "defaults": selected.appearance.defaults }));
    }
    Ok(Json(Data::new(
        json!({ "available": state.app.is_enabled("publication"), "properties": properties.into_iter().filter(|property| property.key != "show_terms" || state.app.is_enabled("tagging")).collect::<Vec<_>>(), "presets": appearance_presets(), "targets": targets, "theme_catalogs": theme_catalogs, "theme_properties": runtime.appearance.properties, "theme_defaults": runtime.appearance.defaults, "themes": runtime.themes, "frame": { "configuration": { "site": settings }, "overview": { "base_url": state.organization_config.snapshot().site_base_url(), "default_language": locale } } }),
    )))
}

#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
struct RenderAppearance {
    #[serde(default)]
    empty: bool,
    entity: String,
    target: String,
    values: Value,
    path: String,
    #[serde(default)]
    files: std::collections::BTreeMap<String, Option<String>>,
}

#[utoipa::path(post, path = "/site/appearance/preview", tag = "site", operation_id = "site_appearance_preview",
    request_body = RenderAppearance, security(("session_cookie" = [])),
    responses((status = 200, body = Data<Value>), (status = 403, body = ErrorEnvelope), (status = 422, body = ErrorEnvelope)))]
async fn render(
    State(state): State<AppState>,
    tenant: AuthContext,
    Json(input): Json<RenderAppearance>,
) -> Result<Response, ApiError> {
    crate::appearance::require(&tenant.scope, &input.entity)?;
    if !state.app.is_enabled("publication") {
        return Err(AppError::validation_code(
            "site_appearance_unavailable",
            "enable publication to customize appearance",
        )
        .into());
    }
    let mut values =
        crate::appearance::validate(&input.values, input.entity == PRESENTATION_ENTITY)?;
    for (key, file) in &input.files {
        if !["logo", "dark_logo", "icon", "opening_image"].contains(&key.as_str())
            || input.entity != APPEARANCE_ENTITY && key != "opening_image" && file.is_some()
        {
            return Err(AppError::validation_code(
                "site_appearance_invalid",
                "unsupported appearance image",
            )
            .into());
        }
        if let Some(file) = file {
            let id = uuid::Uuid::parse_str(file).map_err(|_| {
                AppError::validation_code("site_appearance_invalid", "invalid appearance image")
            })?;
            let ctx = aphel_sdk::Ctx::from_state(&state, &tenant.scope).await?;
            let download = ctx
                .files()
                .await?
                .download(&tenant.scope, &id.to_string())
                .await?;
            if !["image/png", "image/jpeg", "image/gif", "image/webp"]
                .contains(&download.file.mime_type.as_str())
                || download.file.size > 2 * 1024 * 1024
            {
                return Err(AppError::validation_code(
                    "site_appearance_invalid",
                    "appearance files must be images",
                )
                .into());
            }
            let mime = download.file.mime_type.clone();
            let bytes = crate::images::collect(download.content).await?;
            let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
            values
                .0
                .insert(key.clone(), format!("data:{mime};base64,{encoded}").into());
        }
    }
    let path = crate::preview::local_path(&input.path)
        .ok_or_else(|| AppError::Validation("preview needs a local path".into()))?;
    let runtime = if let Some(theme) = values.0.get("theme").and_then(Value::as_str) {
        crate::runtime::current(&state.app, (!theme.is_empty()).then_some(theme)).await?
    } else if input.entity == APPEARANCE_ENTITY {
        let theme = crate::settings::site_settings(&state.organization_config.snapshot()).theme;
        crate::runtime::current(&state.app, theme.as_deref()).await?
    } else {
        crate::runtime::active(&state).await?
    };
    let preview = crate::preview::Preview {
        theme: None,
        record: None,
        appearance: Some(crate::appearance::AppearancePreview {
            empty: input.empty,
            entity: input.entity,
            target: input.target,
            values,
        }),
    };
    let html = crate::handler::render_draft(&state, &runtime, &preview, path)
        .await?
        .ok_or_else(|| AppError::NotFound("no public page answers this path".into()))?;
    let html = html.replacen(
        "<head>",
        "<head><meta name=\"robots\" content=\"noindex,nofollow\">",
        1,
    );
    let mut response = Json(Data::new(json!({ "html": html }))).into_response();
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("private, no-store"),
    );
    response.headers_mut().insert(
        "x-robots-tag",
        HeaderValue::from_static("noindex, nofollow"),
    );
    Ok(response)
}
