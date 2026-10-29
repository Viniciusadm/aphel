use crate::application::security::Clock;
use crate::domain::UserId;
use crate::errors::{AppError, FieldViolation};

use super::ports::{
    NavigationPreferences, SUPPORTED_LOCALES, UserPreferences, UserPreferencesPorts,
    UserPreferencesRepository,
};

const MAX_NAVIGATION_ITEMS: usize = 100;
const MAX_NAVIGATION_KEY_LENGTH: usize = 80;
const MAX_HOME_LENGTH: usize = 200;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct UpdateUserPreferencesCommand {
    pub keyboard_shortcuts: Option<bool>,
    pub locale: Option<Option<String>>,
    pub navigation: Option<NavigationPreferences>,
}

pub struct UserPreferencesUseCases<P: UserPreferencesPorts> {
    ports: P,
}

impl<P: UserPreferencesPorts> UserPreferencesUseCases<P> {
    pub fn new(ports: P) -> Self {
        Self { ports }
    }

    pub async fn get(&self, user_id: &UserId) -> Result<UserPreferences, AppError> {
        Ok(self
            .ports
            .preferences()
            .find(user_id)
            .await?
            .unwrap_or_default())
    }

    pub async fn update(
        &self,
        user_id: &UserId,
        command: UpdateUserPreferencesCommand,
    ) -> Result<UserPreferences, AppError> {
        let mut violations = Vec::new();
        if let Some(Some(locale)) = &command.locale
            && !SUPPORTED_LOCALES.contains(&locale.as_str())
        {
            violations.push(FieldViolation::new(
                "locale",
                format!("must be one of {}", SUPPORTED_LOCALES.join(", ")),
            ));
        }
        let navigation = command
            .navigation
            .map(|navigation| normalize_navigation(navigation, &mut violations));
        if !violations.is_empty() {
            return Err(AppError::invalid_fields("invalid preferences", violations));
        }

        let current = self.get(user_id).await?;
        let next = UserPreferences {
            keyboard_shortcuts: command
                .keyboard_shortcuts
                .unwrap_or(current.keyboard_shortcuts),
            locale: command.locale.unwrap_or(current.locale),
            navigation: navigation.unwrap_or(current.navigation),
        };
        self.ports
            .preferences()
            .save(user_id, &next, self.ports.clock().now())
            .await?;
        Ok(next)
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
