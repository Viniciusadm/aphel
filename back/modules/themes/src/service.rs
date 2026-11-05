use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde_json::{Value, json};

use crate::MANAGE;
use crate::domain::{
    MAX_THEMES, SOURCE_EDITOR, THEME_ACTIVATED, THEME_ACTIVE, THEME_INSTALLED, THEME_INVALID,
    THEME_KEY_MISMATCH, THEME_LIMIT_REACHED, THEME_NOT_FOUND, THEME_REMOVED, THEME_UPDATED,
};
use crate::package::{FORMAT, FORMAT_VERSION, FontFile, ThemePackage};
use crate::store::{ThemeRecord, ThemeStore};
use crate::validate::{Issue, Validation, validate};
use aphel_kernel::Ctx;
use aphel_kernel::errors::{AppError, FieldViolation};
use aphel_kernel::records::{RecordMutation, publish_mutation};
use aphel_sdk::platform::application::access::Authorizer;
use aphel_sdk::platform::application::files::sha256_hex;
use aphel_sdk::platform::application::shared::{FieldChange, RecordRef};

#[derive(Debug, Clone)]
pub(crate) struct ThemeSummary {
    pub(crate) record: ThemeRecord,
    pub(crate) active: bool,
    pub(crate) font_count: usize,
}

#[derive(Debug, Clone)]
pub(crate) struct ThemeList {
    pub(crate) active_key: Option<String>,
    pub(crate) themes: Vec<ThemeSummary>,
}

#[derive(Debug)]
pub(crate) struct PreviewReport {
    pub(crate) issues: Vec<Issue>,
    pub(crate) warnings: Vec<Issue>,
    pub(crate) token_count: usize,
    pub(crate) fonts: Vec<PreviewFont>,
    pub(crate) installed_version: Option<String>,
}

#[derive(Debug)]
pub(crate) struct PreviewFont {
    pub(crate) family: String,
    pub(crate) weight: u16,
    pub(crate) style: String,
    pub(crate) bytes: usize,
}

fn invalid(issues: &[Issue]) -> AppError {
    AppError::invalid_fields_code(
        THEME_INVALID,
        "the theme is invalid",
        issues
            .iter()
            .map(|issue| FieldViolation::new(issue.path.clone(), issue.message.clone()))
            .collect(),
    )
}

fn not_found() -> AppError {
    AppError::not_found_code(THEME_NOT_FOUND, "the theme is not installed")
}

pub(crate) async fn list(ctx: &Ctx<'_>) -> Result<ThemeList, AppError> {
    ctx.scope.require(&MANAGE)?;
    let store = ThemeStore::of(ctx);
    let state = store.state().await?;
    let mut themes = Vec::new();
    for record in store.list().await? {
        let font_count = store.fonts(&record.key).await?.len();
        themes.push(ThemeSummary {
            active: state.active_key.as_deref() == Some(record.key.as_str()),
            font_count,
            record,
        });
    }
    Ok(ThemeList {
        active_key: state.active_key,
        themes,
    })
}

pub(crate) async fn preview(
    ctx: &Ctx<'_>,
    package: ThemePackage,
) -> Result<PreviewReport, AppError> {
    ctx.scope.require(&MANAGE)?;
    let validation = validate(&package);
    let installed_version = ThemeStore::of(ctx)
        .find(&package.id)
        .await?
        .map(|record| record.version);
    Ok(PreviewReport {
        token_count: validation.tokens.shared.len()
            + validation.tokens.light.len()
            + validation.tokens.dark.len(),
        fonts: validation
            .fonts
            .iter()
            .map(|font| PreviewFont {
                family: font.family.clone(),
                weight: font.weight,
                style: font.style.clone(),
                bytes: font.bytes.len(),
            })
            .collect(),
        issues: validation.issues,
        warnings: validation.warnings,
        installed_version,
    })
}

pub(crate) async fn install(
    ctx: &Ctx<'_>,
    package: ThemePackage,
    source: &str,
    ip: Option<String>,
) -> Result<ThemeSummary, AppError> {
    ctx.scope.require(&MANAGE)?;
    let validation = validate(&package);
    if !validation.issues.is_empty() {
        return Err(invalid(&validation.issues));
    }
    write(ctx, package, validation, source, ip).await
}

pub(crate) async fn save_from_editor(
    ctx: &Ctx<'_>,
    key: &str,
    package: ThemePackage,
    ip: Option<String>,
) -> Result<ThemeSummary, AppError> {
    if package.id != key {
        return Err(AppError::validation_code(
            THEME_KEY_MISMATCH,
            "the theme id must match the path",
        ));
    }
    install(ctx, package, SOURCE_EDITOR, ip).await
}

async fn write(
    ctx: &Ctx<'_>,
    package: ThemePackage,
    validation: Validation,
    source: &str,
    ip: Option<String>,
) -> Result<ThemeSummary, AppError> {
    let store = ThemeStore::of(ctx);
    let existing = store.find(&package.id).await?;
    if existing.is_none() && store.count().await? >= MAX_THEMES {
        return Err(AppError::conflict_code(
            THEME_LIMIT_REACHED,
            format!("at most {MAX_THEMES} themes can be installed"),
        ));
    }
    let now = ctx.now();
    let checksum = sha256_hex(
        &serde_json::to_vec(&json!({
            "package": package,
            "tokens": validation.tokens,
        }))
        .map_err(|error| AppError::Internal(error.to_string()))?,
    );
    let record = ThemeRecord {
        key: package.id.clone(),
        name: package.name.trim().to_owned(),
        version: package.version.clone(),
        author: package.author.trim().to_owned(),
        description: package.description.clone(),
        license: package.license.trim().to_owned(),
        min_aphel_version: package.min_aphel_version.clone(),
        tokens: validation.tokens,
        source: source.to_owned(),
        checksum,
        installed_by: Some(ctx.scope.user_id().as_str().to_owned()),
        created_at: existing.as_ref().map_or(now, |current| current.created_at),
        updated_at: now,
    };
    let state = store.state().await?;
    let mut tx = ctx.begin().await?;
    store.upsert(&mut tx, &record).await?;
    store
        .replace_fonts(&mut tx, &record.key, &validation.fonts)
        .await?;
    if state.active_key.as_deref() == Some(record.key.as_str()) {
        store.bump_version(&mut tx).await?;
    }
    let event_type = if existing.is_some() {
        THEME_UPDATED
    } else {
        THEME_INSTALLED
    };
    publish_mutation(
        ctx.events(),
        ctx.audit(),
        &mut tx,
        ctx.scope,
        now,
        RecordMutation {
            record: RecordRef::organization(),
            event_type,
            payload: json!({ "key": record.key, "version": record.version, "source": source }),
            changes: vec![FieldChange::new(
                format!("theme.{}", record.key),
                existing.map_or(Value::Null, |current| json!(current.version)),
                json!(record.version),
            )],
            ip,
        },
    )
    .await?;
    tx.commit().await?;
    Ok(ThemeSummary {
        active: state.active_key.as_deref() == Some(record.key.as_str()),
        font_count: validation.fonts.len(),
        record,
    })
}

pub(crate) async fn remove(ctx: &Ctx<'_>, key: &str, ip: Option<String>) -> Result<(), AppError> {
    ctx.scope.require(&MANAGE)?;
    let store = ThemeStore::of(ctx);
    let record = store.find(key).await?.ok_or_else(not_found)?;
    if store.state().await?.active_key.as_deref() == Some(key) {
        return Err(AppError::conflict_code(
            THEME_ACTIVE,
            "deactivate the theme before removing it",
        ));
    }
    let now = ctx.now();
    let mut tx = ctx.begin().await?;
    if !store.delete(&mut tx, key).await? {
        return Err(not_found());
    }
    publish_mutation(
        ctx.events(),
        ctx.audit(),
        &mut tx,
        ctx.scope,
        now,
        RecordMutation {
            record: RecordRef::organization(),
            event_type: THEME_REMOVED,
            payload: json!({ "key": key }),
            changes: vec![FieldChange::new(
                format!("theme.{key}"),
                json!(record.version),
                Value::Null,
            )],
            ip,
        },
    )
    .await?;
    Ok(tx.commit().await?)
}

pub(crate) async fn activate(
    ctx: &Ctx<'_>,
    key: Option<String>,
    ip: Option<String>,
) -> Result<ThemeList, AppError> {
    ctx.scope.require(&MANAGE)?;
    let store = ThemeStore::of(ctx);
    if let Some(key) = &key {
        store.find(key).await?.ok_or_else(not_found)?;
    }
    let previous = store.state().await?.active_key;
    if previous != key {
        let now = ctx.now();
        let mut tx = ctx.begin().await?;
        store.set_active(&mut tx, key.as_deref()).await?;
        publish_mutation(
            ctx.events(),
            ctx.audit(),
            &mut tx,
            ctx.scope,
            now,
            RecordMutation {
                record: RecordRef::organization(),
                event_type: THEME_ACTIVATED,
                payload: json!({ "key": key }),
                changes: vec![FieldChange::new(
                    "active_theme",
                    json!(previous),
                    json!(key),
                )],
                ip,
            },
        )
        .await?;
        tx.commit().await?;
    }
    list(ctx).await
}

pub(crate) async fn export(ctx: &Ctx<'_>, key: &str) -> Result<ThemePackage, AppError> {
    ctx.scope.require(&MANAGE)?;
    let store = ThemeStore::of(ctx);
    let record = store.find(key).await?.ok_or_else(not_found)?;
    let fonts = store.fonts(key).await?;
    Ok(ThemePackage {
        format: FORMAT.to_owned(),
        format_version: FORMAT_VERSION,
        id: record.key,
        name: record.name,
        version: record.version,
        author: record.author,
        description: record.description,
        license: record.license,
        min_aphel_version: record.min_aphel_version,
        tokens: record.tokens,
        fonts: fonts
            .into_iter()
            .map(|font| FontFile {
                family: font.family,
                weight: font.weight,
                style: font.style,
                format: "woff2".to_owned(),
                data: STANDARD.encode(font.bytes),
            })
            .collect(),
    })
}
