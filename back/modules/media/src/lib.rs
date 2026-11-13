pub(crate) mod http;

use std::sync::Arc;

use aphel_kernel::Ctx;
use aphel_kernel::modules::{AppBuilder, Manifest, Module};
use aphel_sdk::AppError;
use aphel_sdk::module::HookCtx;
use aphel_sdk::platform::application::fields::{EntityDefinition, SystemField};
use aphel_sdk::platform::application::shared::RecordType;
use aphel_sdk::platform::domain::access::{AccessScope, Permission, PermissionDefinition};
use aphel_sdk::platform::domain::fields::{FieldConfig, FieldType};
use aphel_sdk::platform::modules::builder::APHEL_VERSION;
use aphel_sdk::records::{GenericRecordHandler, RecordCtx, RecordDeleting, RemovedRecord};
use rust_decimal::Decimal;
use serde_json::Value;

pub(crate) const KEY: &str = "media";
pub(crate) const ASSET: &str = "media.asset";
pub(crate) const FOLDER: &str = "media.folder";
pub(crate) const READ: &str = "media.assets.read";
pub(crate) const MANAGE: &str = "media.assets.manage";
pub(crate) const IN_USE: &str = "media_asset_in_use";

pub struct MediaModule;

fn text(key: &str, label: &str, max_length: u32) -> Result<SystemField, AppError> {
    Ok(
        SystemField::new(key, label, FieldType::Text, key)?.with_config(FieldConfig {
            max_length: Some(max_length),
            ..Default::default()
        }),
    )
}

fn focal(key: &str, label: &str) -> Result<SystemField, AppError> {
    Ok(
        SystemField::new(key, label, FieldType::Number, key)?.with_config(FieldConfig {
            min: Some(Decimal::ZERO),
            max: Some(Decimal::ONE_HUNDRED),
            decimals: Some(0),
            ..Default::default()
        }),
    )
}

fn definition() -> Result<EntityDefinition, AppError> {
    let read = Permission::from_static(READ);
    let manage = Permission::from_static(MANAGE);
    let record_type = RecordType::from_static(ASSET);
    let mut definition = EntityDefinition::new(record_type.clone(), "", read)
        .managed(ASSET)
        .generic_mutations(manage.clone(), manage.clone())
        .update_permission(manage)
        .title_field("name")
        .system_field(text("name", "Name", 160)?.required())
        .system_field(SystemField::new("file", "File", FieldType::File, "file")?)
        .system_field(text("content_type", "Type", 127)?)
        .system_field(text("alt", "Alternative text", 300)?)
        .system_field(text("caption", "Caption", 300)?)
        .system_field(text("credit", "Credit", 160)?)
        .system_field(focal("focal_x", "Horizontal focal point")?)
        .system_field(focal("focal_y", "Vertical focal point")?)
        .system_field(text("folder_id", "Folder", 36)?)
        .reference(aphel_sdk::fields::SoftReference {
            field: "folder_id".to_owned(),
            target: aphel_sdk::fields::ReferenceTarget::Entity {
                entity: FOLDER.to_owned(),
            },
            on_delete: aphel_sdk::fields::DeletePolicy::Restrict,
        });
    for field in definition.system_fields.iter_mut() {
        if field.key.as_str() == "name" || field.key.as_str() == "alt" {
            field.searchable = true;
        }
    }
    for key in ["created_at", "updated_at"] {
        definition = definition
            .system_field(SystemField::new(key, key, FieldType::DateTime, key)?.read_only());
    }
    Ok(definition.handler(Arc::new(GenericRecordHandler::new(record_type))))
}

fn folder_definition() -> Result<EntityDefinition, AppError> {
    let read = Permission::from_static(READ);
    let manage = Permission::from_static(MANAGE);
    let record_type = RecordType::from_static(FOLDER);
    let mut definition = EntityDefinition::new(record_type.clone(), "", read)
        .managed(FOLDER)
        .generic_mutations(manage.clone(), manage.clone())
        .update_permission(manage)
        .title_field("name")
        .system_field(text("name", "Name", 80)?.required())
        .system_field(text("parent_id", "Parent folder", 36)?)
        .reference(aphel_sdk::fields::SoftReference {
            field: "parent_id".to_owned(),
            target: aphel_sdk::fields::ReferenceTarget::Entity {
                entity: FOLDER.to_owned(),
            },
            on_delete: aphel_sdk::fields::DeletePolicy::Restrict,
        });
    for field in definition.system_fields.iter_mut() {
        if field.key.as_str() == "name" {
            field.searchable = true;
        }
    }
    for key in ["created_at", "updated_at"] {
        definition = definition
            .system_field(SystemField::new(key, key, FieldType::DateTime, key)?.read_only());
    }
    Ok(definition.handler(Arc::new(GenericRecordHandler::new(record_type))))
}

pub(crate) async fn usage(
    scope: &AccessScope,
    file_id: &str,
) -> Result<Vec<aphel_sdk::platform::application::shared::RecordRef>, AppError> {
    let state = aphel_sdk::runtime()?;
    let ctx = Ctx::from_state(state.as_ref(), scope).await?;
    Ok(ctx
        .files()
        .await?
        .attached_records(scope, file_id)
        .await?
        .into_iter()
        .filter(|record| record.record_type.as_str() != ASSET)
        .collect())
}

async fn guard(_ctx: &mut HookCtx<'_>, args: &RemovedRecord) -> Result<(), AppError> {
    if args.record_type.as_str() != ASSET {
        return Ok(());
    }
    let state = aphel_sdk::runtime()?;
    let scope = AccessScope::system();
    let Some(asset) = state
        .registry()
        .await?
        .handler(ASSET)?
        .load(
            &RecordCtx::new(&scope, state.as_ref()),
            args.record.record_id,
        )
        .await?
    else {
        return Ok(());
    };
    let Some(file) = asset.get("file").and_then(Value::as_str) else {
        return Ok(());
    };
    let used = usage(&scope, file).await?;
    if used.is_empty() {
        return Ok(());
    }
    Err(AppError::conflict_code(
        IN_USE,
        format!(
            "the file is used by {} record(s); remove it from them before deleting it from the library",
            used.len()
        ),
    ))
}

impl Module for MediaModule {
    fn manifest(&self) -> Manifest {
        Manifest::new(KEY, "Media library", APHEL_VERSION)
            .description(
                "Library of images and files with alternative text, caption, credit and focal point, reused by any record without uploading again",
            )
            .plugin_toml(include_str!("../plugin.toml"))
    }

    fn register(&self, app: &mut AppBuilder) -> Result<(), AppError> {
        app.permission(PermissionDefinition::administrative(
            Permission::from_static(READ),
        ))
        .grant_to(["admin", "manager", "member"]);
        app.permission(PermissionDefinition::administrative(
            Permission::from_static(MANAGE),
        ))
        .grant_to(["admin", "manager"]);
        app.entity(folder_definition()?);
        app.entity(definition()?);
        app.on(RecordDeleting, 20, |ctx, args| Box::pin(guard(ctx, args)));
        app.openapi_tag(
            http::TAG,
            "Library of reusable images and files with their descriptions",
        )
        .routes(|_| http::router());
        Ok(())
    }
}
