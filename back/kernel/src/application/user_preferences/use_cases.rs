use super::{
    UpdateExperiencePreferences, UserPreferencesView, personal_preferences_view,
    validate_personal_experience_update,
};
use crate::application::organization_profile::OrganizationExperiencesRepository;
use crate::application::security::Clock;
use crate::application::shared::TransactionManager;
use crate::domain::UserId;
use crate::errors::{AppError, FieldViolation};

use super::ports::{
    NavigationPreferences, SUPPORTED_LOCALES, UserPreferencesPorts, UserPreferencesRepository,
};

const MAX_NAVIGATION_ITEMS: usize = 100;
const MAX_NAVIGATION_KEY_LENGTH: usize = 80;
const MAX_HOME_LENGTH: usize = 200;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct UpdateUserPreferencesCommand {
    pub keyboard_shortcuts: Option<bool>,
    pub locale: Option<Option<String>>,
    pub navigation: Option<NavigationPreferences>,
    pub experience: Option<Option<UpdateExperiencePreferences>>,
}

pub struct UserPreferencesUseCases<P: UserPreferencesPorts> {
    ports: P,
}

impl<P: UserPreferencesPorts> UserPreferencesUseCases<P> {
    pub fn new(ports: P) -> Self {
        Self { ports }
    }

    pub async fn get(&self, user_id: &UserId) -> Result<UserPreferencesView, AppError> {
        let stored = self
            .ports
            .preferences()
            .find(user_id)
            .await?
            .unwrap_or_default();
        let configuration = self.ports.experiences().find().await?.configuration;
        Ok(personal_preferences_view(
            stored,
            configuration.as_ref(),
            &self.ports.omittable_sections(),
        ))
    }

    pub async fn update(
        &self,
        user_id: &UserId,
        command: UpdateUserPreferencesCommand,
    ) -> Result<UserPreferencesView, AppError> {
        let mut violations = Vec::new();
        if let Some(Some(locale)) = &command.locale
            && !SUPPORTED_LOCALES.contains(&locale.as_str())
        {
            violations.push(FieldViolation::new(
                "locale",
                format!("must be one of {}", SUPPORTED_LOCALES.join(", ")),
            ));
        }
        let mut command = command;
        if let Some(navigation) = command.navigation.take() {
            command.navigation = Some(normalize_navigation(navigation, &mut violations));
        }
        if !violations.is_empty() {
            return Err(AppError::invalid_fields("invalid preferences", violations));
        }
        let mut tx = self.ports.transactions().begin().await?;
        let configuration = self
            .ports
            .experiences()
            .lock_shared(&mut tx)
            .await?
            .configuration;
        let current = self
            .ports
            .preferences()
            .lock(&mut tx, user_id)
            .await?
            .ok_or_else(|| AppError::NotFound("user was not found".to_owned()))?;
        let mut next = current;
        if let Some(value) = command.keyboard_shortcuts {
            next.keyboard_shortcuts = value;
        }
        if let Some(value) = &command.locale {
            next.locale = value.clone();
        }
        if let Some(value) = &command.navigation {
            next.navigation = value.clone();
        }
        if let Some(change) = &command.experience {
            match change {
                None => next.experience = Default::default(),
                Some(change) => {
                    if let Some(value) = &change.profile {
                        next.experience.profile = value.clone();
                    }
                    if let Some(value) = &change.record_detail {
                        next.experience.record_detail = value.clone();
                    }
                }
            }
        }
        let omittable = self.ports.omittable_sections();
        let violations = validate_personal_experience_update(
            &command,
            &next,
            configuration.as_ref(),
            &omittable,
        );
        if !violations.is_empty() {
            return Err(AppError::invalid_fields("invalid preferences", violations));
        }
        self.ports
            .preferences()
            .save(&mut tx, user_id, &next, self.ports.clock().now())
            .await?;
        self.ports.transactions().commit(tx).await?;
        Ok(personal_preferences_view(
            next,
            configuration.as_ref(),
            &omittable,
        ))
    }
}

fn normalize_keys(
    field: &str,
    keys: Vec<String>,
    violations: &mut Vec<FieldViolation>,
) -> Vec<String> {
    if keys.len() > MAX_NAVIGATION_ITEMS {
        violations.push(FieldViolation::new(
            field,
            format!("must have at most {MAX_NAVIGATION_ITEMS} items"),
        ));
        return Vec::new();
    }
    let mut seen = Vec::new();
    for key in keys {
        let key = key.trim().to_owned();
        let valid = !key.is_empty()
            && key.len() <= MAX_NAVIGATION_KEY_LENGTH
            && key
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | ':' | '.' | '/'));
        if !valid {
            violations.push(FieldViolation::new(field, "contains an invalid item key"));
            return Vec::new();
        }
        if !seen.contains(&key) {
            seen.push(key);
        }
    }
    seen
}

fn normalize_navigation(
    navigation: NavigationPreferences,
    violations: &mut Vec<FieldViolation>,
) -> NavigationPreferences {
    let order = normalize_keys("navigation.order", navigation.order, violations);
    let hidden = normalize_keys("navigation.hidden", navigation.hidden, violations);
    let home = navigation
        .home
        .map(|home| home.trim().to_owned())
        .filter(|home| !home.is_empty());
    if let Some(home) = &home {
        let safe = home.starts_with('/')
            && !home.starts_with("//")
            && !home.contains('\\')
            && !home.contains("://")
            && home.len() <= MAX_HOME_LENGTH
            && !home.chars().any(char::is_whitespace);
        if !safe {
            violations.push(FieldViolation::new(
                "navigation.home",
                "must be an internal path starting with /",
            ));
        }
    }
    NavigationPreferences {
        order,
        hidden,
        home,
    }
}
