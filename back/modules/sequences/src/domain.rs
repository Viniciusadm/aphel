use chrono::{Datelike, Duration, NaiveDate, NaiveDateTime, NaiveTime, Weekday};

pub(crate) const MINUTES_PER_DAY: u16 = 24 * 60;
pub(crate) const DEFAULT_WINDOW_START: u16 = 8 * 60;
pub(crate) const DEFAULT_WINDOW_END: u16 = 18 * 60;
pub(crate) const DEFAULT_DAILY_LIMIT: u32 = 50;
pub(crate) const MAX_DAILY_LIMIT: u32 = 2_000;
pub(crate) const MAX_WAIT_DAYS: u32 = 365;
pub(crate) const MAX_STEPS: usize = 30;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum StepKind {
    AutoEmail,
    ManualEmail,
    Call,
    Linkedin,
    Task,
    Wait,
}

impl StepKind {
    pub(crate) const ALL: [Self; 6] = [
        Self::AutoEmail,
        Self::ManualEmail,
        Self::Call,
        Self::Linkedin,
        Self::Task,
        Self::Wait,
    ];

    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::AutoEmail => "auto_email",
            Self::ManualEmail => "manual_email",
            Self::Call => "call",
            Self::Linkedin => "linkedin",
            Self::Task => "task",
            Self::Wait => "wait",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.as_str() == value)
    }

    pub(crate) fn is_email(&self) -> bool {
        matches!(self, Self::AutoEmail | Self::ManualEmail)
    }

    pub(crate) fn activity_type_key(&self) -> Option<&'static str> {
        match self {
            Self::ManualEmail => Some("email"),
            Self::Call => Some("call"),
            Self::Linkedin | Self::Task => Some("task"),
            Self::AutoEmail | Self::Wait => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum EnrollmentStatus {
    Active,
    Paused,
    Completed,
    Exited,
}

impl EnrollmentStatus {
    pub(crate) const ALL: [Self; 4] = [Self::Active, Self::Paused, Self::Completed, Self::Exited];

    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Paused => "paused",
            Self::Completed => "completed",
            Self::Exited => "exited",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|status| status.as_str() == value)
    }

    pub(crate) fn is_live(&self) -> bool {
        matches!(self, Self::Active | Self::Paused)
    }
}

pub use aphel_sdk::sequences::EnrollmentSource;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum ExitReason {
    Replied,
    MeetingBooked,
    DealWon,
    DealLost,
    Bounced,
    Objection,
    ContactDeleted,
    Removed,
    ContactUnavailable,
    EnrollerUnavailable,
    SendFailed,
}

impl ExitReason {
    pub(crate) const ALL: [Self; 11] = [
        Self::Replied,
        Self::MeetingBooked,
        Self::DealWon,
        Self::DealLost,
        Self::Bounced,
        Self::Objection,
        Self::ContactDeleted,
        Self::Removed,
        Self::ContactUnavailable,
        Self::EnrollerUnavailable,
        Self::SendFailed,
    ];

    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Replied => "replied",
            Self::MeetingBooked => "meeting_booked",
            Self::DealWon => "deal_won",
            Self::DealLost => "deal_lost",
            Self::Bounced => "bounced",
            Self::Objection => "objection",
            Self::ContactDeleted => "contact_deleted",
            Self::Removed => "removed",
            Self::ContactUnavailable => "contact_unavailable",
            Self::EnrollerUnavailable => "enroller_unavailable",
            Self::SendFailed => "send_failed",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|reason| reason.as_str() == value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum RunStatus {
    Running,
    Sent,
    Created,
    Failed,
}

impl RunStatus {
    pub(crate) const ALL: [Self; 4] = [Self::Running, Self::Sent, Self::Created, Self::Failed];

    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Sent => "sent",
            Self::Created => "created",
            Self::Failed => "failed",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|status| status.as_str() == value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SendWindow {
    pub(crate) start_minute: u16,
    pub(crate) end_minute: u16,
}

impl Default for SendWindow {
    fn default() -> Self {
        Self {
            start_minute: DEFAULT_WINDOW_START,
            end_minute: DEFAULT_WINDOW_END,
        }
    }
}

impl SendWindow {
    pub(crate) fn new(start_minute: u16, end_minute: u16) -> Result<Self, &'static str> {
        if start_minute >= MINUTES_PER_DAY || end_minute > MINUTES_PER_DAY {
            return Err("must be a time of the day");
        }
        if start_minute >= end_minute {
            return Err("the window must end after it starts");
        }
        Ok(Self {
            start_minute,
            end_minute,
        })
    }

    fn start_time(&self) -> NaiveTime {
        minute_time(self.start_minute)
    }

    fn contains(&self, time: NaiveTime) -> bool {
        let minute = minute_of(time);
        minute >= self.start_minute && minute < self.end_minute
    }

    pub(crate) fn next_opening(&self, local: NaiveDateTime) -> NaiveDateTime {
        let date = local.date();
        if is_business_day(date) {
            if self.contains(local.time()) {
                return local;
            }
            if minute_of(local.time()) < self.start_minute {
                return date.and_time(self.start_time());
            }
        }
        next_business_day(date).and_time(self.start_time())
    }

    pub(crate) fn next_day_opening(&self, local: NaiveDateTime) -> NaiveDateTime {
        next_business_day(local.date()).and_time(self.start_time())
    }
}

fn minute_of(time: NaiveTime) -> u16 {
    let seconds = time.signed_duration_since(NaiveTime::MIN).num_seconds();
    u16::try_from(seconds / 60).unwrap_or(MINUTES_PER_DAY - 1)
}

fn minute_time(minute: u16) -> NaiveTime {
    NaiveTime::MIN + Duration::minutes(i64::from(minute.min(MINUTES_PER_DAY - 1)))
}

pub(crate) fn parse_clock(value: &str) -> Option<u16> {
    let (hours, minutes) = value.trim().split_once(':')?;
    let hours: u16 = hours.parse().ok()?;
    let minutes: u16 = minutes.parse().ok()?;
    if minutes >= 60 || hours > 24 || (hours == 24 && minutes > 0) {
        return None;
    }
    Some(hours * 60 + minutes)
}

pub(crate) fn format_clock(minute: u16) -> String {
    format!("{:02}:{:02}", minute / 60, minute % 60)
}

pub(crate) fn is_business_day(date: NaiveDate) -> bool {
    !matches!(date.weekday(), Weekday::Sat | Weekday::Sun)
}

pub(crate) fn next_business_day(date: NaiveDate) -> NaiveDate {
    let mut next = date + Duration::days(1);
    while !is_business_day(next) {
        next += Duration::days(1);
    }
    next
}

pub(crate) fn add_business_days(local: NaiveDateTime, days: u32) -> NaiveDateTime {
    let mut date = local.date();
    for _ in 0..days {
        date = next_business_day(date);
    }
    date.and_time(local.time())
}

pub(crate) fn normalize_sequence_name(value: &str) -> Result<String, &'static str> {
    let name = value.split_whitespace().collect::<Vec<_>>().join(" ");
    if name.is_empty() {
        return Err("must not be empty");
    }
    if name.chars().count() > 200 {
        return Err("must have at most 200 characters");
    }
    Ok(name)
}
