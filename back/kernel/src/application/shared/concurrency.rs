use chrono::{DateTime, Utc};

use crate::errors::AppError;

pub const VERSION_CONFLICT: &str = "version_conflict";

pub fn ensure_version(
    expected: Option<DateTime<Utc>>,
    current: DateTime<Utc>,
) -> Result<(), AppError> {
    let Some(expected) = expected else {
        return Ok(());
    };
    let nanos = expected.timestamp_subsec_nanos();
    let matches = if nanos == 0 {
        expected.timestamp() == current.timestamp()
    } else if nanos % 1_000_000 == 0 {
        expected.timestamp_millis() == current.timestamp_millis()
            || expected.timestamp_micros() == current.timestamp_micros()
    } else {
        expected.timestamp_micros() == current.timestamp_micros()
    };
    if matches {
        Ok(())
    } else {
        Err(AppError::conflict_code(
            VERSION_CONFLICT,
            "the resource changed since it was read; reload it and apply the change again",
        ))
    }
}
