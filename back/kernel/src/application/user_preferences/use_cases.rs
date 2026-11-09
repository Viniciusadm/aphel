use crate::application::keybindings::{Keybindings, normalize_keybindings};
use crate::application::organization_profile::{
    CORE_UI_PLUGIN, effective_ui_plugin, is_ui_plugin_key, ui_plugin_available,
};
use crate::application::security::Clock;
use crate::application::shared::TransactionManager;
use crate::domain::UserId;
use crate::errors::{AppError, FieldViolation};

use super::ports::{
    KeybindingDefaults, NavigationPreferences, SUPPORTED_LOCALES, UiPluginDefaults,
    UserPreferences, UserPreferencesPorts, UserPreferencesRepository, UserPreferencesView,
};

const MAX_NAVIGATION_ITEMS: usize = 100;
const MAX_NAVIGATION_KEY_LENGTH: usize = 80;
const MAX_HOME_LENGTH: usize = 200;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct UpdateUserPreferencesCommand {
    pub keyboard_shortcuts: Option<bool>,
    pub keybindings: Option<Keybindings>,
    pub locale: Option<Option<String>>,
    pub navigation: Option<NavigationPreferences>,
    pub ui_plugin: Option<Option<String>>,
}

pub struct UserPreferencesUseCases<P: UserPreferencesPorts> {
    ports: P,
}

impl<P: UserPreferencesPorts> UserPreferencesUseCases<P> {
    pub fn new(ports: P) -> Self {
        Self { ports }
    }

    async fn view(&self, preferences: UserPreferences) -> Result<UserPreferencesView, AppError> {
        let ui_plugin_default = self.ports.ui_defaults().find().await?;
        let effective_ui_plugin = effective_ui_plugin(
            preferences.ui_plugin.as_deref(),
            ui_plugin_default.as_deref(),
        )
        .await?;
        let keybindings_default = self.ports.keybinding_defaults().find().await?;
        Ok(UserPreferencesView {
            preferences,
            ui_plugin_default,
            effective_ui_plugin,
            keybindings_default,
        })
    }

    pub async fn get(&self, user_id: &UserId) -> Result<UserPreferencesView, AppError> {
        let stored = self
            .ports
            .preferences()
            .find(user_id)
            .await?
            .unwrap_or_default();
        self.view(stored).await
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
        if let Some(Some(key)) = &command.ui_plugin
            && key != CORE_UI_PLUGIN
            && (!is_ui_plugin_key(key) || !ui_plugin_available(key).await?)
        {
            violations.push(FieldViolation::new(
                "ui_plugin",
                "must be an enabled plugin that provides an interface",
            ));
        }
        let mut command = command;
        if let Some(navigation) = command.navigation.take() {
            command.navigation = Some(normalize_navigation(navigation, &mut violations));
        }
        if let Some(keybindings) = command.keybindings.take() {
            command.keybindings = Some(normalize_keybindings(
                "keybindings",
                keybindings,
                &mut violations,
            ));
        }
        if !violations.is_empty() {
            return Err(AppError::invalid_fields("invalid preferences", violations));
        }
        let mut tx = self.ports.transactions().begin().await?;
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
        if let Some(value) = &command.keybindings {
            next.keybindings = value.clone();
        }
        if let Some(value) = &command.locale {
            next.locale = value.clone();
        }
        if let Some(value) = &command.navigation {
            next.navigation = value.clone();
        }
        if let Some(value) = &command.ui_plugin {
            next.ui_plugin = value.clone();
        }
        self.ports
            .preferences()
            .save(&mut tx, user_id, &next, self.ports.clock().now())
            .await?;
        self.ports.transactions().commit(tx).await?;
        self.view(next).await
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
