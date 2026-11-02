use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};

use async_trait::async_trait;

use crate::application::custom_objects::{CustomSchema, build_registry};
use crate::application::fields::RecordTypeRegistry;
use crate::errors::AppError;

#[async_trait]
pub trait CustomSchemaSource: Send + Sync {
    async fn version(&self) -> Result<u64, AppError>;
    async fn load(&self) -> Result<CustomSchema, AppError>;
}

const VERSION_CHECK_INTERVAL: Duration = Duration::from_secs(1);

struct CachedRegistry {
    version: u64,
    checked: Mutex<Instant>,
    schema: Arc<CustomSchema>,
    registry: Arc<RecordTypeRegistry>,
}

pub struct RecordTypeCatalog {
    builtin: Arc<RecordTypeRegistry>,
    source: Option<Arc<dyn CustomSchemaSource>>,
    cache: RwLock<Option<Arc<CachedRegistry>>>,
}

impl RecordTypeCatalog {
    pub fn builtin_only(builtin: RecordTypeRegistry) -> Self {
        Self {
            builtin: Arc::new(builtin),
            source: None,
            cache: RwLock::new(None),
        }
    }

    pub fn new(builtin: RecordTypeRegistry, source: Arc<dyn CustomSchemaSource>) -> Self {
        Self {
            builtin: Arc::new(builtin),
            source: Some(source),
            cache: RwLock::new(None),
        }
    }

    pub fn builtin(&self) -> Arc<RecordTypeRegistry> {
        self.builtin.clone()
    }

    pub async fn registry(&self) -> Result<Arc<RecordTypeRegistry>, AppError> {
        Ok(self.entry().await?.registry.clone())
    }

    pub async fn schema(&self) -> Result<Arc<CustomSchema>, AppError> {
        Ok(self.entry().await?.schema.clone())
    }

    pub fn invalidate(&self) {
        if let Ok(mut cache) = self.cache.write() {
            *cache = None;
        }
    }

    async fn entry(&self) -> Result<Arc<CachedRegistry>, AppError> {
        let Some(source) = &self.source else {
            return Ok(Arc::new(CachedRegistry {
                version: 0,
                checked: Mutex::new(Instant::now()),
                schema: Arc::new(self.builtin.custom_schema().clone()),
                registry: self.builtin.clone(),
            }));
        };
        let cached = self.cache.read().ok().and_then(|cache| cache.clone());
        if let Some(cached) = &cached
            && cached
                .checked
                .lock()
                .is_ok_and(|checked| checked.elapsed() < VERSION_CHECK_INTERVAL)
        {
            return Ok(cached.clone());
        }
        let version = source.version().await?;
        if let Some(cached) = cached
            && cached.version == version
        {
            if let Ok(mut checked) = cached.checked.lock() {
                *checked = Instant::now();
            }
            return Ok(cached);
        }
        let schema = Arc::new(source.load().await?);
        let registry = build_registry(&self.builtin, &schema)?.with_custom_schema(schema.clone());
        let entry = Arc::new(CachedRegistry {
            version,
            checked: Mutex::new(Instant::now()),
            schema,
            registry: Arc::new(registry),
        });
        if let Ok(mut cache) = self.cache.write() {
            *cache = Some(entry.clone());
        }
        Ok(entry)
    }
}
