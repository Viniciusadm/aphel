use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{MySqlPool, Row, mysql::MySqlRow};

use crate::application::keybindings::{keybindings_from_json, keybindings_to_json};
use crate::application::user_preferences::{
    NavigationPreferences, UserPreferences, UserPreferencesPorts, UserPreferencesRepository,
};
use crate::database::transaction::MySqlTx;
use crate::domain::UserId;
use crate::errors::AppError;
use crate::infrastructure::auth::SystemClock;
use crate::infrastructure::organization_profile::{MySqlKeybindingDefaults, MySqlUiPluginDefaults};
use crate::infrastructure::repositories::mapper;
use crate::infrastructure::transactions::MySqlTransactionManager;

#[derive(Clone)]
pub(crate) struct MySqlUserPreferencesRepository {
    pool: MySqlPool,
}

impl MySqlUserPreferencesRepository {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

fn decode(row: MySqlRow) -> Result<UserPreferences, AppError> {
    let navigation: Option<serde_json::Value> = row.try_get("navigation")?;
    let keybindings: Option<serde_json::Value> = row.try_get("keybindings")?;
    Ok(UserPreferences {
        keyboard_shortcuts: row
            .try_get::<Option<bool>, _>("keyboard_shortcuts")?
            .unwrap_or(true),
        keybindings: keybindings
            .map(|value| keybindings_from_json(&value))
            .unwrap_or_default(),
        locale: row.try_get("locale")?,
        navigation: navigation
            .map(|value| navigation_from_json(&value))
            .unwrap_or_default(),
        ui_plugin: row.try_get("ui_plugin")?,
    })
}

#[async_trait]
impl UserPreferencesRepository for MySqlUserPreferencesRepository {
    type Tx = MySqlTx;

    async fn find(&self, user_id: &UserId) -> Result<Option<UserPreferences>, AppError> {
        sqlx::query("SELECT u.locale, p.keyboard_shortcuts, p.keybindings, p.navigation, p.ui_plugin FROM users u LEFT JOIN user_preferences p ON p.user_id = u.id WHERE u.id = ?")
            .bind(user_id.as_str()).fetch_optional(&self.pool).await?.map(decode).transpose()
    }

    async fn lock(
        &self,
        tx: &mut MySqlTx,
        user_id: &UserId,
    ) -> Result<Option<UserPreferences>, AppError> {
        sqlx::query("SELECT u.locale, p.keyboard_shortcuts, p.keybindings, p.navigation, p.ui_plugin FROM users u LEFT JOIN user_preferences p ON p.user_id = u.id WHERE u.id = ? FOR UPDATE")
            .bind(user_id.as_str()).fetch_optional(&mut **tx).await?.map(decode).transpose()
    }

    async fn save(
        &self,
        tx: &mut MySqlTx,
        user_id: &UserId,
        preferences: &UserPreferences,
        at: DateTime<Utc>,
    ) -> Result<(), AppError> {
        let navigation = navigation_to_json(&preferences.navigation);
        let keybindings = keybindings_to_json(&preferences.keybindings);
        sqlx::query("INSERT INTO user_preferences (user_id, keyboard_shortcuts, keybindings, navigation, ui_plugin, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?) AS new ON DUPLICATE KEY UPDATE keyboard_shortcuts = new.keyboard_shortcuts, keybindings = new.keybindings, navigation = new.navigation, ui_plugin = new.ui_plugin, updated_at = new.updated_at")
            .bind(user_id.as_str()).bind(preferences.keyboard_shortcuts).bind(&keybindings).bind(&navigation).bind(&preferences.ui_plugin)
            .bind(mapper::naive(at)).bind(mapper::naive(at)).execute(&mut **tx).await?;
        sqlx::query("UPDATE users SET locale = ? WHERE id = ?")
            .bind(&preferences.locale)
            .bind(user_id.as_str())
            .execute(&mut **tx)
            .await?;
        Ok(())
    }
}

fn navigation_to_json(navigation: &NavigationPreferences) -> serde_json::Value {
    serde_json::json!({
        "order": navigation.order,
        "hidden": navigation.hidden,
        "home": navigation.home,
    })
}

fn navigation_from_json(value: &serde_json::Value) -> NavigationPreferences {
    let keys = |name: &str| -> Vec<String> {
        value
            .get(name)
            .and_then(serde_json::Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| item.as_str().map(str::to_owned))
                    .collect()
            })
            .unwrap_or_default()
    };
    NavigationPreferences {
        order: keys("order"),
        hidden: keys("hidden"),
        home: value
            .get("home")
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned),
    }
}

pub(crate) struct MySqlUserPreferencesPorts {
    transactions: MySqlTransactionManager,
    preferences: MySqlUserPreferencesRepository,
    ui_defaults: MySqlUiPluginDefaults,
    keybinding_defaults: MySqlKeybindingDefaults,
}

impl MySqlUserPreferencesPorts {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self {
            transactions: MySqlTransactionManager::new(pool.clone()),
            preferences: MySqlUserPreferencesRepository::new(pool.clone()),
            ui_defaults: MySqlUiPluginDefaults::new(pool.clone()),
            keybinding_defaults: MySqlKeybindingDefaults::new(pool),
        }
    }
}

impl UserPreferencesPorts for MySqlUserPreferencesPorts {
    type Transactions = MySqlTransactionManager;
    type Preferences = MySqlUserPreferencesRepository;
    type UiDefaults = MySqlUiPluginDefaults;
    type KeybindingDefaults = MySqlKeybindingDefaults;
    type Clock = SystemClock;

    fn transactions(&self) -> &Self::Transactions {
        &self.transactions
    }
    fn ui_defaults(&self) -> &Self::UiDefaults {
        &self.ui_defaults
    }
    fn keybinding_defaults(&self) -> &Self::KeybindingDefaults {
        &self.keybinding_defaults
    }
    fn preferences(&self) -> &Self::Preferences {
        &self.preferences
    }
    fn clock(&self) -> &Self::Clock {
        &SystemClock
    }
}
