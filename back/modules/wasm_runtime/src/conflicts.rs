use serde::Serialize;
use serde_json::json;
use sqlx::MySqlPool;
use utoipa::ToSchema;

use crate::manifest::PluginManifest;
use crate::store::{self, InstalledPlugin};
use aphel_kernel::errors::{AppError, CodedKind};

pub const UNACKNOWLEDGED: &str = "plugin_conflicts_unacknowledged";

const CONTESTED_HOOKS: &[&str] = &[
    "record.saving",
    "record.validating",
    "record.deleting",
    "record.archiving",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
pub struct PluginConflict {
    pub kind: String,
    pub target: String,
    pub other_plugin: String,
    pub other_name: String,
}

impl PluginConflict {
    fn new(kind: &str, target: impl Into<String>, other: &InstalledPlugin) -> Self {
        Self {
            kind: kind.to_owned(),
            target: target.into(),
            other_plugin: other.key.clone(),
            other_name: other.name.clone(),
        }
    }
}

pub fn detect(candidate: &PluginManifest, others: &[InstalledPlugin]) -> Vec<PluginConflict> {
    let mut conflicts = Vec::new();
    for other in others
        .iter()
        .filter(|other| other.enabled && other.key != candidate.key)
    {
        let manifest = &other.manifest;
        for rule in &candidate.menu.rules {
            if manifest
                .menu
                .rules
                .iter()
                .any(|existing| existing.target == rule.target)
            {
                conflicts.push(PluginConflict::new("menu_target", &rule.target, other));
            }
        }
        for hook in candidate
            .hooks
            .subscribe
            .iter()
            .filter(|hook| CONTESTED_HOOKS.contains(&hook.hook.as_str()))
        {
            for record_type in &hook.record_types {
                let own = |manifest: &PluginManifest| {
                    manifest.entity_record_types().contains(record_type)
                };
                if own(candidate) || own(manifest) {
                    continue;
                }
                let overlaps = manifest.hooks.subscribe.iter().any(|existing| {
                    existing.hook == hook.hook && existing.record_types.contains(record_type)
                });
                if overlaps {
                    conflicts.push(PluginConflict::new(
                        "hook_overlap",
                        format!("{} ({record_type})", hook.hook),
                        other,
                    ));
                }
            }
        }
        for preset in &candidate.role_presets {
            if manifest
                .role_presets
                .iter()
                .any(|existing| existing.key == preset.key)
            {
                conflicts.push(PluginConflict::new("role_preset_key", &preset.key, other));
            }
        }
    }
    conflicts.sort_by(|left, right| {
        (&left.other_plugin, &left.kind, &left.target).cmp(&(
            &right.other_plugin,
            &right.kind,
            &right.target,
        ))
    });
    conflicts.dedup();
    conflicts
}

pub async fn detect_for(
    pool: &MySqlPool,
    candidate: &PluginManifest,
) -> Result<Vec<PluginConflict>, AppError> {
    let others = store::list(pool).await?;
    Ok(detect(candidate, &others))
}

pub fn require_acknowledged(
    conflicts: &[PluginConflict],
    acknowledged: bool,
) -> Result<(), AppError> {
    if conflicts.is_empty() || acknowledged {
        return Ok(());
    }
    Err(AppError::with_details(
        CodedKind::Conflict,
        UNACKNOWLEDGED,
        "this plugin overlaps with another installed plugin",
        json!({ "conflicts": conflicts }),
    ))
}
