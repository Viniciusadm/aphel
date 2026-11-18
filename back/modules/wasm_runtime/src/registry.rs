use std::collections::HashMap;
use std::sync::{Arc, OnceLock, RwLock};
use std::time::{Duration, Instant};

use serde_json::{Map, Value};
use tokio::sync::Mutex;

use crate::manifest::PluginManifest;
use crate::widgets::PluginWidget;
use crate::{notify, runtime, store};
use aphel_kernel::errors::AppError;
use aphel_sdk::notifications as intern;
use aphel_sdk::platform::database::Db;
use aphel_sdk::platform::tenancy::TenantLocal;

const CHECK_INTERVAL: Duration = Duration::from_secs(2);
const REFRESH_INTERVAL: Duration = Duration::from_secs(1);

pub struct LoadedPlugin {
    pub key: String,
    pub manifest: PluginManifest,
    pub settings: Map<String, Value>,
    pub installed_by: Option<String>,
    pub backend: Option<Arc<BackendSlot>>,
    pub backend_error: Option<String>,
}

pub struct BackendSlot {
    key: String,
    checksum: String,
    loaded: tokio::sync::OnceCell<Result<runtime::Backend, String>>,
}

impl BackendSlot {
    pub async fn get(&self) -> Result<runtime::Backend, AppError> {
        let loaded = self
            .loaded
            .get_or_try_init(|| async {
                let pool = aphel_sdk::platform::state::runtime_pool()?;
                let installed = store::find(&pool, &self.key)
                    .await?
                    .filter(|plugin| plugin.enabled && plugin.checksum == self.checksum)
                    .ok_or_else(|| {
                        AppError::unavailable_code(
                            "plugin_catalog_changed",
                            "the plugin changed while its backend was loading",
                        )
                    })?;
                let bytes = store::backend(&pool, &installed.key)
                    .await?
                    .ok_or_else(|| {
                        AppError::unavailable_code(
                            "plugin_failed",
                            "the declared backend is missing",
                        )
                    })?;
                Ok::<_, AppError>(
                    runtime::compile(bytes)
                        .await
                        .map_err(|error| error.client_message().to_owned()),
                )
            })
            .await?;
        loaded
            .clone()
            .map_err(|message| AppError::unavailable_code("plugin_failed", message))
    }

    fn error(&self) -> Option<String> {
        self.loaded
            .get()
            .and_then(|result| result.as_ref().err().cloned())
    }
}

#[derive(Default)]
pub struct Snapshot {
    pub version: Option<u64>,
    pub plugins: Vec<Arc<LoadedPlugin>>,
}

impl Snapshot {
    pub fn plugin(&self, key: &str) -> Option<&Arc<LoadedPlugin>> {
        self.plugins.iter().find(|plugin| plugin.key == key)
    }

    pub fn backend_error(&self, key: &str) -> Option<String> {
        self.plugin(key).and_then(|plugin| {
            plugin
                .backend_error
                .clone()
                .or_else(|| plugin.backend.as_ref().and_then(|backend| backend.error()))
        })
    }

    pub fn hook_subscribers(&self, hook: &str, record_type: &str) -> Vec<Arc<LoadedPlugin>> {
        let mut subscribers: Vec<(i32, &Arc<LoadedPlugin>)> = self
            .plugins
            .iter()
            .filter(|plugin| plugin.backend.is_some() || plugin.backend_error.is_some())
            .filter_map(|plugin| {
                plugin
                    .manifest
                    .subscribes(hook, record_type)
                    .map(|priority| (priority, plugin))
            })
            .collect();
        subscribers.sort_by(|left, right| left.0.cmp(&right.0).then(left.1.key.cmp(&right.1.key)));
        subscribers
            .into_iter()
            .map(|(_, plugin)| plugin.clone())
            .collect()
    }

    pub fn event_consumers(&self, event_type: &str) -> Vec<Arc<LoadedPlugin>> {
        self.plugins
            .iter()
            .filter(|plugin| plugin.manifest.consumes(event_type))
            .cloned()
            .collect()
    }
}

type LoadedBackend = (Option<Arc<BackendSlot>>, Option<String>);

#[derive(Default)]
struct Cache {
    checked_at: Option<Instant>,
    compiled: HashMap<String, (String, LoadedBackend)>,
}

#[derive(Default)]
struct RegistryState {
    cache: Mutex<Cache>,
    verified_at: RwLock<Option<Instant>>,
    published: RwLock<Arc<Snapshot>>,
    refreshing: OnceLock<()>,
}

static REGISTRY: TenantLocal<RegistryState> = TenantLocal::new(RegistryState::default);

fn mark_verified(at: Option<Instant>) {
    if let Some(state) = REGISTRY.get()
        && let Ok(mut verified) = state.verified_at.write()
    {
        *verified = at;
    }
}

pub fn has_event_consumers(event_type: &str) -> Option<bool> {
    let state = REGISTRY.get()?;
    let fresh = state
        .verified_at
        .read()
        .ok()
        .and_then(|verified| *verified)
        .is_some_and(|verified| verified.elapsed() < CHECK_INTERVAL);
    let snapshot = cached();
    (fresh && snapshot.version.is_some()).then(|| !snapshot.event_consumers(event_type).is_empty())
}

#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct CatalogReadiness {
    pub state: String,
    pub version: Option<u64>,
    pub backend_errors: Vec<String>,
}

pub fn readiness() -> CatalogReadiness {
    let snapshot = cached();
    let backend_errors: Vec<_> = snapshot
        .plugins
        .iter()
        .filter(|plugin| snapshot.backend_error(&plugin.key).is_some())
        .map(|plugin| plugin.key.clone())
        .collect();
    CatalogReadiness {
        state: if snapshot.version.is_none() {
            "loading"
        } else if backend_errors.is_empty() {
            "ready"
        } else {
            "degraded"
        }
        .to_owned(),
        version: snapshot.version,
        backend_errors,
    }
}

pub fn cached() -> Arc<Snapshot> {
    REGISTRY
        .get()
        .and_then(|state| state.published.read().ok().map(|snapshot| snapshot.clone()))
        .unwrap_or_default()
}

pub async fn invalidate() {
    aphel_sdk::platform::application::access::invalidate_access_catalog();
    mark_verified(None);
    if let Some(state) = REGISTRY.get() {
        state.cache.lock().await.checked_at = None;
    }
}

pub async fn current(pool: &Db) -> Result<Arc<Snapshot>, AppError> {
    load(pool).await.map_err(|error| {
        tracing::warn!(error = %error, "plugin catalog is unavailable");
        AppError::unavailable_code(
            "plugin_catalog_unavailable",
            "the plugin catalog is not ready; retry the request",
        )
    })
}

async fn load(pool: &Db) -> Result<Arc<Snapshot>, AppError> {
    let state = REGISTRY.try_get()?;
    let available = cached();
    let mut cache = match state.cache.try_lock() {
        Ok(cache) => cache,
        Err(_) if available.version.is_some() => return Ok(available),
        Err(_) => state.cache.lock().await,
    };
    let snapshot = cached();
    if snapshot.version.is_some()
        && cache
            .checked_at
            .is_some_and(|checked| checked.elapsed() < CHECK_INTERVAL)
    {
        return Ok(snapshot);
    }
    let version = store::version(pool).await?;
    if snapshot.version == Some(version) {
        let now = Instant::now();
        cache.checked_at = Some(now);
        mark_verified(Some(now));
        return Ok(snapshot);
    }
    let installed_plugins = store::list(pool).await?;
    cache.compiled.retain(|key, _| {
        installed_plugins
            .iter()
            .any(|plugin| plugin.enabled && &plugin.key == key)
    });
    let mut plugins = Vec::new();
    for installed in installed_plugins {
        if !installed.enabled {
            continue;
        }
        let (backend, backend_error) = match cache.compiled.get(&installed.key) {
            Some((checksum, loaded)) if checksum == &installed.checksum => loaded.clone(),
            _ => {
                let present: i32 = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM plugins_files WHERE plugin_key = ? AND path = 'backend.wasm')")
                    .bind(&installed.key).fetch_one(pool).await?;
                let backend = (
                    if present != 0 {
                        Some(Arc::new(BackendSlot {
                            key: installed.key.clone(),
                            checksum: installed.checksum.clone(),
                            loaded: tokio::sync::OnceCell::new(),
                        }))
                    } else {
                        None
                    },
                    if present == 0 && installed.manifest.needs_backend() {
                        Some("the declared plugin backend is missing".to_owned())
                    } else {
                        None
                    },
                );
                cache.compiled.insert(
                    installed.key.clone(),
                    (installed.checksum.clone(), backend.clone()),
                );
                backend
            }
        };
        plugins.push(Arc::new(LoadedPlugin {
            settings: installed.effective_settings(),
            installed_by: installed.installed_by,
            key: installed.key,
            manifest: installed.manifest,
            backend,
            backend_error,
        }));
    }
    if store::version(pool).await? != version {
        cache.checked_at = None;
        return Err(AppError::unavailable_code(
            "plugin_catalog_changed",
            "the plugin catalog changed while loading; retry the request",
        ));
    }
    let verified = Instant::now();
    cache.checked_at = Some(verified);
    let snapshot = Arc::new(Snapshot {
        version: Some(version),
        plugins,
    });
    publish_extensions(&snapshot);
    if let Ok(mut published) = state.published.write() {
        *published = snapshot.clone();
    }
    mark_verified(Some(verified));
    Ok(snapshot)
}

fn publish_extensions(snapshot: &Snapshot) {
    let mut events = Vec::new();
    let mut kinds = Vec::new();
    let mut widgets = Vec::new();
    for plugin in &snapshot.plugins {
        for spec in &plugin.manifest.emits {
            use aphel_sdk::platform::application::public_events::{PublicResource, event};
            events.push(
                event(
                    intern::intern(&plugin.manifest.event_type(&spec.name)),
                    PublicResource::payload(intern::intern(&format!("plugin.{}", plugin.key))),
                    intern::intern(spec.description.as_deref().unwrap_or(&spec.label)),
                )
                .with_verbatim_payload(),
            );
        }
        for spec in &plugin.manifest.notifications {
            let has = |channel: &str| spec.channels.iter().any(|candidate| candidate == channel);
            kinds.push(intern::runtime_notification_kind(
                &notify::kind_key(&plugin.key, &spec.name),
                intern::channel_set(has("in_app"), has("email"), has("push")),
                spec.email_default,
                spec.push_default,
            ));
        }
        if plugin.backend.is_some() {
            widgets.extend(
                plugin
                    .manifest
                    .widgets
                    .iter()
                    .map(|spec| PluginWidget::shared(plugin, spec)),
            );
        }
    }
    let app = aphel_sdk::platform::modules::app();
    let mut batch = aphel_kernel::modules::RuntimeBatch::new();
    let mut provided = std::collections::HashSet::new();
    for plugin in snapshot
        .plugins
        .iter()
        .filter(|plugin| plugin.backend.is_some())
    {
        let entries = match crate::bridge::provided_operations(plugin) {
            Ok(entries) => entries,
            Err(error) => {
                tracing::warn!(plugin = %plugin.key, error = %error, "could not build provided operations");
                continue;
            }
        };
        let replaced = crate::modules::held_modules(&plugin.manifest);
        for entry in entries {
            let key = entry.key;
            if !provided.insert(key) {
                tracing::warn!(plugin = %plugin.key, operation = key, "operation already provided by another plugin; skipped");
                continue;
            }
            if let Err(error) = batch.add_operation(app, entry, &replaced) {
                tracing::warn!(plugin = %plugin.key, operation = key, error = %error, "provided operation skipped");
            }
        }
    }
    for plugin in &snapshot.plugins {
        for contribution in crate::bridge::provided_contributions(plugin) {
            batch.add(std::sync::Arc::new(contribution));
        }
    }
    for plugin in &snapshot.plugins {
        for declaration in &plugin.manifest.hooks.declare {
            let kind = match declaration.kind.as_str() {
                "veto" => aphel_kernel::modules::PluginHookKind::Veto,
                "filter" => aphel_kernel::modules::PluginHookKind::Filter,
                _ => aphel_kernel::modules::PluginHookKind::Action,
            };
            batch.add(std::sync::Arc::new(aphel_kernel::modules::PluginHook::new(
                intern::intern(&declaration.key),
                intern::intern(&declaration.record_type),
                kind,
                intern::intern(&declaration.description),
            )));
        }
    }
    for kind in kinds {
        batch.add(std::sync::Arc::new(kind));
    }
    for widget in widgets {
        batch.add::<dyn aphel_sdk::dashboard::WidgetProvider<aphel_sdk::dashboard::WidgetPorts>>(
            widget,
        );
    }
    for event in events {
        batch.add(std::sync::Arc::new(event));
    }
    app.replace_runtime(crate::KEY, batch);
}

pub fn spawn_refresh(pool: Db) {
    if tokio::runtime::Handle::try_current().is_err() {
        return;
    }
    let Some(state) = REGISTRY.get() else {
        return;
    };
    if state.refreshing.set(()).is_err() {
        return;
    }
    aphel_sdk::platform::tenancy::spawn(async move {
        loop {
            if let Err(error) = current(&pool).await {
                tracing::warn!(error = %error, "could not refresh the installed plugins");
            }
            tokio::time::sleep(REFRESH_INTERVAL).await;
        }
    });
}

pub struct PublicEvents;

#[async_trait::async_trait]
impl aphel_sdk::platform::application::public_events::RuntimePublicEventSource for PublicEvents {
    fn recognizes(&self, event_type: &str) -> bool {
        event_type.starts_with("plugin.")
    }

    async fn refresh(&self, pool: &Db) -> Result<(), AppError> {
        current(pool).await?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl aphel_sdk::platform::modules::runtime_catalog::RuntimeCatalogSource for PublicEvents {
    async fn ensure_ready(&self, pool: &Db) -> Result<(), AppError> {
        current(pool).await?;
        Ok(())
    }

    fn backend_errors(&self) -> Vec<String> {
        readiness().backend_errors
    }
}

pub(crate) struct DeferredPluginPolicy;

#[async_trait::async_trait]
impl aphel_sdk::platform::infrastructure::deferred_authority::DeferredPluginPolicy
    for DeferredPluginPolicy
{
    async fn reauthorize(
        &self,
        pool: &Db,
        key: &str,
        scope: aphel_sdk::platform::domain::access::AccessScope,
    ) -> Result<aphel_sdk::platform::domain::access::AccessScope, AppError> {
        let snapshot = current(pool).await?;
        let plugin = snapshot.plugin(key).ok_or_else(|| {
            AppError::forbidden_code(
                "plugin_disabled",
                "the originating plugin is disabled or uninstalled",
            )
        })?;
        Ok(scope
            .restricted(|permission| plugin.manifest.allows(permission))
            .acting_as("plugin", key))
    }
}
