use std::collections::HashMap;
use std::sync::{Arc, OnceLock, RwLock};
use std::time::{Duration, Instant};

use serde_json::{Map, Value};
use sqlx::MySqlPool;
use tokio::sync::Mutex;

use crate::manifest::PluginManifest;
use crate::widgets::PluginWidget;
use crate::{notify, runtime, store};
use aphel_kernel::errors::AppError;
use aphel_sdk::notifications as intern;

const CHECK_INTERVAL: Duration = Duration::from_secs(2);
const REFRESH_INTERVAL: Duration = Duration::from_secs(5);

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
                let pool = aphel_kernel::state::runtime_pool()?;
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

struct Cache {
    checked_at: Option<Instant>,
    compiled: HashMap<String, (String, LoadedBackend)>,
}

fn cache() -> &'static Mutex<Cache> {
    static CACHE: OnceLock<Mutex<Cache>> = OnceLock::new();
    CACHE.get_or_init(|| {
        Mutex::new(Cache {
            checked_at: None,
            compiled: HashMap::new(),
        })
    })
}

fn published() -> &'static RwLock<Arc<Snapshot>> {
    static PUBLISHED: OnceLock<RwLock<Arc<Snapshot>>> = OnceLock::new();
    PUBLISHED.get_or_init(|| RwLock::new(Arc::new(Snapshot::default())))
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
    published()
        .read()
        .map(|snapshot| snapshot.clone())
        .unwrap_or_default()
}

pub async fn invalidate() {
    cache().lock().await.checked_at = None;
}

pub async fn current(pool: &MySqlPool) -> Result<Arc<Snapshot>, AppError> {
    load(pool).await.map_err(|error| {
        tracing::warn!(error = %error, "plugin catalog is unavailable");
        AppError::unavailable_code(
            "plugin_catalog_unavailable",
            "the plugin catalog is not ready; retry the request",
        )
    })
}

async fn load(pool: &MySqlPool) -> Result<Arc<Snapshot>, AppError> {
    let mut cache = cache().lock().await;
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
        cache.checked_at = Some(Instant::now());
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
    cache.checked_at = Some(Instant::now());
    let snapshot = Arc::new(Snapshot {
        version: Some(version),
        plugins,
    });
    publish_extensions(&snapshot);
    if let Ok(mut published) = published().write() {
        *published = snapshot.clone();
    }
    Ok(snapshot)
}

fn publish_extensions(snapshot: &Snapshot) {
    let mut events = Vec::new();
    let mut kinds = Vec::new();
    let mut widgets = Vec::new();
    for plugin in &snapshot.plugins {
        for spec in &plugin.manifest.emits {
            use aphel_kernel::application::public_events::{PublicResource, event};
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
    let mut batch = aphel_kernel::modules::RuntimeBatch::new();
    for kind in kinds {
        batch.add(std::sync::Arc::new(kind));
    }
    for widget in widgets {
        batch.add::<dyn ::dashboard::engine::WidgetProvider<
                ::dashboard::repository::MySqlDashboardPorts,
            >>(widget);
    }
    for event in events {
        batch.add(std::sync::Arc::new(event));
    }
    aphel_kernel::modules::app().replace_runtime(crate::KEY, batch);
}

pub fn spawn_refresh(pool: MySqlPool) {
    if tokio::runtime::Handle::try_current().is_err() {
        return;
    }
    static STARTED: OnceLock<()> = OnceLock::new();
    if STARTED.set(()).is_err() {
        return;
    }
    tokio::spawn(async move {
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
impl aphel_kernel::application::public_events::RuntimePublicEventSource for PublicEvents {
    fn recognizes(&self, event_type: &str) -> bool {
        event_type.starts_with("plugin.")
    }

    async fn refresh(&self, pool: &MySqlPool) -> Result<(), AppError> {
        current(pool).await?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl aphel_kernel::modules::runtime_catalog::RuntimeCatalogSource for PublicEvents {
    async fn ensure_ready(&self, pool: &MySqlPool) -> Result<(), AppError> {
        current(pool).await?;
        Ok(())
    }
}

pub(crate) struct DeferredPluginPolicy;

#[async_trait::async_trait]
impl aphel_kernel::infrastructure::deferred_authority::DeferredPluginPolicy
    for DeferredPluginPolicy
{
    async fn reauthorize(
        &self,
        pool: &MySqlPool,
        key: &str,
        scope: aphel_kernel::domain::access::AccessScope,
    ) -> Result<aphel_kernel::domain::access::AccessScope, AppError> {
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
