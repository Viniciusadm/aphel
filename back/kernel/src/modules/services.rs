use std::any::{Any, TypeId, type_name};
use std::collections::HashMap;
use std::sync::Arc;

use crate::errors::AppError;
use crate::modules::enabled::EnabledModules;

struct Layer {
    module: &'static str,
    value: Box<dyn Any + Send + Sync>,
}

struct ServiceEntry {
    type_name: &'static str,
    layers: Vec<Layer>,
    previous: Vec<Layer>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceInfo {
    pub type_name: &'static str,
    pub module: &'static str,
    pub decorators: Vec<&'static str>,
    pub replaced: Vec<&'static str>,
}

#[derive(Default)]
pub struct Services {
    entries: HashMap<TypeId, ServiceEntry>,
    enabled: Option<Arc<EnabledModules>>,
}

impl Services {
    pub fn new(enabled: Arc<EnabledModules>) -> Self {
        Self {
            entries: HashMap::new(),
            enabled: Some(enabled),
        }
    }

    fn is_enabled(&self, module: &str) -> bool {
        self.enabled
            .as_ref()
            .is_none_or(|enabled| enabled.is_enabled(module))
    }

    pub fn get<T>(&self) -> Option<Arc<T>>
    where
        T: ?Sized + Send + Sync + 'static,
    {
        let entry = self.entries.get(&TypeId::of::<Arc<T>>())?;
        let layer = entry
            .layers
            .iter()
            .take_while(|layer| self.is_enabled(layer.module))
            .last()
            .or_else(|| {
                entry
                    .previous
                    .iter()
                    .rev()
                    .find(|layer| self.is_enabled(layer.module))
            })?;
        layer.value.downcast_ref::<Arc<T>>().cloned()
    }

    pub fn require<T>(&self) -> Result<Arc<T>, AppError>
    where
        T: ?Sized + Send + Sync + 'static,
    {
        self.get::<T>().ok_or_else(|| {
            AppError::Internal(format!(
                "no enabled module provides the service {}",
                type_name::<T>()
            ))
        })
    }

    pub fn provide<T>(&mut self, module: &'static str, service: Arc<T>)
    where
        T: ?Sized + Send + Sync + 'static,
    {
        let layer = Layer {
            module,
            value: Box::new(service),
        };
        match self.entries.get_mut(&TypeId::of::<Arc<T>>()) {
            Some(entry) => {
                let mut layers = std::mem::take(&mut entry.layers).into_iter();
                if let Some(provider) = layers.next() {
                    tracing::info!(
                        service = type_name::<T>(),
                        previous = provider.module,
                        replacement = module,
                        "module service replaced"
                    );
                    entry.previous.push(provider);
                }
                entry.layers = vec![layer];
            }
            None => {
                self.entries.insert(
                    TypeId::of::<Arc<T>>(),
                    ServiceEntry {
                        type_name: type_name::<T>(),
                        layers: vec![layer],
                        previous: Vec::new(),
                    },
                );
            }
        }
    }

    pub fn decorate<T, F>(&mut self, module: &'static str, wrap: F) -> Result<(), AppError>
    where
        T: ?Sized + Send + Sync + 'static,
        F: FnOnce(Arc<T>) -> Arc<T>,
    {
        let entry = self
            .entries
            .get_mut(&TypeId::of::<Arc<T>>())
            .ok_or_else(|| {
                AppError::Configuration(format!(
                    "module {module} decorates {} but no module provides it",
                    type_name::<T>()
                ))
            })?;
        let inner = entry
            .layers
            .last()
            .and_then(|layer| layer.value.downcast_ref::<Arc<T>>())
            .cloned()
            .ok_or_else(|| {
                AppError::Internal(format!("service {} is corrupt", type_name::<T>()))
            })?;
        entry.layers.push(Layer {
            module,
            value: Box::new(wrap(inner)),
        });
        Ok(())
    }

    pub fn describe(&self) -> Vec<ServiceInfo> {
        let mut services: Vec<ServiceInfo> = self
            .entries
            .values()
            .map(|entry| ServiceInfo {
                type_name: entry.type_name,
                module: entry.layers.first().map_or("", |layer| layer.module),
                decorators: entry
                    .layers
                    .iter()
                    .skip(1)
                    .map(|layer| layer.module)
                    .collect(),
                replaced: entry.previous.iter().map(|layer| layer.module).collect(),
            })
            .collect();
        services.sort_by(|left, right| left.type_name.cmp(right.type_name));
        services
    }
}
