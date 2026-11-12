use std::collections::BTreeMap;
use std::sync::Arc;

use aphel_contract_publication::{MODULE as PUBLICATION_MODULE, PublishableDeclaration};
use aphel_contract_site::{DESIGN_EDIT, DESIGN_ENTITY, DESIGN_KINDS};
use aphel_sdk::AppError;
use aphel_sdk::module::{AppBuilder, HookCtx};
use aphel_sdk::platform::api::state::AppState;
use aphel_sdk::platform::application::fields::{
    EntityDefinition, PUBLISHED_AT_FIELD, REVISED_AT_FIELD, SystemField, publication_state_field,
};
use aphel_sdk::platform::application::organization_admin::ListWindow;
use aphel_sdk::platform::application::query::{
    Filter, FilterOperator, RecordQuery, RecordQueryEngine,
};
use aphel_sdk::platform::application::shared::RecordType;
use aphel_sdk::platform::domain::access::{AccessScope, Permission, PermissionDefinition};
use aphel_sdk::platform::domain::fields::{FieldConfig, FieldType};
use aphel_sdk::records::{ChangedRecord, GenericRecordHandler, RecordValidating, query_ports};
use serde_json::{Value, json};

use crate::appearance::STATE_FIELD;

const MAX_CONTENT_CHARS: u32 = 65_535;
const MAX_DESIGNS: u64 = 200;
pub(crate) const DEFAULT_HEADER: &str = include_str!("../theme/parts/header.json");
pub(crate) const DEFAULT_FOOTER: &str = include_str!("../theme/parts/footer.json");

pub(crate) fn register(app: &mut AppBuilder) -> Result<(), AppError> {
    let permission = Permission::from_static(DESIGN_EDIT);
    app.permission(PermissionDefinition::administrative(permission.clone()))
        .grant_to(["admin"]);
    let record_type = RecordType::from_static(DESIGN_ENTITY);
    let mut kind = SystemField::new("kind", "Kind", FieldType::Select, "kind")?.required();
    for option in DESIGN_KINDS {
        kind = kind.with_option(*option, *option, None);
    }
    let mut definition = EntityDefinition::new(record_type.clone(), "", permission.clone())
        .managed(DESIGN_ENTITY)
        .generic_mutations(permission.clone(), permission.clone())
        .update_permission(permission.clone())
        .title_field("name")
        .index("target", &["target"], true)
        .system_field(
            SystemField::new("target", "Target", FieldType::Text, "target")?
                .required()
                .with_config(FieldConfig {
                    max_length: Some(160),
                    ..Default::default()
                }),
        )
        .system_field(kind)
        .system_field(
            SystemField::new("name", "Name", FieldType::Text, "name")?.with_config(FieldConfig {
                max_length: Some(160),
                ..Default::default()
            }),
        )
        .system_field(
            SystemField::new(
                "draft_content",
                "Draft content",
                FieldType::LongText,
                "draft_content",
            )?
            .with_config(FieldConfig {
                max_length: Some(MAX_CONTENT_CHARS),
                ..Default::default()
            }),
        );
    for key in ["created_at", "updated_at"] {
        definition = definition
            .system_field(SystemField::new(key, key, FieldType::DateTime, key)?.read_only());
    }
    let state_field = publication_state_field(PUBLICATION_MODULE);
    for (key, kind) in [
        ("content", FieldType::LongText),
        (state_field.as_str(), FieldType::Select),
        (PUBLISHED_AT_FIELD, FieldType::DateTime),
        (REVISED_AT_FIELD, FieldType::DateTime),
    ] {
        let mut field = SystemField::new(key, key, kind, key)?.owner_managed();
        field.writers = vec![PUBLICATION_MODULE.into()];
        if key == state_field {
            for state in aphel_contract_publication::STATES {
                field = field.with_option(state, state, None);
            }
        }
        if key == "content" {
            field = field.with_config(FieldConfig {
                max_length: Some(MAX_CONTENT_CHARS),
                ..Default::default()
            });
        }
        definition = definition.system_field(field);
    }
    let mut seed = SystemField::new(
        "seed_key",
        "Provisioning resource",
        FieldType::Text,
        "seed_key",
    )?
    .owner_managed();
    seed.writers = vec!["site".into()];
    definition = definition.system_field(seed);
    app.entity(definition.handler(Arc::new(GenericRecordHandler::new(record_type))));
    app.contribute(Arc::new(PublishableDeclaration {
        entity: DESIGN_ENTITY.into(),
        format: 1,
        permission: DESIGN_EDIT.into(),
    }));
    app.on(RecordValidating, 21, |ctx, args| Box::pin(guard(ctx, args)));
    Ok(())
}

pub(crate) fn validate_content(kind: &str, text: &str) -> Result<String, AppError> {
    let invalid = |message: String| AppError::validation_code("site_design_invalid", message);
    if kind == "font" {
        return font_content(text).map_err(invalid);
    }
    if kind == "menu" {
        return menu_items(text)
            .map(|items| Value::Array(items).to_string())
            .map_err(invalid);
    }
    let parsed: Value = serde_json::from_str(if text.trim().is_empty() { "{}" } else { text })
        .map_err(|_| invalid("the design is not valid JSON".into()))?;
    let blocks = parsed
        .get("blocks")
        .cloned()
        .unwrap_or(Value::Array(Vec::new()));
    let normalized = aphel_blocks::blocks::normalize(&blocks.to_string())
        .map_err(|error| invalid(error.message))?;
    let mut clean = serde_json::Map::new();
    clean.insert(
        "blocks".into(),
        serde_json::from_str(&normalized).unwrap_or_default(),
    );
    if let Some(css) = parsed.get("css").and_then(Value::as_str) {
        let css = aphel_contract_site::styles::validate_css(css).map_err(invalid)?;
        if !css.trim().is_empty() {
            clean.insert("css".into(), Value::String(css));
        }
    }
    if let Some(name) = parsed.get("name").and_then(Value::as_str) {
        if name.chars().count() > 80 || name.chars().any(char::is_control) {
            return Err(invalid(
                "the design name has more than 80 characters".into(),
            ));
        }
        clean.insert("name".into(), Value::String(name.trim().to_owned()));
    }
    for key in ["header", "footer", "category"] {
        if let Some(value) = parsed.get(key).and_then(Value::as_str) {
            let valid = value.len() <= 64
                && value.chars().all(|character| {
                    character.is_ascii_alphanumeric() || matches!(character, '-' | '_')
                });
            if !valid {
                return Err(invalid(format!("{key} is not a valid name")));
            }
            clean.insert(key.into(), Value::String(value.to_owned()));
        }
    }
    Ok(Value::Object(clean).to_string())
}

fn font_content(text: &str) -> Result<String, String> {
    let parsed: Value = serde_json::from_str(if text.trim().is_empty() { "{}" } else { text })
        .map_err(|_| "the font is not valid JSON".to_owned())?;
    let family = parsed["family"].as_str().unwrap_or_default().trim();
    let valid_family = !family.is_empty()
        && family.chars().count() <= 60
        && family
            .chars()
            .all(|character| character.is_alphanumeric() || matches!(character, ' ' | '-'));
    if !valid_family {
        return Err("the font family uses up to 60 letters, digits, spaces and -".into());
    }
    let faces = parsed["faces"].as_array().cloned().unwrap_or_default();
    if faces.len() > 12 {
        return Err("a font has at most 12 files".into());
    }
    let mut clean = Vec::new();
    for face in faces {
        let file = face["file"].as_str().unwrap_or_default();
        if uuid::Uuid::parse_str(file).is_err() {
            return Err("every font file must be uploaded first".into());
        }
        let weight = face["weight"].as_u64().unwrap_or(400);
        if !(100..=900).contains(&weight) || weight % 100 != 0 {
            return Err("the font weight goes from 100 to 900".into());
        }
        let style = face["style"].as_str().unwrap_or("normal");
        if !["normal", "italic"].contains(&style) {
            return Err("the font style is normal or italic".into());
        }
        clean.push(json!({ "file": file, "weight": weight, "style": style }));
    }
    let fallback = parsed["fallback"].as_str().unwrap_or("sans-serif");
    if !["sans-serif", "serif", "monospace"].contains(&fallback) {
        return Err("the fallback is sans-serif, serif or monospace".into());
    }
    Ok(json!({ "family": family, "faces": clean, "fallback": fallback }).to_string())
}

pub(crate) async fn uploaded_fonts(state: &AppState) -> Result<Value, AppError> {
    let mut fonts = serde_json::Map::new();
    for (key, font) in published(state, "font", None).await? {
        let id = font["id"].as_str().unwrap_or_default();
        let faces: Vec<Value> = font["faces"]
            .as_array()
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .map(|face| {
                json!({
                    "url": format!("/_aphel/site/fonts/{id}/{}", face["file"].as_str().unwrap_or_default()),
                    "weight": face["weight"],
                    "style": face["style"],
                })
            })
            .collect();
        fonts.insert(
            key,
            json!({ "family": font["family"], "fallback": font["fallback"], "faces": faces }),
        );
    }
    Ok(Value::Object(fonts))
}

pub(crate) async fn font_file(
    state: &AppState,
    record_id: &str,
    file_id: &str,
) -> Result<Option<axum::response::Response>, AppError> {
    use axum::response::IntoResponse;
    let Some(record) = query(
        state,
        Filter::condition("id", FilterOperator::Eq, record_id.into()),
        1,
    )
    .await?
    .into_iter()
    .next() else {
        return Ok(None);
    };
    if record["kind"] != "font" || record[STATE_FIELD] != "published" {
        return Ok(None);
    }
    let content: Value =
        serde_json::from_str(record["content"].as_str().unwrap_or("{}")).unwrap_or_default();
    let listed = content["faces"]
        .as_array()
        .is_some_and(|faces| faces.iter().any(|face| face["file"] == file_id));
    if !listed {
        return Ok(None);
    }
    let scope = AccessScope::system().acting_as("site", "public");
    let download = aphel_sdk::Ctx::from_state(state, &scope)
        .await?
        .files()
        .await?
        .download(&scope, file_id)
        .await?;
    let mime = download.file.mime_type.clone();
    if !["font/woff2", "font/woff"].contains(&mime.as_str()) {
        return Ok(None);
    }
    let mut response = (
        axum::http::StatusCode::OK,
        axum::body::Body::from_stream(download.content),
    )
        .into_response();
    let headers = response.headers_mut();
    if let Ok(value) = axum::http::HeaderValue::from_str(&mime) {
        headers.insert(axum::http::header::CONTENT_TYPE, value);
    }
    headers.insert(
        axum::http::header::CACHE_CONTROL,
        axum::http::HeaderValue::from_static("public, max-age=31536000, immutable"),
    );
    headers.insert(
        axum::http::header::ACCESS_CONTROL_ALLOW_ORIGIN,
        axum::http::HeaderValue::from_static("*"),
    );
    Ok(Some(response))
}

fn menu_items(text: &str) -> Result<Vec<Value>, String> {
    let parsed: Value = serde_json::from_str(if text.trim().is_empty() { "[]" } else { text })
        .map_err(|_| "the menu is not valid JSON".to_owned())?;
    let items = parsed.as_array().ok_or("the menu must be a list")?;
    if items.len() > 60 {
        return Err("a menu has at most 60 items".into());
    }
    items
        .iter()
        .map(|item| {
            let label = item["label"].as_str().unwrap_or_default().trim();
            let url = item["url"].as_str().unwrap_or_default().trim();
            if label.is_empty() || label.chars().count() > 80 {
                return Err("every menu item needs a label of up to 80 characters".to_owned());
            }
            if !aphel_blocks::blocks::is_safe_link(url) {
                return Err(format!(
                    "{label}: use a site address such as /contact or an https:// address"
                ));
            }
            let children = match item.get("children") {
                Some(Value::Array(children)) if !children.is_empty() => {
                    menu_items(&Value::Array(children.clone()).to_string())?
                        .into_iter()
                        .map(|mut child| {
                            if let Some(child) = child.as_object_mut() {
                                child.remove("children");
                            }
                            child
                        })
                        .collect()
                }
                _ => Vec::new(),
            };
            Ok(json!({ "label": label, "url": url, "children": children }))
        })
        .collect()
}

async fn guard(ctx: &mut HookCtx<'_>, args: &ChangedRecord) -> Result<(), AppError> {
    if args.record_type.as_str() != DESIGN_ENTITY {
        return Ok(());
    }
    let field = |key: &str| {
        args.values
            .get(key)
            .and_then(Value::as_str)
            .map(str::to_owned)
            .unwrap_or_default()
    };
    let target = field("target");
    let kind = field("kind");
    let kind = kind.as_str();
    let valid_target = target
        .split_once(':')
        .is_some_and(|(prefix, key)| prefix == kind && !key.is_empty() && key.len() <= 120);
    if !valid_target {
        return Err(AppError::validation_code(
            "site_design_invalid",
            "the design target must be <kind>:<key>",
        ));
    }
    if args.values.get("draft_content").is_some() {
        validate_content(kind, &field("draft_content"))?;
    }
    let _ = ctx;
    Ok(())
}

async fn query(state: &AppState, filter: Filter, limit: u64) -> Result<Vec<Value>, AppError> {
    let scope = AccessScope::system();
    let ports = query_ports(state).await?;
    let found = RecordQueryEngine::new(&ports)
        .query(
            &scope,
            DESIGN_ENTITY,
            RecordQuery {
                filter: Some(filter),
                window: ListWindow { offset: 0, limit },
                ..Default::default()
            },
        )
        .await?;
    let handler = state.registry().await?.handler(DESIGN_ENTITY)?;
    let mut records = Vec::new();
    for id in found.items {
        if let Some(record) = handler
            .load(&aphel_sdk::records::RecordCtx::new(&scope, state), id)
            .await?
        {
            records.push(record);
        }
    }
    Ok(records)
}

#[derive(Clone)]
pub(crate) struct DesignPreview {
    pub target: String,
    pub content: String,
}

async fn shaped(state: &AppState, record: &Value, content: &str) -> Result<Value, AppError> {
    let mut value: Value = serde_json::from_str(content).unwrap_or_else(|_| json!({}));
    let id = record["id"].as_str().unwrap_or_default();
    value["id"] = json!(id);
    value["name"] = record["name"].clone();
    value["images"] = json!(crate::images::record_images(state, DESIGN_ENTITY, id).await?);
    Ok(value)
}

pub(crate) async fn published(
    state: &AppState,
    kind: &str,
    preview: Option<&DesignPreview>,
) -> Result<BTreeMap<String, Value>, AppError> {
    if !state.app.is_enabled("publication") {
        return Ok(BTreeMap::new());
    }
    let mut found = BTreeMap::new();
    for record in query(
        state,
        Filter::condition("kind", FilterOperator::Eq, kind.into()),
        MAX_DESIGNS,
    )
    .await?
    {
        let target = record["target"].as_str().unwrap_or_default().to_owned();
        let key = target
            .split_once(':')
            .map(|(_, key)| key.to_owned())
            .unwrap_or_default();
        let content = match preview.filter(|preview| preview.target == target) {
            Some(preview) => Some(preview.content.clone()),
            None if record[STATE_FIELD] == "published" => {
                record["content"].as_str().map(str::to_owned)
            }
            None => None,
        };
        if let Some(content) = content {
            let value = if kind == "menu" {
                json!({ "name": record["name"], "items": serde_json::from_str::<Value>(&content).unwrap_or_default() })
            } else {
                shaped(state, &record, &content).await?
            };
            found.insert(key, value);
        }
    }
    Ok(found)
}

pub(crate) async fn record(state: &AppState, target: &str) -> Result<Option<Value>, AppError> {
    Ok(query(
        state,
        Filter::condition("target", FilterOperator::Eq, target.into()),
        1,
    )
    .await?
    .into_iter()
    .next())
}

pub(crate) async fn image_is_published(
    state: &AppState,
    record_id: &str,
) -> Result<bool, AppError> {
    Ok(query(
        state,
        Filter::condition("id", FilterOperator::Eq, record_id.into()),
        1,
    )
    .await?
    .first()
    .is_some_and(|record| record[STATE_FIELD] == "published"))
}

pub(crate) fn apply_preview(site: &mut Value, preview: &DesignPreview) {
    let Some((kind, key)) = preview.target.split_once(':') else {
        return;
    };
    let collection = match kind {
        "part" => "parts",
        "pattern" => "patterns",
        "menu" => "menus",
        _ => return,
    };
    let parsed: Value = serde_json::from_str(&preview.content).unwrap_or_default();
    let value = if kind == "menu" {
        json!({ "name": key, "items": parsed })
    } else {
        let mut value = parsed;
        value["images"] = site[collection][key]["images"].clone();
        if value["images"].is_null() {
            value["images"] = json!([]);
        }
        value
    };
    if !site[collection].is_object() {
        site[collection] = json!({});
    }
    site[collection][key] = value;
}

pub(crate) fn default_part(key: &str) -> Option<Value> {
    let source = match key {
        "header" => DEFAULT_HEADER,
        "footer" => DEFAULT_FOOTER,
        _ => return None,
    };
    let normalized = validate_content("part", source).ok()?;
    let mut value: Value = serde_json::from_str(&normalized).ok()?;
    value["images"] = json!([]);
    Some(value)
}

fn localized_menu(items: &Value, prefix: &str, current: &str) -> Value {
    let current = format!("{prefix}{current}");
    Value::Array(
        items
            .as_array()
            .map(|items| {
                items
                    .iter()
                    .map(|item| {
                        let raw = item["url"].as_str().unwrap_or_default();
                        let url = if raw.starts_with('/') && !raw.starts_with("//") {
                            format!("{prefix}{raw}")
                        } else {
                            raw.to_owned()
                        };
                        let children = localized_menu(
                            &item["children"],
                            prefix,
                            current.trim_start_matches(prefix),
                        );
                        let child_active = children.as_array().is_some_and(|children| {
                            children.iter().any(|child| child["active"] == true)
                        });
                        json!({
                            "label": item["label"],
                            "url": url,
                            "active": url == current || child_active,
                            "children": children,
                        })
                    })
                    .collect()
            })
            .unwrap_or_default(),
    )
}

pub(crate) async fn decorate(
    state: &AppState,
    runtime: &crate::runtime::SiteRuntime,
    site: &mut Value,
    prefix: &str,
    current: &str,
) -> Result<(), AppError> {
    let packaged = |kind: &str| -> BTreeMap<String, Value> {
        runtime
            .designs
            .iter()
            .chain(runtime.theme_designs.iter())
            .filter_map(|(target, value)| {
                target
                    .strip_prefix(&format!("{kind}:"))
                    .map(|key| (key.to_owned(), value.clone()))
            })
            .collect()
    };
    let mut parts = packaged("part");
    parts.extend(published(state, "part", None).await?);
    for key in ["header", "footer"] {
        if !parts.contains_key(key)
            && let Some(part) = default_part(key)
        {
            parts.insert(key.to_owned(), part);
        }
    }
    let menus: serde_json::Map<String, Value> = published(state, "menu", None)
        .await?
        .into_iter()
        .map(|(key, menu)| {
            let items = localized_menu(&menu["items"], prefix, current);
            (key, json!({ "name": menu["name"], "items": items }))
        })
        .collect();
    site["parts"] = json!(parts);
    site["menus"] = Value::Object(menus);
    let mut patterns = packaged("pattern");
    patterns.extend(published(state, "pattern", None).await?);
    site["patterns"] = json!(patterns);
    Ok(())
}

pub(crate) async fn template(
    state: &AppState,
    key: &str,
    preview: Option<&DesignPreview>,
) -> Result<Option<Value>, AppError> {
    let target = format!("template:{key}");
    if let Some(preview) = preview.filter(|preview| preview.target == target) {
        let record = record(state, &target).await?.unwrap_or_else(|| json!({}));
        return Ok(Some(shaped(state, &record, &preview.content).await?));
    }
    let Some(record) = record(state, &target).await? else {
        return Ok(None);
    };
    if record[STATE_FIELD] != "published" {
        return Ok(None);
    }
    let Some(content) = record["content"].as_str() else {
        return Ok(None);
    };
    Ok(Some(shaped(state, &record, content).await?))
}
