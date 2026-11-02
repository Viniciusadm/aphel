use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use utoipa::ToSchema;

use crate::application::module_admin::{missing_required, parse_settings, with_defaults};
use crate::domain::Secret;
use crate::errors::{AppError, FieldViolation};
use crate::modules::App;

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct InstallerProfile {
    pub key: String,
    pub modules: Vec<String>,
}

pub fn profiles() -> Vec<InstallerProfile> {
    crate::modules::app()
        .contributions::<dyn super::InstallerBlueprintSource>()
        .into_iter()
        .flat_map(|source| source.profiles())
        .collect()
}

pub struct PreparedModule {
    pub key: String,
    pub values: Map<String, Value>,
    pub secrets: Vec<(String, Secret)>,
}

pub fn prepare(
    app: &App,
    selected: &[String],
    settings: &BTreeMap<String, Map<String, Value>>,
) -> Result<Vec<PreparedModule>, AppError> {
    let mut keys: BTreeSet<String> = selected.iter().cloned().collect();
    keys.extend(
        app.modules()
            .iter()
            .filter(|entry| entry.manifest.required)
            .map(|entry| entry.key().to_owned()),
    );
    loop {
        let before = keys.len();
        for key in keys.clone() {
            let entry = app.module(&key).ok_or_else(|| {
                AppError::invalid_fields(
                    "invalid modules",
                    vec![FieldViolation::new(
                        "modules",
                        format!("unknown module: {key}"),
                    )],
                )
            })?;
            if !entry.is_compatible() {
                return Err(AppError::invalid_fields(
                    "invalid modules",
                    vec![FieldViolation::new(
                        "modules",
                        format!("incompatible module: {key}"),
                    )],
                ));
            }
            keys.extend(
                entry
                    .manifest
                    .dependencies
                    .iter()
                    .map(|dependency| dependency.key.to_owned()),
            );
        }
        if keys.len() == before {
            break;
        }
    }
    if settings.keys().any(|key| !keys.contains(key)) {
        return Err(AppError::invalid_fields(
            "invalid modules",
            vec![FieldViolation::new(
                "modules",
                "settings provided for an unselected module",
            )],
        ));
    }
    let mut prepared = Vec::new();
    let mut violations = Vec::new();
    for entry in app
        .modules()
        .iter()
        .filter(|entry| keys.contains(entry.key()))
    {
        let input = settings.get(entry.key()).cloned().unwrap_or_default();
        let parsed = parse_settings(&entry.manifest.settings, &Map::new(), &input);
        match parsed {
            Err(errors) => violations.extend(errors.into_iter().map(|mut error| {
                error.field = format!("module_settings.{}.{}", entry.key(), error.field);
                error
            })),
            Ok(change) => {
                let secrets: Vec<_> = change
                    .secrets
                    .into_iter()
                    .filter_map(|(key, value)| value.map(|value| (key, Secret::new(value))))
                    .collect();
                let configured: Vec<_> = secrets.iter().map(|(key, _)| key.clone()).collect();
                let values = with_defaults(&entry.manifest.settings, &change.values);
                let errors = missing_required(&entry.manifest.settings, &values, &configured)
                    .into_iter()
                    .chain(
                        entry
                            .module
                            .validate_settings(&Value::Object(values.clone())),
                    );
                violations.extend(errors.map(|mut error| {
                    error.field = format!("module_settings.{}.{}", entry.key(), error.field);
                    error
                }));
                prepared.push(PreparedModule {
                    key: entry.key().to_owned(),
                    values,
                    secrets,
                });
            }
        }
    }
    if !violations.is_empty() {
        return Err(AppError::invalid_fields(
            "invalid module settings",
            violations,
        ));
    }
    Ok(prepared)
}
