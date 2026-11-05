use std::sync::OnceLock;

use chrono::{DateTime, Duration, Utc};

use aphel_contract_automation::application::time::duration::parse_offset;
use aphel_sdk::platform::application::security::Clock;

pub(crate) const DEV_CLOCK_OFFSET_ENV: &str = "AUTOMATIONS_DEV_CLOCK_OFFSET";

#[derive(Debug, Clone, Copy)]
pub(crate) struct AutomationClock {
    offset: Duration,
}

impl AutomationClock {
    pub(crate) fn system() -> Self {
        Self {
            offset: Duration::zero(),
        }
    }

    pub(crate) fn from_env() -> Self {
        if !cfg!(debug_assertions) {
            return Self::system();
        }
        let offset = std::env::var(DEV_CLOCK_OFFSET_ENV)
            .ok()
            .filter(|value| !value.trim().is_empty())
            .and_then(|value| {
                let parsed = parse_offset(&value);
                if parsed.is_none() {
                    tracing::warn!("{DEV_CLOCK_OFFSET_ENV} ignored: {value:?} is not an offset");
                }
                parsed
            })
            .unwrap_or_else(Duration::zero);
        if !offset.is_zero() {
            tracing::warn!(
                "automations run with a shifted development clock ({DEV_CLOCK_OFFSET_ENV} = {} minutes)",
                offset.num_minutes()
            );
        }
        Self { offset }
    }
}

pub(crate) fn automation_clock() -> &'static AutomationClock {
    static CLOCK: OnceLock<AutomationClock> = OnceLock::new();
    CLOCK.get_or_init(AutomationClock::from_env)
}

impl Default for AutomationClock {
    fn default() -> Self {
        Self::from_env()
    }
}

impl Clock for AutomationClock {
    fn now(&self) -> DateTime<Utc> {
        Utc::now() + self.offset
    }
}
