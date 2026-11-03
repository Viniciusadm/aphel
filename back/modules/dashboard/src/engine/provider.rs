use super::DashboardPorts;
use aphel_kernel::{domain::access::AccessScope, errors::AppError};
pub use aphel_sdk::dashboard::provider::*;
use std::sync::Arc;
pub struct WidgetRegistry<P> {
    providers: Vec<Arc<dyn WidgetProvider<P>>>,
}

impl<P> Clone for WidgetRegistry<P> {
    fn clone(&self) -> Self {
        Self {
            providers: self.providers.clone(),
        }
    }
}

impl<P> WidgetRegistry<P>
where
    P: DashboardPorts,
{
    pub fn empty() -> Self {
        Self {
            providers: Vec::new(),
        }
    }

    pub fn register(mut self, provider: Arc<dyn WidgetProvider<P>>) -> Result<Self, AppError> {
        let key = provider.key();
        let valid = !key.is_empty()
            && key.len() <= 64
            && key.chars().all(|character| {
                character.is_ascii_lowercase() || character == '_' || character == '.'
            });
        if !valid {
            return Err(AppError::Configuration(format!(
                "widget key {key:?} must use lowercase letters, dots and underscores"
            )));
        }
        if self.get(key).is_some() {
            return Err(AppError::Configuration(format!(
                "widget {key} is registered twice"
            )));
        }
        self.providers.push(provider);
        Ok(self)
    }

    pub fn get(&self, key: &str) -> Option<&Arc<dyn WidgetProvider<P>>> {
        self.providers.iter().find(|provider| provider.key() == key)
    }

    pub fn providers(&self) -> impl Iterator<Item = &Arc<dyn WidgetProvider<P>>> {
        self.providers.iter()
    }

    pub fn available<'a>(
        &'a self,
        scope: &'a AccessScope,
    ) -> impl Iterator<Item = &'a Arc<dyn WidgetProvider<P>>> + 'a {
        self.providers
            .iter()
            .filter(move |provider| provider.is_available(scope))
    }
}
