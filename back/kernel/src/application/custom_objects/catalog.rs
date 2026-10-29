use std::sync::{Arc, RwLock};

use async_trait::async_trait;

use crate::application::custom_objects::{CustomSchema, build_registry};
use crate::application::fields::RecordTypeRegistry;
use crate::errors::AppError;

#[async_trait]
pub trait CustomSchemaSource: Send + Sync {
    async fn version(&self) -> Result<u64, AppError>;
    async fn load(&self) -> Result<CustomSchema, AppError>;
}

struct CachedRegistry {
    version: u64,
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
                schema: Arc::new(CustomSchema::default()),
                registry: self.builtin.clone(),
            }));
        };
        let version = source.version().await?;
        let cached = self.cache.read().ok().and_then(|cache| cache.clone());
        if let Some(cached) = cached
            && cached.version == version
        {
            return Ok(cached);
        }
        let schema = Arc::new(source.load().await?);
        let registry = build_registry(&self.builtin, &schema)?.with_custom_schema(schema.clone());
        let entry = Arc::new(CachedRegistry {
            version,
            schema,
            registry: Arc::new(registry),
        });
        if let Ok(mut cache) = self.cache.write() {
            *cache = Some(entry.clone());
        }
        Ok(entry)
    }
}
