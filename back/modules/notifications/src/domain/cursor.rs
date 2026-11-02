use chrono::{DateTime, Utc};
use uuid::Uuid;

const MAX_CURSOR_LENGTH: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NotificationCursor {
    pub created_at: DateTime<Utc>,
    pub id: Uuid,
}

impl NotificationCursor {
    pub fn new(created_at: DateTime<Utc>, id: Uuid) -> Self {
        Self { created_at, id }
    }

    pub fn encode(&self) -> String {
        format!("{}|{}", self.created_at.timestamp_micros(), self.id)
            .bytes()
            .map(|byte| format!("{byte:02x}"))
            .collect()
    }

    pub fn decode(value: &str) -> Option<Self> {
        if value.is_empty() || !value.len().is_multiple_of(2) || value.len() > MAX_CURSOR_LENGTH {
            return None;
        }
        let bytes = (0..value.len())
            .step_by(2)
            .map(|index| u8::from_str_radix(value.get(index..index + 2)?, 16).ok())
            .collect::<Option<Vec<u8>>>()?;
        let text = String::from_utf8(bytes).ok()?;
        let (micros, id) = text.split_once('|')?;
        let created_at = DateTime::<Utc>::from_timestamp_micros(micros.parse().ok()?)?;
        Some(Self::new(created_at, Uuid::parse_str(id).ok()?))
    }
}
