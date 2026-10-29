use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::application::security::Clock;
use crate::domain::UserId;
use crate::errors::AppError;

pub const SUPPORTED_LOCALES: [&str; 2] = ["pt-BR", "en"];

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct NavigationPreferences {
    pub order: Vec<String>,
    pub hidden: Vec<String>,
    pub home: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserPreferences {
    pub keyboard_shortcuts: bool,
    pub locale: Option<String>,
    pub navigation: NavigationPreferences,
}

impl Default for UserPreferences {
    fn default() -> Self {
        Self {
            keyboard_shortcuts: true,
            locale: None,
            navigation: NavigationPreferences::default(),
        }
    }
}

#[async_trait]
pub trait UserPreferencesRepository: Send + Sync {
    async fn find(&self, user_id: &UserId) -> Result<Option<UserPreferences>, AppError>;
    async fn save(
        &self,
        user_id: &UserId,
        preferences: &UserPreferences,
        at: DateTime<Utc>,
    ) -> Result<(), AppError>;
}

pub trait UserPreferencesPorts: Send + Sync {
    type Preferences: UserPreferencesRepository;
    type Clock: Clock;

    fn preferences(&self) -> &Self::Preferences;
    fn clock(&self) -> &Self::Clock;
}
