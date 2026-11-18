use std::collections::{BTreeMap, BTreeSet};

use crate::store::grants as store;
use serde::Serialize;
use utoipa::ToSchema;

use crate::manifest::PluginManifest;
use aphel_kernel::errors::{AppError, FieldViolation};
use aphel_sdk::platform::database::Db;
use aphel_sdk::platform::database::transaction::Tx;

const UNKNOWN_PRESET: &str = "plugin_role_preset_unknown";

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, ToSchema)]
pub struct GrantReport {
    pub created_roles: Vec<String>,
    pub granted: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
pub struct RoleGrantStatus {
    pub role: String,
    pub label: Option<String>,
    pub preset: bool,
    pub role_exists: bool,
    pub expected: u64,
    pub granted: u64,
    pub missing: Vec<String>,
}

pub fn selected_presets(
    manifest: &PluginManifest,
    selected: &[String],
) -> Result<Vec<String>, AppError> {
    let mut violations = Vec::new();
    let mut presets = BTreeSet::new();
    for (index, key) in selected.iter().enumerate() {
        if manifest
            .role_presets
            .iter()
            .any(|preset| &preset.key == key)
        {
            presets.insert(key.clone());
        } else {
            violations.push(FieldViolation::new(
                format!("role_presets[{index}]"),
                "is not a role preset of this plugin",
            ));
        }
    }
    if violations.is_empty() {
        Ok(presets.into_iter().collect())
    } else {
        Err(AppError::invalid_fields_code(
            UNKNOWN_PRESET,
            "unknown role presets",
            violations,
        ))
    }
}

async fn role_ids(
    tx: &mut Tx,
    roles: &BTreeSet<String>,
) -> Result<BTreeMap<String, String>, AppError> {
    let mut ids = BTreeMap::new();
    for role in roles {
        if let Some(id) = store::role_id(tx, role).await? {
            ids.insert(role.clone(), id);
        }
    }
    Ok(ids)
}

pub async fn reconcile(
    tx: &mut Tx,
    manifest: &PluginManifest,
    create_presets: &[String],
) -> Result<GrantReport, AppError> {
    let mut report = GrantReport::default();
    for key in create_presets {
        let Some(preset) = manifest
            .role_presets
            .iter()
            .find(|preset| &preset.key == key)
        else {
            continue;
        };
        let created = store::create_role(tx, &preset.key, &preset.label).await?;
        if created {
            report.created_roles.push(preset.key.clone());
        }
    }
    let planned = manifest.planned_grants();
    let roles: BTreeSet<String> = planned.iter().map(|grant| grant.role.clone()).collect();
    let ids = role_ids(tx, &roles).await?;
    for grant in &planned {
        let Some(role_id) = ids.get(&grant.role) else {
            continue;
        };
        report.granted +=
            store::grant(tx, role_id, &grant.permission, grant.scope.as_str()).await?;
    }
    for entity in &manifest.entities {
        let Some(scope) = manifest.entity_scope(entity) else {
            continue;
        };
        report.granted += store::grant_entity_to_system_roles(
            tx,
            &manifest.entity_record_type(&entity.key),
            scope.as_str(),
        )
        .await?;
    }
    Ok(report)
}

pub async fn status(
    pool: &Db,
    manifest: &PluginManifest,
) -> Result<Vec<RoleGrantStatus>, AppError> {
    let mut expected: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for grant in manifest.planned_grants() {
        expected
            .entry(grant.role)
            .or_default()
            .push(grant.permission);
    }
    let mut statuses = Vec::with_capacity(expected.len());
    for (role, permissions) in expected {
        let preset = manifest
            .role_presets
            .iter()
            .find(|preset| preset.key == role);
        let stored = store::role_permissions(pool, &role).await?;
        let role_exists = stored.is_some();
        let granted: BTreeSet<String> = stored.unwrap_or_default().into_iter().collect();
        let missing: Vec<String> = permissions
            .iter()
            .filter(|permission| !granted.contains(*permission))
            .cloned()
            .collect();
        statuses.push(RoleGrantStatus {
            label: preset.map(|preset| preset.label.clone()),
            preset: preset.is_some(),
            role_exists,
            expected: permissions.len() as u64,
            granted: (permissions.len() - missing.len()) as u64,
            missing,
            role,
        });
    }
    Ok(statuses)
}
