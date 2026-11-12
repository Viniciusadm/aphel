use std::collections::HashMap;

use async_trait::async_trait;
use serde::Serialize;
use sqlx::{MySqlPool, Row};
use utoipa::ToSchema;

use crate::manifest::PluginManifest;
use crate::store;
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::api::state::AppState;
use aphel_sdk::platform::application::access::Authorizer;
use aphel_sdk::platform::domain::access::{AccessScope, Permission};
use aphel_sdk::platform::modules::runtime::module_use_cases;
use aphel_sdk::platform::modules::toggle_guard::ModuleToggleGuard;

pub const BLOCKED: &str = "plugin_module_conflict_blocked";

pub struct PluginModuleGuard;

#[async_trait]
impl ModuleToggleGuard for PluginModuleGuard {
    async fn held_modules(&self, pool: &MySqlPool) -> Result<HashMap<String, String>, AppError> {
        let rows = sqlx::query(
            "SELECT c.module_key, p.name FROM plugins_module_conflicts c \
             JOIN plugins_installed p ON p.plugin_key = c.plugin_key \
             ORDER BY c.created_at",
        )
        .fetch_all(pool)
        .await?;
        let mut held: HashMap<String, String> = HashMap::new();
        for row in rows {
            let module: String = row.try_get("module_key")?;
            let name: String = row.try_get("name")?;
            held.entry(module)
                .and_modify(|names| {
                    names.push_str(", ");
                    names.push_str(&name);
                })
                .or_insert(name);
        }
        Ok(held)
    }

    async fn dependents(&self, pool: &MySqlPool) -> Result<HashMap<String, Vec<String>>, AppError> {
        let snapshot = crate::registry::current(pool).await?;
        let app = aphel_sdk::platform::modules::app();
        let enabled_modules: Vec<_> = app
            .modules()
            .iter()
            .filter(|loaded| app.is_enabled(loaded.key()))
            .collect();
        let mut dependents: HashMap<String, Vec<String>> = HashMap::new();
        for module in &enabled_modules {
            let key = module.key();
            let others: Vec<String> = enabled_modules
                .iter()
                .filter(|other| other.key() != key)
                .flat_map(|other| other.manifest.provides.contracts.clone())
                .chain(
                    snapshot
                        .plugins
                        .iter()
                        .flat_map(|plugin| plugin.manifest.provides.contracts.clone()),
                )
                .collect();
            let prefix = format!("{key}.");
            for plugin in &snapshot.plugins {
                let manifest = &plugin.manifest;
                let contract = manifest.uses.required.iter().any(|requirement| {
                    module.manifest.provides.contracts.contains(requirement)
                        && !others.contains(requirement)
                });
                let operation = manifest.operations.iter().any(|operation| {
                    operation.starts_with(&prefix)
                        && !snapshot.plugins.iter().any(|provider| {
                            provider
                                .manifest
                                .provides
                                .operations
                                .iter()
                                .any(|provided| provided.key == *operation)
                        })
                });
                let permission = manifest.requires.iter().any(|required| {
                    Permission::parse(required.clone())
                        .ok()
                        .and_then(|permission| app.catalog().permission_owner(&permission))
                        .is_some_and(|owner| owner == key)
                });
                if contract || operation || permission {
                    dependents
                        .entry(key.to_owned())
                        .or_default()
                        .push(manifest.name.clone());
                }
            }
        }
        Ok(dependents)
    }
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ModuleConflictPlan {
    pub module: String,
    pub name: String,
    pub enabled: bool,
    pub blocked_by: Vec<String>,
}

pub fn held_modules(manifest: &PluginManifest) -> Vec<String> {
    let provided: std::collections::HashSet<&str> = manifest
        .provides
        .contracts
        .iter()
        .map(String::as_str)
        .collect();
    let mut held = std::collections::BTreeSet::new();
    for (module, descriptor) in
        aphel_sdk::platform::modules::app().all_contributions::<aphel_sdk::ContractDescriptor>()
    {
        if module != "kernel" && provided.contains(descriptor.contract.requirement().as_str()) {
            held.insert(module.to_owned());
        }
    }
    held.into_iter().collect()
}

pub async fn plan(
    state: &AppState,
    scope: &AccessScope,
    manifest: &PluginManifest,
) -> Result<Vec<ModuleConflictPlan>, AppError> {
    let replaced = held_modules(manifest);
    if replaced.is_empty() {
        return Ok(Vec::new());
    }
    scope.require(&Permission::SETTINGS_MANAGE)?;
    let views = module_use_cases(state)?.list(scope).await?;
    let plugins = store::list(&state.db()?).await?;
    let mut plans = Vec::new();
    for module in &replaced {
        let Some(view) = views.iter().find(|view| view.descriptor.key() == module) else {
            continue;
        };
        let mut blocked_by = Vec::new();
        if view.descriptor.required {
            blocked_by.push(format!("{} (required)", view.descriptor.manifest.name));
        }
        blocked_by.extend(view.enabled_dependents.iter().cloned());
        blocked_by.extend(
            plugins
                .iter()
                .filter(|other| other.enabled && other.key != manifest.key)
                .filter(|other| depends_on_module(state, &other.manifest, module, manifest))
                .map(|other| other.name.clone()),
        );
        plans.push(ModuleConflictPlan {
            module: module.clone(),
            name: view.descriptor.manifest.name.to_owned(),
            enabled: view.enabled(),
            blocked_by,
        });
    }
    Ok(plans)
}

fn depends_on_module(
    state: &AppState,
    manifest: &PluginManifest,
    module: &str,
    replacement: &PluginManifest,
) -> bool {
    let owns_permission = manifest.requires.iter().any(|required| {
        Permission::parse(required.clone())
            .ok()
            .and_then(|permission| state.app.catalog().permission_owner(&permission))
            .is_some_and(|owner| owner == module)
    });
    let prefix = format!("{module}.");
    owns_permission
        || manifest.operations.iter().any(|operation| {
            operation.starts_with(&prefix)
                && !replacement
                    .provides
                    .operations
                    .iter()
                    .any(|provided| provided.key == *operation)
        })
}

pub fn ensure_unblocked(plans: &[ModuleConflictPlan]) -> Result<(), AppError> {
    let Some(blocked) = plans.iter().find(|plan| !plan.blocked_by.is_empty()) else {
        return Ok(());
    };
    Err(AppError::conflict_code(
        BLOCKED,
        format!(
            "module {} cannot be disabled while in use by {}",
            blocked.module,
            blocked.blocked_by.join(", ")
        ),
    ))
}

async fn held_rows(pool: &MySqlPool, plugin_key: &str) -> Result<Vec<(String, bool)>, AppError> {
    let rows = sqlx::query(
        "SELECT module_key, was_enabled FROM plugins_module_conflicts WHERE plugin_key = ?",
    )
    .bind(plugin_key)
    .fetch_all(pool)
    .await?;
    rows.iter()
        .map(|row| {
            Ok((
                row.try_get::<String, _>("module_key")?,
                row.try_get::<bool, _>("was_enabled")?,
            ))
        })
        .collect()
}

pub async fn hold(
    state: &AppState,
    scope: &AccessScope,
    manifest: &PluginManifest,
) -> Result<Vec<String>, AppError> {
    let pool = state.db()?;
    let existing: Vec<String> = held_rows(&pool, &manifest.key)
        .await?
        .into_iter()
        .map(|(module, _)| module)
        .collect();
    let replaced = held_modules(manifest);
    let mut newly_held = Vec::new();
    for module in replaced.iter().filter(|module| !existing.contains(module)) {
        match hold_module(state, scope, &manifest.key, module).await {
            Ok(()) => newly_held.push(module.clone()),
            Err(error) => {
                release_modules(state, scope, &manifest.key, &newly_held).await;
                return Err(error);
            }
        }
    }
    let stale: Vec<String> = existing
        .into_iter()
        .filter(|module| !replaced.contains(module))
        .collect();
    release_modules(state, scope, &manifest.key, &stale).await;
    Ok(newly_held)
}

async fn hold_module(
    state: &AppState,
    scope: &AccessScope,
    plugin_key: &str,
    module: &str,
) -> Result<(), AppError> {
    let pool = state.db()?;
    let held_by_others: Option<bool> = sqlx::query_scalar(
        "SELECT MAX(was_enabled) FROM plugins_module_conflicts WHERE module_key = ? AND plugin_key <> ?",
    )
    .bind(module)
    .bind(plugin_key)
    .fetch_one(&pool)
    .await?;
    let was_enabled = held_by_others.unwrap_or_else(|| state.app.is_enabled(module));
    if state.app.is_enabled(module) {
        module_use_cases(state)?.disable(scope, module).await?;
    }
    sqlx::query(
        "INSERT INTO plugins_module_conflicts (plugin_key, module_key, was_enabled) VALUES (?, ?, ?) \
         ON DUPLICATE KEY UPDATE was_enabled = VALUES(was_enabled)",
    )
    .bind(plugin_key)
    .bind(module)
    .bind(was_enabled)
    .execute(&pool)
    .await?;
    Ok(())
}

pub async fn release(state: &AppState, scope: &AccessScope, plugin_key: &str) {
    let Ok(pool) = state.db() else {
        return;
    };
    match held_rows(&pool, plugin_key).await {
        Ok(rows) => {
            let modules: Vec<String> = rows.into_iter().map(|(module, _)| module).collect();
            release_modules(state, scope, plugin_key, &modules).await;
        }
        Err(error) => {
            tracing::warn!(plugin = plugin_key, error = %error, "could not read held modules")
        }
    }
}

pub async fn release_modules(
    state: &AppState,
    scope: &AccessScope,
    plugin_key: &str,
    modules: &[String],
) {
    let Ok(pool) = state.db() else {
        return;
    };
    for module in modules {
        if let Err(error) = release_module(state, scope, &pool, plugin_key, module).await {
            tracing::warn!(plugin = plugin_key, module = %module, error = %error, "could not release module");
        }
    }
}

async fn release_module(
    state: &AppState,
    scope: &AccessScope,
    pool: &MySqlPool,
    plugin_key: &str,
    module: &str,
) -> Result<(), AppError> {
    let was_enabled: Option<bool> = sqlx::query_scalar(
        "SELECT was_enabled FROM plugins_module_conflicts WHERE plugin_key = ? AND module_key = ?",
    )
    .bind(plugin_key)
    .bind(module)
    .fetch_optional(pool)
    .await?;
    sqlx::query("DELETE FROM plugins_module_conflicts WHERE plugin_key = ? AND module_key = ?")
        .bind(plugin_key)
        .bind(module)
        .execute(pool)
        .await?;
    let remaining: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM plugins_module_conflicts WHERE module_key = ?")
            .bind(module)
            .fetch_one(pool)
            .await?;
    if was_enabled == Some(true) && remaining == 0 {
        module_use_cases(state)?.enable(scope, module).await?;
    }
    Ok(())
}
