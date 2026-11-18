use std::collections::HashSet;
use std::sync::{Arc, RwLock};
use std::time::Duration;

use tokio::task::JoinHandle;

use crate::database::Db;
use crate::errors::AppError;
use crate::tenancy::TenantLocal;

pub const REFRESH_INTERVAL: Duration = Duration::from_secs(5);

pub struct EnabledModules {
    required: HashSet<&'static str>,
    compiled: HashSet<&'static str>,
    enabled: TenantLocal<RwLock<HashSet<String>>>,
}

impl EnabledModules {
    pub fn new(
        required: impl IntoIterator<Item = &'static str>,
        compiled: impl IntoIterator<Item = &'static str>,
    ) -> Self {
        Self {
            required: required.into_iter().collect(),
            compiled: compiled.into_iter().collect(),
            enabled: TenantLocal::new(RwLock::default),
        }
    }

    pub fn is_required(&self, key: &str) -> bool {
        self.required.contains(key)
    }

    pub fn is_enabled(&self, key: &str) -> bool {
        self.required.contains(key)
            || self.enabled.get().is_some_and(|enabled| {
                enabled
                    .read()
                    .map(|enabled| enabled.contains(key))
                    .unwrap_or(false)
            })
    }

    pub fn snapshot(&self) -> Vec<String> {
        let mut keys: Vec<String> = self
            .enabled
            .get()
            .and_then(|enabled| {
                enabled
                    .read()
                    .map(|enabled| enabled.iter().cloned().collect())
                    .ok()
            })
            .unwrap_or_default();
        keys.extend(
            self.required
                .iter()
                .filter(|key| !keys.iter().any(|existing| existing == *key))
                .map(|key| (*key).to_owned())
                .collect::<Vec<_>>(),
        );
        keys.sort();
        keys
    }

    pub fn replace(&self, keys: impl IntoIterator<Item = String>) {
        let keys: HashSet<String> = keys
            .into_iter()
            .filter(|key| self.compiled.contains(key.as_str()))
            .collect();
        let Some(enabled) = self.enabled.get() else {
            return;
        };
        if let Ok(mut enabled) = enabled.write()
            && *enabled != keys
        {
            *enabled = keys;
            crate::application::access::invalidate_access_catalog();
        }
    }

    pub async fn refresh(&self, pool: &Db) -> Result<(), AppError> {
        let keys = stored_enabled(pool).await?;
        self.replace(keys);
        Ok(())
    }

    pub fn spawn_refresh(self: Arc<Self>, pool: Db) -> JoinHandle<()> {
        crate::tenancy::spawn(async move {
            let mut interval = tokio::time::interval(REFRESH_INTERVAL);
            loop {
                interval.tick().await;
                if let Err(error) = self.refresh(&pool).await {
                    tracing::warn!("could not refresh the enabled modules: {error}");
                }
            }
        })
    }
}

pub async fn stored_enabled(pool: &Db) -> Result<Vec<String>, AppError> {
    Ok(sqlx::query_scalar(
        "SELECT module_key FROM installed_modules WHERE enabled ORDER BY module_key",
    )
    .fetch_all(pool)
    .await?)
}
