use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::application::security::Clock;
use crate::application::shared::TransactionManager;
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
    pub ui_plugin: Option<String>,
}

impl Default for UserPreferences {
    fn default() -> Self {
        Self {
            keyboard_shortcuts: true,
            locale: None,
            navigation: NavigationPreferences::default(),
            ui_plugin: None,
        }
    }
}

#[async_trait]
pub trait UiPluginDefaults: Send + Sync {
    async fn find(&self) -> Result<Option<String>, AppError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserPreferencesView {
    pub preferences: UserPreferences,
    pub ui_plugin_default: Option<String>,
    pub effective_ui_plugin: Option<String>,
}

#[async_trait]
pub trait UserPreferencesRepository: Send + Sync {
    type Tx: Send;
    async fn lock(
        &self,
        tx: &mut Self::Tx,
        user_id: &UserId,
    ) -> Result<Option<UserPreferences>, AppError>;
    async fn find(&self, user_id: &UserId) -> Result<Option<UserPreferences>, AppError>;
    async fn save(
        &self,
        tx: &mut Self::Tx,
        user_id: &UserId,
        preferences: &UserPreferences,
        at: DateTime<Utc>,
    ) -> Result<(), AppError>;
}

pub type UserPreferencesTx<P> =
    <<P as UserPreferencesPorts>::Transactions as TransactionManager>::Tx;

pub trait UserPreferencesPorts: Send + Sync {
    type Transactions: TransactionManager;
    type Preferences: UserPreferencesRepository<Tx = UserPreferencesTx<Self>>;
    type UiDefaults: UiPluginDefaults;
    type Clock: Clock;

    fn transactions(&self) -> &Self::Transactions;
    fn ui_defaults(&self) -> &Self::UiDefaults;
    fn preferences(&self) -> &Self::Preferences;
    fn clock(&self) -> &Self::Clock;
}
