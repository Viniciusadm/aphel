use std::collections::BTreeMap;

use aphel_contract_site::{DESIGN_EDIT, PageSource};
use aphel_sdk::AppError;
use aphel_sdk::platform::api::dto::common::{Data, ErrorEnvelope};
use aphel_sdk::platform::api::error::ApiError;
use aphel_sdk::platform::api::extractors::AuthContext;
use aphel_sdk::platform::api::state::AppState;
use aphel_sdk::platform::application::access::Authorizer;
use aphel_sdk::platform::domain::access::Permission;
use axum::http::{HeaderValue, header};
use axum::response::{IntoResponse, Response};
use axum::{Json, extract::State};
use serde::Deserialize;
use serde_json::{Value, json};
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};

pub(crate) fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(catalog))
        .routes(routes!(preview))
        .routes(routes!(export))
}

fn packaged(runtime: &crate::runtime::SiteRuntime, target: &str) -> Option<Value> {
    runtime
        .theme_designs
        .get(target)
        .or_else(|| runtime.designs.get(target))
        .cloned()
}

#[utoipa::path(get, path = "/site/designs/catalog", tag = "site", operation_id = "site_designs_catalog",
    security(("session_cookie" = [])), responses((status = 200, body = Data<Value>), (status = 403, body = ErrorEnvelope)))]
async fn catalog(
    State(state): State<AppState>,
    tenant: AuthContext,
) -> Result<Json<Data<Value>>, ApiError> {
    tenant
        .scope
        .require(&Permission::from_static(DESIGN_EDIT))?;
    let runtime = crate::runtime::active(&state).await?;
    let mut templates = Vec::new();
    for route in runtime.routes.iter().chain(runtime.prefixed.iter()) {
        let key = format!("{}:{}", route.owner, route.page.key);
        let listing = matches!(route.page.source, PageSource::List { .. });
        let fallback = std::iter::once(key.clone())
            .chain(
                listing
                    .then(|| ["archive", "index"].map(|name| format!("{}:{name}", route.owner)))
                    .into_iter()
                    .flatten(),
            )
            .find_map(|candidate| packaged(&runtime, &format!("template:{candidate}")));
        let variant_prefix = format!("template:{key}@");
        let variants: Vec<Value> = runtime
            .designs
            .keys()
            .chain(runtime.theme_designs.keys())
            .filter_map(|target| target.strip_prefix(&variant_prefix))
            .map(|variant| {
                json!({
                    "variant": variant,
                    "default": packaged(&runtime, &format!("{variant_prefix}{variant}")),
                })
            })
            .collect();
        templates.push(json!({
            "key": key,
            "owner": route.owner,
            "owner_name": runtime.owner_name(&route.owner),
            "page": route.page.key,
            "entity": route.page.source.entity(),
            "listing": listing,
            "prefixed": route.page.prefix.is_some(),
            "prefix_entity": route.page.prefix.as_ref().map(|prefix| prefix.entity.clone()),
            "path": route.page.path,
            "default": fallback,
            "variants": variants,
        }));
    }
    let mut parts: BTreeMap<String, Value> = BTreeMap::new();
    for key in ["header", "footer"] {
        if let Some(part) = crate::design::default_part(key) {
            parts.insert(key.into(), part);
        }
    }
    let mut patterns: BTreeMap<String, Value> = BTreeMap::new();
    for (target, value) in runtime.designs.iter().chain(runtime.theme_designs.iter()) {
        if let Some(key) = target.strip_prefix("part:") {
            parts.insert(key.into(), value.clone());
        } else if let Some(key) = target.strip_prefix("pattern:") {
            patterns.insert(key.into(), value.clone());
        }
    }
    Ok(Json(Data::new(json!({
        "templates": templates,
        "parts": parts,
        "patterns": patterns,
    }))))
}

fn theme_key(name: &str) -> String {
    let slug: String = name
        .to_lowercase()
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character
            } else {
                '_'
            }
        })
        .collect();
    let slug = slug.trim_matches('_').chars().take(40).collect::<String>();
    format!(
        "theme_{}",
        if slug.is_empty() { "site".into() } else { slug }
    )
}

fn toml_text(text: &str) -> String {
    format!("\"{}\"", text.replace('\\', "\\\\").replace('"', "\\\""))
}

#[utoipa::path(get, path = "/site/designs/export", tag = "site", operation_id = "site_designs_export",
    description = "Downloads the published site design (styles, templates, parts and patterns) as a theme plugin package. Uploaded fonts fall back to their generic family. Requires `site.design.edit`.",
    security(("session_cookie" = [])), responses((status = 200, description = "Theme plugin package", content_type = "application/zip"), (status = 403, body = ErrorEnvelope)))]
async fn export(State(state): State<AppState>, tenant: AuthContext) -> Result<Response, ApiError> {
    use std::io::Write;
    tenant
        .scope
        .require(&Permission::from_static(DESIGN_EDIT))?;
    let appearance = crate::appearance::resolved(&state, None).await?;
    let mut styles = appearance["styles"].clone();
    let fonts = styles["uploaded_fonts"].clone();
    if let Some(object) = styles.as_object_mut() {
        object.remove("uploaded_fonts");
    }
    if let Some(families) = styles["families"].as_array_mut() {
        for family in families {
            if let Some(key) = family["stack"]
                .as_str()
                .and_then(|stack| stack.strip_prefix("font:"))
            {
                let fallback = match fonts[key]["fallback"].as_str() {
                    Some("serif") => "transitional",
                    Some("monospace") => "mono",
                    _ => "sans",
                };
                family["stack"] = json!(fallback);
            }
        }
    }
    let site_name = appearance["name"]
        .as_str()
        .filter(|name| !name.trim().is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| "Site".into());
    let key = theme_key(&site_name);
    let manifest = format!(
        "key = {key}\nname = {name}\nversion = \"1.0.0\"\napi = 1\nrequires_aphel = \">=0.1.1\"\ndescription = {description}\n\n[site.theme]\nname = {name}\ndescription = {description}\n",
        key = toml_text(&key),
        name = toml_text(&format!("Tema {site_name}")),
        description =
            toml_text("Exportado do editor do site: estilos, templates, partes e padrões."),
    );
    let mut files: Vec<(String, String)> = vec![
        ("plugin.toml".into(), manifest),
        (
            "site/theme/styles.json".into(),
            serde_json::to_string_pretty(&styles).unwrap_or_default(),
        ),
    ];
    for (kind, folder) in [
        ("template", "templates"),
        ("part", "parts"),
        ("pattern", "patterns"),
    ] {
        for (design_key, mut value) in crate::design::published(&state, kind, None).await? {
            if let Some(object) = value.as_object_mut() {
                object.remove("images");
                object.remove("id");
            }
            let file = if kind == "template" {
                design_key.replacen(':', ".", 1)
            } else {
                design_key
            };
            files.push((
                format!("site/theme/{folder}/{file}.json"),
                serde_json::to_string_pretty(&value).unwrap_or_default(),
            ));
        }
    }
    let mut buffer = std::io::Cursor::new(Vec::new());
    {
        let mut writer = zip::ZipWriter::new(&mut buffer);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        for (name, content) in files {
            writer
                .start_file(name, options)
                .map_err(|error| AppError::Internal(error.to_string()))?;
            writer
                .write_all(content.as_bytes())
                .map_err(|error| AppError::Internal(error.to_string()))?;
        }
        writer
            .finish()
            .map_err(|error| AppError::Internal(error.to_string()))?;
    }
    let mut response = (axum::http::StatusCode::OK, buffer.into_inner()).into_response();
    let headers = response.headers_mut();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/zip"),
    );
    if let Ok(value) =
        HeaderValue::from_str(&format!("attachment; filename=\"{key}-1.0.0.aphelplugin\""))
    {
        headers.insert(header::CONTENT_DISPOSITION, value);
    }
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("private, no-store"),
    );
    Ok(response)
}

#[derive(Deserialize, ToSchema)]
struct DesignPreviewInput {
    target: String,
    content: String,
    path: String,
}

#[utoipa::path(post, path = "/site/designs/preview", tag = "site", operation_id = "site_designs_preview",
    security(("session_cookie" = [])), request_body = DesignPreviewInput,
    responses((status = 200, body = Data<Value>), (status = 403, body = ErrorEnvelope)))]
async fn preview(
    State(state): State<AppState>,
    tenant: AuthContext,
    Json(input): Json<DesignPreviewInput>,
) -> Result<Response, ApiError> {
    tenant
        .scope
        .require(&Permission::from_static(DESIGN_EDIT))?;
    let kind = input
        .target
        .split_once(':')
        .map(|(kind, _)| kind.to_owned())
        .unwrap_or_default();
    let content = crate::design::validate_content(&kind, &input.content)?;
    let path = crate::preview::local_path(&input.path)
        .ok_or_else(|| AppError::Validation("preview needs a local path".into()))?;
    let runtime = crate::runtime::active(&state).await?;
    let previewing = crate::preview::Preview {
        theme: None,
        record: None,
        appearance: None,
        design: Some(crate::design::DesignPreview {
            target: input.target,
            content,
        }),
    };
    let html = crate::handler::render_draft(&state, &runtime, &previewing, path)
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
    Ok(response)
}
