use std::collections::{BTreeMap, BTreeSet};
use std::io::{Cursor, Read};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use utoipa::ToSchema;

use crate::package::{PluginPackage, read_package};
use aphel_kernel::errors::{AppError, CodedKind, FieldViolation};

pub const PACK_FILE: &str = "pack.toml";
pub const MAX_PACK_BYTES: usize = 200 * 1024 * 1024;
const MAX_PACK_PLUGINS: usize = 32;
const MAX_PACK_ENTRIES: usize = 64;
pub const PACK_INVALID: &str = "plugin_pack_invalid";
pub const PACK_SETTING_INVALID: &str = "plugin_pack_setting_invalid";
pub const PACK_DEPENDENCY_CYCLE: &str = "plugin_pack_dependency_cycle";
pub const PACK_MISSING_DEPENDENCY: &str = "plugin_pack_missing_dependency";
pub const PACK_PLUGINS_INVALID: &str = "plugin_pack_plugins_invalid";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackManifest {
    pub key: String,
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub role_presets: Vec<String>,
    pub plugins: Vec<PackPluginEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackPluginEntry {
    pub file: String,
    #[serde(default)]
    pub settings: Map<String, Value>,
}

#[derive(Debug, Clone)]
pub struct PackMember {
    pub package: PluginPackage,
    pub settings: Map<String, Value>,
}

#[derive(Debug, Clone)]
pub struct PluginPack {
    pub manifest: PackManifest,
    pub members: Vec<PackMember>,
}

impl PluginPack {
    pub fn keys(&self) -> BTreeSet<String> {
        self.members
            .iter()
            .map(|member| member.package.manifest.key.clone())
            .collect()
    }

    pub fn provided_operations(&self) -> Vec<String> {
        self.members
            .iter()
            .flat_map(|member| {
                member
                    .package
                    .manifest
                    .provides
                    .operations
                    .iter()
                    .map(|operation| operation.key.clone())
            })
            .collect()
    }
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PackPluginError {
    pub code: String,
    pub message: String,
    pub fields: Vec<String>,
}

impl PackPluginError {
    pub fn from_error(error: &AppError) -> Self {
        let (code, fields) = match error {
            AppError::CodedFields { code, fields, .. } => ((*code).to_owned(), fields.as_slice()),
            AppError::InvalidFields { fields, .. } => {
                ("validation_error".to_owned(), fields.as_slice())
            }
            AppError::Coded { code, .. } | AppError::CodedDetails { code, .. } => {
                ((*code).to_owned(), &[][..])
            }
            AppError::Conflict(_) => ("conflict".to_owned(), &[][..]),
            AppError::NotFound(_) => ("not_found".to_owned(), &[][..]),
            AppError::Validation(_) => ("validation_error".to_owned(), &[][..]),
            _ => ("internal_error".to_owned(), &[][..]),
        };
        Self {
            code,
            message: error.client_message().to_owned(),
            fields: fields
                .iter()
                .map(|violation| format!("{}: {}", violation.field, violation.message))
                .collect(),
        }
    }
}

fn invalid(message: impl Into<String>) -> AppError {
    AppError::validation_code(PACK_INVALID, message)
}

fn is_pack_key(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 48
        && value.starts_with(|first: char| first.is_ascii_lowercase())
        && value.chars().all(|character| {
            character.is_ascii_lowercase()
                || character.is_ascii_digit()
                || character == '_'
                || character == '-'
        })
}

fn entries(bytes: &[u8]) -> Result<BTreeMap<String, Vec<u8>>, AppError> {
    if bytes.len() > MAX_PACK_BYTES {
        return Err(invalid("the pack is larger than 200 MB"));
    }
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes))
        .map_err(|_| invalid("the pack is not a valid zip file"))?;
    if archive.len() > MAX_PACK_ENTRIES {
        return Err(invalid("the pack has too many files"));
    }
    let mut files = BTreeMap::new();
    let mut total = 0usize;
    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .map_err(|_| invalid("the pack has an unreadable entry"))?;
        if entry.is_dir() {
            continue;
        }
        let path = entry
            .name()
            .map_err(|_| invalid("the pack has an entry with an invalid name"))?
            .trim_start_matches("./")
            .to_owned();
        let mut content = Vec::new();
        entry
            .by_ref()
            .take((MAX_PACK_BYTES - total + 1) as u64)
            .read_to_end(&mut content)
            .map_err(|_| invalid(format!("could not read {path}")))?;
        total += content.len();
        if total > MAX_PACK_BYTES {
            return Err(invalid("the unpacked pack is larger than 200 MB"));
        }
        files.insert(path, content);
    }
    Ok(files)
}

pub fn read_pack(bytes: &[u8]) -> Result<PluginPack, AppError> {
    let mut files = entries(bytes)?;
    let text = files
        .get(PACK_FILE)
        .ok_or_else(|| invalid("the pack has no pack.toml"))?;
    let text = std::str::from_utf8(text).map_err(|_| invalid("pack.toml is not UTF-8"))?;
    let manifest: PackManifest = toml::from_str(text)
        .map_err(|error| invalid(format!("pack.toml is invalid: {}", error.message())))?;
    let mut violations = Vec::new();
    if !is_pack_key(&manifest.key) {
        violations.push(FieldViolation::new(
            "key",
            "must be a lowercase key of at most 48 characters",
        ));
    }
    if manifest.name.trim().is_empty() || manifest.name.len() > 120 {
        violations.push(FieldViolation::new(
            "name",
            "must have between 1 and 120 characters",
        ));
    }
    if semver::Version::parse(&manifest.version).is_err() {
        violations.push(FieldViolation::new(
            "version",
            "must be a semantic version such as 1.0.0",
        ));
    }
    if manifest.plugins.is_empty() || manifest.plugins.len() > MAX_PACK_PLUGINS {
        violations.push(FieldViolation::new(
            "plugins",
            format!("must list between 1 and {MAX_PACK_PLUGINS} plugins"),
        ));
    }
    if !violations.is_empty() {
        return Err(AppError::invalid_fields_code(
            PACK_INVALID,
            "invalid pack.toml",
            violations,
        ));
    }
    let mut members = Vec::new();
    let mut seen_files = BTreeSet::new();
    let mut seen_keys = BTreeSet::new();
    for (index, entry) in manifest.plugins.iter().enumerate() {
        if !seen_files.insert(entry.file.clone()) {
            violations.push(FieldViolation::new(
                format!("plugins[{index}].file"),
                "is listed twice",
            ));
            continue;
        }
        let Some(content) = files.remove(&entry.file) else {
            violations.push(FieldViolation::new(
                format!("plugins[{index}].file"),
                "is not in the pack",
            ));
            continue;
        };
        let package = match read_package(&content) {
            Ok(package) => package,
            Err(error) => {
                violations.push(FieldViolation::new(
                    format!("plugins[{index}].file"),
                    format!("is not a valid plugin package: {}", error.client_message()),
                ));
                continue;
            }
        };
        if !seen_keys.insert(package.manifest.key.clone()) {
            violations.push(FieldViolation::new(
                format!("plugins[{index}].file"),
                format!("repeats the plugin {}", package.manifest.key),
            ));
            continue;
        }
        members.push(PackMember {
            package,
            settings: entry.settings.clone(),
        });
    }
    for (index, preset) in manifest.role_presets.iter().enumerate() {
        let declared = members.iter().any(|member| {
            member
                .package
                .manifest
                .role_presets
                .iter()
                .any(|candidate| &candidate.key == preset)
        });
        if !declared {
            violations.push(FieldViolation::new(
                format!("role_presets[{index}]"),
                "is not a role preset of any plugin in the pack",
            ));
        }
    }
    if !violations.is_empty() {
        return Err(AppError::invalid_fields_code(
            PACK_INVALID,
            "invalid pack",
            violations,
        ));
    }
    let mut setting_violations = Vec::new();
    for (index, member) in members.iter().enumerate() {
        for (key, value) in &member.settings {
            if let Some(message) = setting_problem(&member.package.manifest, key, value) {
                setting_violations.push(FieldViolation::new(
                    format!("plugins[{index}].settings.{key}"),
                    message,
                ));
            }
        }
    }
    if !setting_violations.is_empty() {
        return Err(AppError::invalid_fields_code(
            PACK_SETTING_INVALID,
            "invalid plugin settings in the pack",
            setting_violations,
        ));
    }
    let members = install_order(members)?;
    Ok(PluginPack { manifest, members })
}

fn setting_problem(
    manifest: &crate::manifest::PluginManifest,
    key: &str,
    value: &Value,
) -> Option<String> {
    let Some(setting) = manifest.settings.iter().find(|setting| setting.key == key) else {
        return Some(format!("is not a setting of {}", manifest.key));
    };
    if crate::manifest::is_sensitive_setting(&setting.kind) {
        return Some("is a secret and cannot travel in a pack".to_owned());
    }
    let valid = match setting.kind.as_str() {
        "number" => value.as_i64().is_some_and(|number| {
            setting.min.is_none_or(|min| number >= min)
                && setting.max.is_none_or(|max| number <= max)
        }),
        "boolean" => value.is_boolean(),
        crate::manifest::SETTING_SELECT => value
            .as_str()
            .is_some_and(|text| setting.options.iter().any(|option| option.value == text)),
        _ => value.as_str().is_some_and(|text| text.len() <= 1000),
    };
    (!valid).then(|| format!("must be a valid {}", setting.kind))
}

fn dependencies(members: &[PackMember]) -> Vec<BTreeSet<usize>> {
    let mut providers: BTreeMap<&str, usize> = BTreeMap::new();
    for (index, member) in members.iter().enumerate() {
        for operation in &member.package.manifest.provides.operations {
            providers.insert(operation.key.as_str(), index);
        }
    }
    let by_key: BTreeMap<&str, usize> = members
        .iter()
        .enumerate()
        .map(|(index, member)| (member.package.manifest.key.as_str(), index))
        .collect();
    members
        .iter()
        .enumerate()
        .map(|(index, member)| {
            let manifest = &member.package.manifest;
            let mut needs = BTreeSet::new();
            for operation in &manifest.operations {
                if let Some(&provider) = providers.get(operation.as_str())
                    && provider != index
                {
                    needs.insert(provider);
                }
            }
            for event in &manifest.events {
                if let Some(rest) = event.strip_prefix(crate::manifest::PLUGIN_EVENT_PREFIX)
                    && let Some((owner, _)) = rest.split_once('.')
                    && let Some(&producer) = by_key.get(owner)
                    && producer != index
                {
                    needs.insert(producer);
                }
            }
            for contract in &manifest.uses.required {
                for (other, candidate) in members.iter().enumerate() {
                    if other != index
                        && candidate
                            .package
                            .manifest
                            .provides
                            .contracts
                            .contains(contract)
                    {
                        needs.insert(other);
                    }
                }
            }
            for target in record_targets(manifest) {
                if let Some((owner, _)) = target.split_once('.')
                    && let Some(&owner) = by_key.get(owner)
                    && owner != index
                {
                    needs.insert(owner);
                }
            }
            for setting in &manifest.settings {
                if let Some(source) = &setting.options_from
                    && let Some((owner, _)) = source.entity.split_once('.')
                    && let Some(&owner) = by_key.get(owner)
                    && owner != index
                {
                    needs.insert(owner);
                }
            }
            needs
        })
        .collect()
}

fn install_order(members: Vec<PackMember>) -> Result<Vec<PackMember>, AppError> {
    let needs = dependencies(&members);
    let mut remaining: Vec<BTreeSet<usize>> = needs.clone();
    let mut placed = vec![false; members.len()];
    let mut order = Vec::with_capacity(members.len());
    while order.len() < members.len() {
        let next = (0..members.len())
            .filter(|&index| !placed[index] && remaining[index].is_empty())
            .min_by(|&left, &right| {
                members[left]
                    .package
                    .manifest
                    .key
                    .cmp(&members[right].package.manifest.key)
            });
        let Some(next) = next else {
            let cycle: Vec<String> = (0..members.len())
                .filter(|&index| !placed[index])
                .map(|index| members[index].package.manifest.key.clone())
                .collect();
            return Err(AppError::with_details(
                CodedKind::Validation,
                PACK_DEPENDENCY_CYCLE,
                format!(
                    "these plugins depend on each other in a cycle: {}",
                    cycle.join(", ")
                ),
                json!({ "plugins": cycle }),
            ));
        };
        placed[next] = true;
        order.push(next);
        for needs in &mut remaining {
            needs.remove(&next);
        }
    }
    let mut slots: Vec<Option<PackMember>> = members.into_iter().map(Some).collect();
    Ok(order
        .into_iter()
        .map(|index| slots[index].take().expect("placed once"))
        .collect())
}

pub fn ensure_dependencies(pack: &PluginPack, host_operations: &[&str]) -> Result<(), AppError> {
    let provided = pack.provided_operations();
    let mut missing = Vec::new();
    for member in &pack.members {
        for operation in &member.package.manifest.operations {
            if !host_operations.contains(&operation.as_str()) && !provided.contains(operation) {
                missing
                    .push(json!({ "plugin": member.package.manifest.key, "operation": operation }));
            }
        }
    }
    if missing.is_empty() {
        return Ok(());
    }
    let list: Vec<String> = missing
        .iter()
        .map(|item| {
            format!(
                "{} needs {}",
                item["plugin"].as_str().unwrap_or_default(),
                item["operation"].as_str().unwrap_or_default()
            )
        })
        .collect();
    Err(AppError::with_details(
        CodedKind::Validation,
        PACK_MISSING_DEPENDENCY,
        format!(
            "the pack needs operations that neither this Aphel nor the pack offer: {}",
            list.join("; ")
        ),
        json!({ "missing": missing }),
    ))
}

fn record_targets(manifest: &crate::manifest::PluginManifest) -> Vec<String> {
    let mut targets: Vec<String> = crate::schema::declared_fields(manifest)
        .into_iter()
        .filter(|(_, field)| field.field_type == "record")
        .filter_map(|(_, field)| field.target.clone())
        .collect();
    for entity in &manifest.entities {
        for reference in &entity.references {
            if let aphel_sdk::platform::application::fields::ReferenceTarget::Entity {
                entity: target,
            } = &reference.target
            {
                targets.push(target.clone());
            }
        }
    }
    targets
}
