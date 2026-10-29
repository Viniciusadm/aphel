use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{MySqlPool, Row};

use crate::application::user_preferences::{
    NavigationPreferences, UserPreferences, UserPreferencesPorts, UserPreferencesRepository,
};
use crate::domain::UserId;
use crate::errors::AppError;
use crate::infrastructure::auth::SystemClock;
use crate::infrastructure::repositories::mapper;

#[derive(Clone)]
pub(crate) struct MySqlUserPreferencesRepository {
    pool: MySqlPool,
}

impl MySqlUserPreferencesRepository {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl UserPreferencesRepository for MySqlUserPreferencesRepository {
    async fn find(&self, user_id: &UserId) -> Result<Option<UserPreferences>, AppError> {
        let Some(row) = sqlx::query(
            "SELECT u.locale, p.keyboard_shortcuts, p.navigation \
             FROM users u LEFT JOIN user_preferences p ON p.user_id = u.id WHERE u.id = ?",
        )
        .bind(user_id.as_str())
        .fetch_optional(&self.pool)
        .await?
        else {
            return Ok(None);
        };
        let navigation: Option<serde_json::Value> = row.try_get("navigation")?;
        Ok(Some(UserPreferences {
            keyboard_shortcuts: row
                .try_get::<Option<bool>, _>("keyboard_shortcuts")?
                .unwrap_or(true),
            locale: row.try_get("locale")?,
            navigation: navigation
                .map(|value| navigation_from_json(&value))
                .unwrap_or_default(),
        }))
    }

    async fn save(
        &self,
        user_id: &UserId,
        preferences: &UserPreferences,
        at: DateTime<Utc>,
    ) -> Result<(), AppError> {
        let navigation = navigation_to_json(&preferences.navigation);
        let mut transaction = self.pool.begin().await?;
        sqlx::query(
            "INSERT INTO user_preferences (user_id, keyboard_shortcuts, navigation, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?) \
             ON DUPLICATE KEY UPDATE keyboard_shortcuts = ?, navigation = ?, updated_at = ?",
        )
        .bind(user_id.as_str())
        .bind(preferences.keyboard_shortcuts)
        .bind(&navigation)
        .bind(mapper::naive(at))
        .bind(mapper::naive(at))
        .bind(preferences.keyboard_shortcuts)
        .bind(&navigation)
        .bind(mapper::naive(at))
        .execute(&mut *transaction)
        .await?;
        sqlx::query("UPDATE users SET locale = ? WHERE id = ?")
            .bind(&preferences.locale)
            .bind(user_id.as_str())
            .execute(&mut *transaction)
            .await?;
        transaction.commit().await?;
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
    preferences: MySqlUserPreferencesRepository,
}

impl MySqlUserPreferencesPorts {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self {
            preferences: MySqlUserPreferencesRepository::new(pool),
        }
    }
}

impl UserPreferencesPorts for MySqlUserPreferencesPorts {
    type Preferences = MySqlUserPreferencesRepository;
    type Clock = SystemClock;

    fn preferences(&self) -> &Self::Preferences {
        &self.preferences
    }

    fn clock(&self) -> &Self::Clock {
        &SystemClock
    }
}
