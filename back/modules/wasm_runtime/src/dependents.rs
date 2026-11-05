use crate::manifest::PluginManifest;
use crate::registry;
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::api::state::AppState;

const PLUGIN_HAS_DEPENDENTS: &str = "plugin_has_dependents";

pub async fn ensure_none_broken(
    state: &AppState,
    key: &str,
    replacement: Option<&PluginManifest>,
) -> Result<(), AppError> {
    let snapshot = registry::current(&state.db()?).await?;
    let Some(current) = snapshot.plugin(key) else {
        return Ok(());
    };
    let others = || snapshot.plugins.iter().filter(|other| other.key != key);
    let mut base: Vec<String> = state
        .app
        .modules()
        .iter()
        .filter(|loaded| state.app.is_enabled(loaded.key()))
        .flat_map(|loaded| loaded.manifest.provides.contracts.clone())
        .collect();
    base.extend(others().flat_map(|other| other.manifest.provides.contracts.clone()));
    let mut before = base.clone();
    before.extend(current.manifest.provides.contracts.clone());
    let mut after = base;
    let lost_operations: Vec<&str> = current
        .manifest
        .provides
        .operations
        .iter()
        .map(|operation| operation.key.as_str())
        .filter(|operation| {
            !replacement.is_some_and(|manifest| {
                manifest
                    .provides
                    .operations
                    .iter()
                    .any(|provided| provided.key == *operation)
            })
        })
        .collect();
    if let Some(manifest) = replacement {
        after.extend(manifest.provides.contracts.clone());
    }
    let dependents: Vec<&str> = others()
        .filter(|other| {
            (other.manifest.uses.unmet(&before).is_empty()
                && !other.manifest.uses.unmet(&after).is_empty())
                || other
                    .manifest
                    .operations
                    .iter()
                    .any(|operation| lost_operations.contains(&operation.as_str()))
        })
        .map(|other| other.key.as_str())
        .collect();
    if dependents.is_empty() {
        return Ok(());
    }
    Err(AppError::conflict_code(
        PLUGIN_HAS_DEPENDENTS,
        format!(
            "the change would break the enabled plugins: {}",
            dependents.join(", ")
        ),
    ))
}
