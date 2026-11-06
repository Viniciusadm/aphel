use async_trait::async_trait;
use chrono::{DateTime, Utc};

use super::ExperiencePreferences;
use crate::application::organization_profile::{
    ExperienceSection, OrganizationExperiencesRepository,
};
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
    pub experience: ExperiencePreferences,
}

impl Default for UserPreferences {
    fn default() -> Self {
        Self {
            keyboard_shortcuts: true,
            locale: None,
            navigation: NavigationPreferences::default(),
            experience: ExperiencePreferences::default(),
        }
    }
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
    type Experiences: OrganizationExperiencesRepository<Tx = UserPreferencesTx<Self>>;
    type Clock: Clock;

    fn transactions(&self) -> &Self::Transactions;
    fn experiences(&self) -> &Self::Experiences;
    fn omittable_sections(&self) -> Vec<ExperienceSection>;
    fn preferences(&self) -> &Self::Preferences;
    fn clock(&self) -> &Self::Clock;
}
