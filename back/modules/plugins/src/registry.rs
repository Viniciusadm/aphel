use std::collections::HashMap;
use std::sync::{Arc, OnceLock, RwLock};
use std::time::{Duration, Instant};

use serde_json::{Map, Value};
use sqlx::MySqlPool;
use tokio::sync::Mutex;

use crate::manifest::PluginManifest;
use crate::{runtime, store};
use aphel_kernel::errors::AppError;

const CHECK_INTERVAL: Duration = Duration::from_secs(2);
const REFRESH_INTERVAL: Duration = Duration::from_secs(5);

pub struct LoadedPlugin {
    pub key: String,
    pub manifest: PluginManifest,
    pub settings: Map<String, Value>,
    pub backend: Option<runtime::Backend>,
    pub backend_error: Option<String>,
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
        self.plugin(key)
            .and_then(|plugin| plugin.backend_error.clone())
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
            .filter(|plugin| plugin.backend.is_some() && plugin.manifest.consumes(event_type))
            .cloned()
            .collect()
    }
}

type LoadedBackend = (Option<runtime::Backend>, Option<String>);

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
    cache.checked_at = Some(Instant::now());
    if snapshot.version == Some(version) {
        return Ok(snapshot);
    }
    let mut plugins = Vec::new();
    for installed in store::list(pool).await? {
        if !installed.enabled {
            continue;
        }
        let (backend, backend_error) = match cache.compiled.get(&installed.key) {
            Some((checksum, loaded)) if checksum == &installed.checksum => loaded.clone(),
            _ => {
                let backend = match store::backend(pool, &installed.key).await? {
                    Some(bytes) => match runtime::compile(bytes).await {
                        Ok(backend) => (Some(backend), None),
                        Err(error) => {
                            tracing::error!(plugin = %installed.key, error = %error, "the plugin backend could not be loaded; writes to the record types it hooks are refused and its events and routes are skipped");
                            (None, Some(error.client_message().to_owned()))
                        }
                    },
                    None => (None, None),
                };
                cache.compiled.insert(
                    installed.key.clone(),
                    (installed.checksum.clone(), backend.clone()),
                );
                backend
            }
        };
        plugins.push(Arc::new(LoadedPlugin {
            settings: installed.effective_settings(),
            key: installed.key,
            manifest: installed.manifest,
            backend,
            backend_error,
        }));
    }
    let snapshot = Arc::new(Snapshot {
        version: Some(version),
        plugins,
    });
    if let Ok(mut published) = published().write() {
        *published = snapshot.clone();
    }
    Ok(snapshot)
}

pub fn spawn_refresh(pool: MySqlPool) {
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
