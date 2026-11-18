use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

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
        let id: Option<String> = sqlx::query_scalar("SELECT id FROM roles WHERE `key` = ?")
            .bind(role)
            .fetch_optional(&mut **tx)
            .await?;
        if let Some(id) = id {
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
        let created = sqlx::query(
            "INSERT IGNORE INTO roles (id, `key`, name, is_system) VALUES (?, ?, ?, FALSE)",
        )
        .bind(Uuid::now_v7().to_string())
        .bind(&preset.key)
        .bind(&preset.label)
        .execute(&mut **tx)
        .await?
        .rows_affected();
        if created > 0 {
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
        report.granted += sqlx::query(
            "INSERT IGNORE INTO role_permissions (role_id, permission, scope) VALUES (?, ?, ?)",
        )
        .bind(role_id)
        .bind(&grant.permission)
        .bind(grant.scope.as_str())
        .execute(&mut **tx)
        .await?
        .rows_affected();
    }
    for entity in &manifest.entities {
        let Some(scope) = manifest.entity_scope(entity) else {
            continue;
        };
        report.granted += sqlx::query(
            "INSERT IGNORE INTO role_permissions (role_id, permission, scope) \
             SELECT r.id, CONCAT(?, '.', p.action), ? \
             FROM roles r JOIN ( \
               SELECT 'read' AS action UNION ALL SELECT 'create' UNION ALL SELECT 'update' \
               UNION ALL SELECT 'delete' UNION ALL SELECT 'archive' UNION ALL SELECT 'export' \
             ) p \
             WHERE r.is_system = TRUE \
             AND (r.`key` = 'manager' OR (r.`key` = 'member' AND p.action <> 'export'))",
        )
        .bind(manifest.entity_record_type(&entity.key))
        .bind(scope.as_str())
        .execute(&mut **tx)
        .await?
        .rows_affected();
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
        let id: Option<String> = sqlx::query_scalar("SELECT id FROM roles WHERE `key` = ?")
            .bind(&role)
            .fetch_optional(pool)
            .await?;
        let granted: BTreeSet<String> = match &id {
            Some(id) => sqlx::query_scalar::<_, String>(
                "SELECT permission FROM role_permissions WHERE role_id = ?",
            )
            .bind(id)
            .fetch_all(pool)
            .await?
            .into_iter()
            .collect(),
            None => BTreeSet::new(),
        };
        let missing: Vec<String> = permissions
            .iter()
            .filter(|permission| !granted.contains(*permission))
            .cloned()
            .collect();
        statuses.push(RoleGrantStatus {
            label: preset.map(|preset| preset.label.clone()),
            preset: preset.is_some(),
            role_exists: id.is_some(),
            expected: permissions.len() as u64,
            granted: (permissions.len() - missing.len()) as u64,
            missing,
            role,
        });
    }
    Ok(statuses)
}
