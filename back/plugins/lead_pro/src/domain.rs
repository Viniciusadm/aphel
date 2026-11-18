use serde_json::Value;

use aphel_kernel::errors::AppError;
use aphel_sdk::notifications::NotificationKind;
use aphel_sdk::platform::database::Db;
use aphel_sdk::platform::domain::access::Permission;
use aphel_sdk::platform::modules::module_settings;

pub(crate) const KEY: &str = "lead_pro";
pub(crate) const TEMPERATURE_FIELD: &str = "lead_temperature";
pub(crate) const READ_PERMISSION: &str = "lead_pro.read";
pub(crate) const READ: Permission = Permission::from_static(READ_PERMISSION);
pub(crate) const TEMPERATURE_CHANGED: &str = "lead_pro.temperature_changed";
pub(crate) const LEAD_HOT: NotificationKind = NotificationKind::from_static("lead_pro_hot");
pub(crate) const LEAD_NOT_HOT_ENOUGH: &str = "lead_not_hot_enough";
pub(crate) const CONVERTED_TOUCH: &str = "lead.converted";
pub(crate) const MANUAL_TOUCH: &str = "manual";
pub(crate) const MAX_TEMPERATURE: i64 = 100;

pub(crate) const HOT_THRESHOLD_SETTING: &str = "hot_threshold";
pub(crate) const INITIAL_TEMPERATURE_SETTING: &str = "initial_temperature";
pub(crate) const TOUCH_POINTS_SETTING: &str = "touch_points";
pub(crate) const COOLDOWN_POINTS_SETTING: &str = "cooldown_points";
pub(crate) const COOLDOWN_DAYS_SETTING: &str = "cooldown_days";

pub(crate) const DEFAULT_HOT_THRESHOLD: i64 = 70;
pub(crate) const DEFAULT_INITIAL_TEMPERATURE: i64 = 20;
pub(crate) const DEFAULT_TOUCH_POINTS: i64 = 10;
pub(crate) const DEFAULT_COOLDOWN_POINTS: i64 = 5;
pub(crate) const DEFAULT_COOLDOWN_DAYS: i64 = 7;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LeadProSettings {
    pub(crate) hot_threshold: i64,
    pub(crate) initial_temperature: i64,
    pub(crate) touch_points: i64,
    pub(crate) cooldown_points: i64,
    pub(crate) cooldown_days: i64,
}

impl LeadProSettings {
    pub(crate) async fn load(pool: &Db) -> Result<Self, AppError> {
        let settings = module_settings(pool, KEY).await?;
        let number = |key: &str, default: i64| {
            settings
                .get(key)
                .and_then(Value::as_i64)
                .unwrap_or(default)
                .clamp(0, MAX_TEMPERATURE)
        };
        Ok(Self {
            hot_threshold: number(HOT_THRESHOLD_SETTING, DEFAULT_HOT_THRESHOLD),
            initial_temperature: number(INITIAL_TEMPERATURE_SETTING, DEFAULT_INITIAL_TEMPERATURE),
            touch_points: number(TOUCH_POINTS_SETTING, DEFAULT_TOUCH_POINTS),
            cooldown_points: number(COOLDOWN_POINTS_SETTING, DEFAULT_COOLDOWN_POINTS),
            cooldown_days: number(COOLDOWN_DAYS_SETTING, DEFAULT_COOLDOWN_DAYS),
        })
    }

    pub(crate) fn is_hot(&self, temperature: Option<i64>) -> bool {
        temperature.is_some_and(|value| value >= self.hot_threshold)
    }
}

pub(crate) fn heated(current: Option<i64>, settings: &LeadProSettings) -> i64 {
    (current.unwrap_or(settings.initial_temperature) + settings.touch_points).min(MAX_TEMPERATURE)
}
