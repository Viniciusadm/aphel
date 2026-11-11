use aphel_contract_publication::{PublishableDefinition, PublishableField};
use aphel_contract_site::{
    APPEARANCE_EDIT, APPEARANCE_ENTITY, AppearanceValues, PRESENTATION_EDIT, PRESENTATION_ENTITY,
    appearance_properties,
};
use aphel_sdk::AppError;
use aphel_sdk::module::{AppBuilder, HookCtx};
use aphel_sdk::platform::api::state::AppState;
use aphel_sdk::platform::application::access::Authorizer;
use aphel_sdk::platform::application::fields::{EntityDefinition, SystemField};
use aphel_sdk::platform::application::organization_admin::ListWindow;
use aphel_sdk::platform::application::query::{
    Filter, FilterOperator, RecordQuery, RecordQueryEngine,
};
use aphel_sdk::platform::application::shared::RecordType;
use aphel_sdk::platform::domain::access::{AccessScope, Permission, PermissionDefinition};
use aphel_sdk::platform::domain::fields::{FieldConfig, FieldType};
use aphel_sdk::records::{ChangedRecord, GenericRecordHandler, RecordValidating, query_ports};
use serde_json::Value;
use std::sync::Arc;

pub(crate) fn register(app: &mut AppBuilder) -> Result<(), AppError> {
    for (entity, permission) in [
        (APPEARANCE_ENTITY, APPEARANCE_EDIT),
        (PRESENTATION_ENTITY, PRESENTATION_EDIT),
    ] {
        let permission = Permission::from_static(permission);
        app.permission(PermissionDefinition::administrative(permission.clone()))
            .grant_to(["admin"]);
        let record_type = RecordType::from_static(entity);
        let mut definition = EntityDefinition::new(record_type.clone(), "", permission.clone())
            .managed(entity)
            .generic_mutations(permission.clone(), permission.clone())
            .update_permission(permission)
            .title_field("target")
            .index("target", &["target"], true)
            .system_field(
                SystemField::new("target", "Target", FieldType::Text, "target")?
                    .required()
                    .with_config(FieldConfig {
                        max_length: Some(160),
                        ..Default::default()
                    }),
            )
            .system_field(
                SystemField::new(
                    "draft_values",
                    "Appearance draft",
                    FieldType::LongText,
                    "draft_values",
                )?
                .with_config(FieldConfig {
                    max_length: Some(16000),
                    ..Default::default()
                }),
            );
        for key in ["created_at", "updated_at"] {
            definition = definition
                .system_field(SystemField::new(key, key, FieldType::DateTime, key)?.read_only());
        }
        let mut mappings = vec![PublishableField {
            draft: "draft_values".into(),
            published: "values".into(),
            file: false,
        }];
        for key in ["logo", "dark_logo", "icon", "opening_image"] {
            definition = definition.system_field(SystemField::new(
                &format!("draft_{key}"),
                key,
                FieldType::Text,
                format!("draft_{key}"),
            )?);
            mappings.push(PublishableField {
                draft: format!("draft_{key}"),
                published: key.into(),
                file: true,
            });
        }
        for (key, kind) in [
            ("values", FieldType::LongText),
            ("logo", FieldType::Text),
            ("dark_logo", FieldType::Text),
            ("icon", FieldType::Text),
            ("opening_image", FieldType::Text),
            ("state", FieldType::Select),
            ("published_at", FieldType::DateTime),
            ("revised_at", FieldType::DateTime),
        ] {
            let mut field = SystemField::new(key, key, kind, key)?.owner_managed();
            field.writers = vec!["publication".into()];
            if key == "state" {
                for state in aphel_contract_publication::STATES {
                    field = field.with_option(state, state, None);
                }
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
        app.contribute(Arc::new(PublishableDefinition {
            entity: entity.into(),
            format: 1,
            permission: if entity == APPEARANCE_ENTITY {
                APPEARANCE_EDIT
            } else {
                PRESENTATION_EDIT
            }
            .into(),
            fields: mappings,
            state: "state".into(),
            published_at: "published_at".into(),
            revised_at: "revised_at".into(),
        }));
    }
    app.contribute::<dyn aphel_sdk::provisioning::Provisioner>(Arc::new(AppearanceProvisioner));
    app.on(RecordValidating, 20, |ctx, args| Box::pin(guard(ctx, args)));
    Ok(())
}

pub(crate) fn validate(values: &Value, collection: bool) -> Result<AppearanceValues, AppError> {
    let values: AppearanceValues = serde_json::from_value(values.clone()).map_err(|_| {
        AppError::validation_code("site_appearance_invalid", "appearance must be an object")
    })?;
    let properties = appearance_properties();
    for (key, value) in &values.0 {
        let property = properties
            .iter()
            .find(|property| property.key == *key && (!collection || property.collection))
            .ok_or_else(|| {
                AppError::validation_code(
                    "site_appearance_invalid",
                    format!("unsupported appearance property: {key}"),
                )
            })?;
        let valid = match property.kind.as_str() {
            "theme" => value.as_str().is_some_and(|theme| {
                theme.len() <= 64
                    && theme.bytes().all(|byte| {
                        byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_'
                    })
            }),
            "color" => value.as_str().is_some_and(|text| {
                text.len() == 7
                    && text.starts_with('#')
                    && text[1..].bytes().all(|byte| byte.is_ascii_hexdigit())
            }),
            "select" => value
                .as_str()
                .is_some_and(|text| property.options.iter().any(|option| option == text)),
            "boolean" => value.is_boolean(),
            "frame" => {
                let mut settings = crate::settings::SiteSettings::default();
                let parsed = if key == "header" {
                    serde_json::from_value(value.clone()).map(|header| settings.header = header)
                } else if key == "footer" {
                    serde_json::from_value(value.clone()).map(|footer| settings.footer = footer)
                } else {
                    serde_json::from_value(value.clone())
                        .map(|translations| settings.translations = translations)
                };
                parsed.is_ok() && crate::settings::frame_violations(&settings).is_empty()
            }
            "text" => {
                value.is_null()
                    || value.as_str().is_some_and(|text| {
                        text.chars().count() <= if key == "tagline" { 160 } else { 120 }
                            && !text.chars().any(char::is_control)
                    })
            }
            _ => false,
        };
        if !valid {
            return Err(AppError::validation_code(
                "site_appearance_invalid",
                format!("invalid appearance property: {key}"),
            ));
        }
    }
    Ok(values)
}

async fn guard(ctx: &mut HookCtx<'_>, args: &ChangedRecord) -> Result<(), AppError> {
    let entity = args.record_type.as_str();
    if ![APPEARANCE_ENTITY, PRESENTATION_ENTITY].contains(&entity) {
        return Ok(());
    }
    let app = aphel_sdk::platform::modules::app();
    if !app.is_enabled("publication") {
        return Err(AppError::validation_code(
            "site_appearance_unavailable",
            "enable publication to customize appearance",
        ));
    }
    for change in &args.changes {
        if change.field == "target" && !change.old.is_null() {
            return Err(AppError::validation_code(
                "site_appearance_invalid",
                "the appearance target cannot change",
            ));
        }
    }
    let target = args
        .values
        .get("target")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if entity == APPEARANCE_ENTITY && target != "site" {
        return Err(AppError::validation_code(
            "site_appearance_invalid",
            "global appearance target must be site",
        ));
    }
    if entity == PRESENTATION_ENTITY {
        let Some((record_type, id)) = target.split_once(':') else {
            return Err(AppError::validation_code(
                "site_appearance_invalid",
                "a presentation needs an entity and record id",
            ));
        };
        let id = uuid::Uuid::parse_str(id).map_err(|_| {
            AppError::validation_code("site_appearance_invalid", "invalid presentation target")
        })?;
        let state = aphel_sdk::runtime()?;
        let settings =
            crate::settings::site_settings(&state.as_ref().organization_config.snapshot());
        let runtime = crate::runtime::current(app, settings.theme.as_deref()).await?;
        let permitted = runtime.prefixed.iter().any(|route| {
            route
                .page
                .prefix
                .as_ref()
                .is_some_and(|prefix| prefix.entity == record_type)
        });
        if !permitted {
            return Err(AppError::validation_code(
                "site_appearance_invalid",
                "this entity has no collection presentation",
            ));
        }
        let handler = args.registry.handler(record_type)?;
        if handler
            .load(
                &aphel_sdk::records::RecordCtx::new(ctx.scope, state.as_ref()),
                id,
            )
            .await?
            .is_none()
        {
            return Err(AppError::NotFound("presentation target not found".into()));
        }
    }
    for key in ["draft_values", "values"] {
        if let Some(text) = args.values.get(key).and_then(Value::as_str) {
            let value = serde_json::from_str(text).map_err(|_| {
                AppError::validation_code(
                    "site_appearance_invalid",
                    "appearance must contain valid JSON",
                )
            })?;
            validate(&value, entity == PRESENTATION_ENTITY)?;
            if let Some(theme) = value["theme"].as_str().filter(|theme| !theme.is_empty()) {
                let runtime = crate::runtime::current(app, None).await?;
                if !runtime
                    .themes
                    .iter()
                    .any(|candidate| candidate.key == theme)
                {
                    return Err(AppError::validation_code(
                        "site_appearance_invalid",
                        "the selected theme is not installed",
                    ));
                }
            }
        }
    }
    Ok(())
}

pub(crate) async fn record(
    state: &AppState,
    entity: &str,
    target: &str,
    scope: &AccessScope,
) -> Result<Option<Value>, AppError> {
    let ports = query_ports(state).await?;
    let records = RecordQueryEngine::new(&ports)
        .query(
            scope,
            entity,
            RecordQuery {
                filter: Some(Filter::condition(
                    "target",
                    FilterOperator::Eq,
                    target.into(),
                )),
                window: ListWindow {
                    offset: 0,
                    limit: 1,
                },
                ..Default::default()
            },
        )
        .await?;
    let Some(id) = records.items.into_iter().next() else {
        return Ok(None);
    };
    state
        .registry()
        .await?
        .handler(entity)?
        .load(&aphel_sdk::records::RecordCtx::new(scope, state), id)
        .await
}

pub(crate) async fn resolved(
    state: &AppState,
    target: Option<&str>,
    preview: Option<&AppearancePreview>,
) -> Result<Value, AppError> {
    let mut values: serde_json::Map<String, Value> = appearance_properties()
        .into_iter()
        .map(|property| (property.key, property.default))
        .collect();
    let settings = crate::settings::site_settings(&state.organization_config.snapshot());
    values.insert("tagline".into(), serde_json::json!(settings.tagline));
    let runtime = if let Some(preview) =
        preview.filter(|preview| preview.entity == APPEARANCE_ENTITY)
    {
        let fallback = crate::settings::site_settings(&state.organization_config.snapshot()).theme;
        let theme = preview
            .values
            .0
            .get("theme")
            .and_then(Value::as_str)
            .map(|theme| (!theme.is_empty()).then_some(theme))
            .unwrap_or(fallback.as_deref());
        crate::runtime::current(&state.app, theme).await?
    } else {
        crate::runtime::active(state).await?
    };
    values.extend(
        validate(
            &serde_json::to_value(&runtime.appearance.defaults)
                .map_err(|error| AppError::Internal(error.to_string()))?,
            false,
        )?
        .0,
    );
    if state.app.is_enabled("publication") {
        for (entity, key) in std::iter::once((APPEARANCE_ENTITY, "site"))
            .chain(target.map(|target| (PRESENTATION_ENTITY, target)))
        {
            if !preview.is_some_and(|preview| preview.entity == entity && preview.target == key)
                && let Some(record) = record(state, entity, key, &AccessScope::system()).await?
                && record["state"] == "published"
            {
                let text = record["values"].as_str().unwrap_or("{}");
                let appearance: Value = serde_json::from_str(text)
                    .map_err(|error| AppError::Internal(error.to_string()))?;
                values.extend(validate(&appearance, entity == PRESENTATION_ENTITY)?.0);
                {
                    for key in ["logo", "dark_logo", "icon", "opening_image"] {
                        if let Some(file) = record[key].as_str().filter(|file| !file.is_empty()) {
                            values.insert(
                                key.into(),
                                format!(
                                    "/_aphel/site/appearance/{entity}/{}/{key}?v={file}",
                                    record["id"].as_str().unwrap_or_default()
                                )
                                .into(),
                            );
                        }
                    }
                }
            }
            if let Some(preview) =
                preview.filter(|preview| preview.entity == entity && preview.target == key)
            {
                values.extend(preview.values.0.clone());
            }
        }
    }
    Ok(Value::Object(values))
}

#[derive(Clone)]
pub(crate) struct AppearancePreview {
    pub empty: bool,
    pub entity: String,
    pub target: String,
    pub values: AppearanceValues,
}

pub(crate) fn require(scope: &AccessScope, entity: &str) -> Result<(), AppError> {
    let permission = match entity {
        APPEARANCE_ENTITY => APPEARANCE_EDIT,
        PRESENTATION_ENTITY => PRESENTATION_EDIT,
        _ => {
            return Err(AppError::validation_code(
                "site_appearance_invalid",
                "unknown appearance entity",
            ));
        }
    };
    scope
        .require(&Permission::from_static(permission))
        .map(|_| ())
}

pub(crate) fn apply_frame(
    site: &mut Value,
    appearance: &Value,
    state: &AppState,
) -> Result<(), AppError> {
    let mut settings = crate::settings::site_settings(&state.organization_config.snapshot());
    if let Some(header) = appearance.get("header").filter(|value| !value.is_null()) {
        settings.header = serde_json::from_value(header.clone())
            .map_err(|error| AppError::Internal(error.to_string()))?;
    }
    if let Some(footer) = appearance.get("footer").filter(|value| !value.is_null()) {
        settings.footer = serde_json::from_value(footer.clone())
            .map_err(|error| AppError::Internal(error.to_string()))?;
    }
    if let Some(translations) = appearance
        .get("translations")
        .filter(|value| !value.is_null())
    {
        settings.translations = serde_json::from_value(translations.clone())
            .map_err(|error| AppError::Internal(error.to_string()))?;
    }
    if let Some(tagline) = appearance.get("tagline") {
        settings.tagline = tagline.as_str().map(str::to_owned);
    }
    let language = site["prefix"].as_str().unwrap_or_default().to_owned();
    let mount = site["mount"].as_str().unwrap_or_default().to_owned();
    let tools = aphel_sdk::module::public_app_paths(state);
    let prefixes = crate::frame::SitePrefixes {
        language: &language,
        mount: &mount,
        tools: &tools,
    };
    let texts = site["locale"]
        .as_str()
        .and_then(|locale| settings.translations.get(locale));
    site["tagline"] = serde_json::json!(crate::frame::tagline(&settings, texts));
    site["header"] = crate::frame::header(&settings, texts, &prefixes);
    site["footer"] = crate::frame::footer(&settings, texts, &prefixes, "");
    Ok(())
}

struct AppearanceProvisioner;

#[async_trait::async_trait]
impl aphel_sdk::provisioning::Provisioner for AppearanceProvisioner {
    fn kind(&self) -> &'static str {
        "site.appearance"
    }

    async fn apply(
        &self,
        ctx: &aphel_sdk::Ctx<'_>,
        request: &aphel_sdk::provisioning::ProvisionRequest,
    ) -> Result<aphel_sdk::provisioning::ProvisionOutcome, AppError> {
        use aphel_sdk::provisioning::{ProvisionAction, ProvisionOutcome};
        let entity = request.target.as_deref().unwrap_or(PRESENTATION_ENTITY);
        require(ctx.scope, entity)?;
        let target = request.data["target"].as_str().ok_or_else(|| {
            AppError::validation_code(
                "site_appearance_invalid",
                "appearance provisioning needs a target",
            )
        })?;
        let values = validate(&request.data["values"], entity == PRESENTATION_ENTITY)?;
        let state = aphel_sdk::runtime()?;
        let seed_key = format!("{}:{}", request.pack, request.key);
        let serialized = serde_json::to_string(&values)
            .map_err(|error| AppError::Internal(error.to_string()))?;
        let existing = record(state.as_ref(), entity, target, ctx.scope).await?;
        let record = if let Some(existing) = existing {
            if existing["seed_key"] != seed_key
                || existing["state"] == "published"
                || existing["draft_values"] != serialized
            {
                return Ok(ProvisionOutcome {
                    action: ProvisionAction::Kept,
                    id: existing["id"].as_str().unwrap_or_default().into(),
                    applied: request.data.clone(),
                });
            }
            existing
        } else {
            let scope = ctx.scope.clone().acting_as("module", "site");
            let initiator = aphel_sdk::platform::application::shared::Actor::from_scope(ctx.scope);
            let execution = aphel_sdk::platform::application::shared::ExecutionContext::module(
                "site",
                "site.appearance.provision",
                Some(&initiator),
            )?;
            execution.scope(state.as_ref().registry().await?.handler(entity)?.create(&aphel_sdk::records::RecordCtx::new(&scope, state.as_ref()), serde_json::json!({ "target": target, "draft_values": serialized, "seed_key": seed_key }))).await?
        };
        let capture: aphel_sdk::OperationKey<Value, Value> =
            aphel_sdk::operation!(aphel_contract_publication::OP_CAPTURE);
        let activate: aphel_sdk::OperationKey<Value, Value> =
            aphel_sdk::operation!(aphel_contract_publication::OP_REQUEST);
        let revision = ctx.call(capture, serde_json::json!({ "record_type": entity, "record_id": record["id"], "expected_version": record["updated_at"] })).await?;
        let result = ctx.call(activate, serde_json::json!({ "revision_id": revision["id"], "idempotency_key": format!("pack:{}:{}", request.pack, request.key) })).await?;
        if result["status"] != "completed" {
            return Err(AppError::validation_code(
                "site_appearance_pending",
                "the appearance is awaiting publication; resolve the publication request before completing the pack",
            ));
        }
        Ok(ProvisionOutcome {
            action: ProvisionAction::Created,
            id: record["id"].as_str().unwrap_or_default().into(),
            applied: request.data.clone(),
        })
    }
}

pub(crate) fn probe(failures: &mut Vec<String>) {
    let app = aphel_sdk::platform::modules::app();
    let registry = app.record_types();
    for entity in [APPEARANCE_ENTITY, PRESENTATION_ENTITY] {
        let Ok(definition) = registry.require(entity) else {
            failures.push(format!("appearance: missing {entity}"));
            continue;
        };
        if let Err(error) = aphel_sdk::platform::infrastructure::fields::managed_schema(definition)
        {
            failures.push(format!(
                "appearance: invalid storage schema for {entity}: {error}"
            ));
        }
        for key in [
            "values",
            "logo",
            "dark_logo",
            "icon",
            "opening_image",
            "state",
            "published_at",
            "revised_at",
        ] {
            if !definition.system_field_by_key(key).is_some_and(|field| {
                field.writable && field.owner_managed && field.writers == ["publication"]
            }) {
                failures.push(format!(
                    "appearance: {entity}.{key} must be publication-owned"
                ));
            }
        }
        if !app
            .all_contributions::<PublishableDefinition>()
            .iter()
            .any(|(_, definition)| definition.entity == entity)
        {
            failures.push(format!(
                "appearance: missing publication definition for {entity}"
            ));
        }
    }
    for value in [
        serde_json::json!({"unknown": "x"}),
        serde_json::json!({"background": "red;display:none"}),
        serde_json::json!({"columns": "99"}),
    ] {
        if validate(&value, false).is_ok() {
            failures.push("appearance: unsafe or unsupported property accepted".into());
        }
    }
    if validate(&serde_json::json!({"background": "#ffffff"}), true).is_ok() {
        failures.push("appearance: collection accepted a global identity override".into());
    }
    for preset in aphel_contract_site::appearance_presets() {
        if validate(
            &serde_json::to_value(preset.values).unwrap_or_default(),
            true,
        )
        .is_err()
        {
            failures.push(format!("appearance: invalid preset {}", preset.key));
        }
    }
}

pub(crate) async fn presentation_values(
    state: &AppState,
    target: &str,
    preview: Option<&AppearancePreview>,
) -> Result<std::collections::BTreeMap<String, Value>, AppError> {
    if let Some(preview) =
        preview.filter(|preview| preview.entity == PRESENTATION_ENTITY && preview.target == target)
    {
        return Ok(preview.values.0.clone());
    }
    if !state.app.is_enabled("publication") {
        return Ok(Default::default());
    }
    let Some(record) = record(state, PRESENTATION_ENTITY, target, &AccessScope::system()).await?
    else {
        return Ok(Default::default());
    };
    if record["state"] != "published" {
        return Ok(Default::default());
    }
    let values = serde_json::from_str(record["values"].as_str().unwrap_or("{}"))
        .map_err(|error| AppError::Internal(error.to_string()))?;
    Ok(validate(&values, true)?.0)
}
