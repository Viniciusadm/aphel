use aphel_contract_crm::CrmRecordTypes;
use std::collections::HashSet;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use uuid::Uuid;

use aphel_kernel::application::query::Filter;
use aphel_kernel::application::shared::{DomainEvent, RecordRef, RecordType};
use aphel_kernel::domain::UserId;
use aphel_sdk::telephony::{CALL_LOGGED, CALL_UPDATED};
use booking::MEETING_BOOKED;
use email::{EMAIL_CLICKED, EMAIL_OPENED, EMAIL_RECEIVED};

pub(crate) const SCORING_MODEL_NOT_FOUND: &str = "scoring_model_not_found";
pub(crate) const SCORING_RECORD_NOT_FOUND: &str = "record_not_found";

pub(crate) const SCORING_MODEL_CREATED: &str = "scoring_model.created";
pub(crate) const SCORING_MODEL_UPDATED: &str = "scoring_model.updated";
pub(crate) const SCORING_MODEL_DELETED: &str = "scoring_model.deleted";
pub(crate) const SCORE_BAND_CHANGED: &str = "score.band_changed";
pub(crate) const COMPANY_SCORE_BAND_CHANGED: &str = "score.company_band_changed";

pub(crate) fn band_changed_event(record_type: &str) -> &'static str {
    if record_type == RecordType::COMPANY.as_str() {
        COMPANY_SCORE_BAND_CHANGED
    } else {
        SCORE_BAND_CHANGED
    }
}

pub(crate) const SCORING_MODEL_RECORD: RecordType =
    RecordType::from_static("scoring.scoring_model");

pub(crate) const SCORING_EVENTS_CONSUMER: &str = "scoring.events";
pub(crate) const SCORING_DECAY_JOB: &str = "scoring.decay";
pub(crate) const SCORING_DECAY_CRON: &str = "0 30 3 * * *";
pub(crate) const SCORING_RECALCULATE_JOB: &str = "scoring.recalculate";

pub(crate) const SCORE_FIELD: &str = "score";
pub(crate) const MAX_MODEL_NAME_LENGTH: usize = 120;
pub(crate) const MAX_MODEL_DESCRIPTION_LENGTH: usize = 500;
pub(crate) const MAX_CRITERION_LABEL_LENGTH: usize = 120;
pub(crate) const MAX_CRITERIA: usize = 50;
pub(crate) const MAX_MODELS: usize = 50;
pub(crate) const MAX_POINTS: i64 = 1000;
pub(crate) const MAX_CAP: i64 = 100_000;
pub(crate) const MAX_DECAY_DAYS: i64 = 3650;
pub(crate) const MAX_BAND_THRESHOLD: i64 = 1_000_000;
pub(crate) const DEFAULT_WARM_FROM: i32 = 30;
pub(crate) const DEFAULT_HOT_FROM: i32 = 70;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ScoreBand {
    #[default]
    Cold,
    Warm,
    Hot,
}

impl ScoreBand {
    pub(crate) const ALL: [Self; 3] = [Self::Cold, Self::Warm, Self::Hot];

    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Cold => "cold",
            Self::Warm => "warm",
            Self::Hot => "hot",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|band| band.as_str() == value)
    }

    pub(crate) fn rank(&self) -> u8 {
        match self {
            Self::Cold => 0,
            Self::Warm => 1,
            Self::Hot => 2,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ScoreBands {
    pub(crate) warm_from: i32,
    pub(crate) hot_from: i32,
}

impl Default for ScoreBands {
    fn default() -> Self {
        Self {
            warm_from: DEFAULT_WARM_FROM,
            hot_from: DEFAULT_HOT_FROM,
        }
    }
}

impl ScoreBands {
    pub(crate) fn band_for(&self, score: i32) -> ScoreBand {
        if score >= self.hot_from {
            ScoreBand::Hot
        } else if score >= self.warm_from {
            ScoreBand::Warm
        } else {
            ScoreBand::Cold
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum CriterionKind {
    Profile,
    Behavior,
}

impl CriterionKind {
    pub(crate) const ALL: [Self; 2] = [Self::Profile, Self::Behavior];

    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Profile => "profile",
            Self::Behavior => "behavior",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.as_str() == value)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum CriterionRule {
    Profile {
        filter: Filter,
    },
    Behavior {
        signal: BehaviorSignal,
        max_points: Option<i32>,
        decay_days: Option<i32>,
    },
}

impl CriterionRule {
    pub(crate) fn kind(&self) -> CriterionKind {
        match self {
            Self::Profile { .. } => CriterionKind::Profile,
            Self::Behavior { .. } => CriterionKind::Behavior,
        }
    }

    pub(crate) fn filter(&self) -> Option<&Filter> {
        match self {
            Self::Profile { filter } => Some(filter),
            Self::Behavior { .. } => None,
        }
    }

    pub(crate) fn signal(&self) -> Option<BehaviorSignal> {
        match self {
            Self::Behavior { signal, .. } => Some(*signal),
            Self::Profile { .. } => None,
        }
    }

    pub(crate) fn max_points(&self) -> Option<i32> {
        match self {
            Self::Behavior { max_points, .. } => *max_points,
            Self::Profile { .. } => None,
        }
    }

    pub(crate) fn decay_days(&self) -> Option<i32> {
        match self {
            Self::Behavior { decay_days, .. } => *decay_days,
            Self::Profile { .. } => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ScoringCriterion {
    pub(crate) id: Uuid,
    pub(crate) key: String,
    pub(crate) label: String,
    pub(crate) points: i32,
    pub(crate) rule: CriterionRule,
    pub(crate) position: i32,
}

impl ScoringCriterion {
    pub(crate) fn snapshot(&self) -> Value {
        json!({
            "key": self.key,
            "label": self.label,
            "kind": self.rule.kind().as_str(),
            "points": self.points,
            "filter": self.rule.filter().map(|filter| serde_json::to_value(filter).unwrap_or(Value::Null)),
            "signal": self.rule.signal().map(|signal| signal.as_str()),
            "max_points": self.rule.max_points(),
            "decay_days": self.rule.decay_days(),
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ScoringModel {
    pub(crate) id: Uuid,
    pub(crate) key: String,
    pub(crate) record_type: String,
    pub(crate) name: String,
    pub(crate) description: Option<String>,
    pub(crate) is_primary: bool,
    pub(crate) is_active: bool,
    pub(crate) bands: ScoreBands,
    pub(crate) criteria: Vec<ScoringCriterion>,
    pub(crate) recalculated_at: Option<DateTime<Utc>>,
    pub(crate) created_by: Option<UserId>,
    pub(crate) updated_by: Option<UserId>,
    pub(crate) created_at: DateTime<Utc>,
    pub(crate) updated_at: DateTime<Utc>,
}

impl ScoringModel {
    pub(crate) fn snapshot(&self) -> Value {
        json!({
            "key": self.key,
            "record_type": self.record_type,
            "name": self.name,
            "description": self.description,
            "is_primary": self.is_primary,
            "is_active": self.is_active,
            "warm_from": self.bands.warm_from,
            "hot_from": self.bands.hot_from,
            "criteria": self.criteria.iter().map(ScoringCriterion::snapshot).collect::<Vec<_>>(),
        })
    }

    pub(crate) fn summary(&self) -> Value {
        json!({
            "id": self.id.to_string(),
            "key": self.key,
            "name": self.name,
            "record_type": self.record_type,
        })
    }

    pub(crate) fn signals(&self) -> Vec<BehaviorSignal> {
        let mut signals: Vec<BehaviorSignal> = self
            .criteria
            .iter()
            .filter_map(|criterion| criterion.rule.signal())
            .collect();
        signals.sort_by_key(|signal| signal.as_str());
        signals.dedup();
        signals
    }

    pub(crate) fn profile_criteria(&self) -> impl Iterator<Item = (&ScoringCriterion, &Filter)> {
        self.criteria
            .iter()
            .filter_map(|criterion| criterion.rule.filter().map(|filter| (criterion, filter)))
    }

    pub(crate) fn longest_decay(&self) -> Option<i32> {
        let mut longest = Some(0);
        for criterion in &self.criteria {
            if let CriterionRule::Behavior { decay_days, .. } = &criterion.rule {
                match (decay_days, longest) {
                    (None, _) => longest = None,
                    (Some(days), Some(current)) => longest = Some(current.max(*days)),
                    (Some(_), None) => {}
                }
            }
        }
        longest
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ScoreContribution {
    pub(crate) criterion_id: Uuid,
    pub(crate) key: String,
    pub(crate) label: String,
    pub(crate) kind: CriterionKind,
    pub(crate) points: i32,
    #[serde(default)]
    pub(crate) occurrences: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RecordScore {
    pub(crate) model_id: Uuid,
    pub(crate) record_type: String,
    pub(crate) record_id: Uuid,
    pub(crate) score: i32,
    pub(crate) band: ScoreBand,
    pub(crate) explanation: Vec<ScoreContribution>,
    pub(crate) calculated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RecordScoreView {
    pub(crate) model: ScoringModel,
    pub(crate) score: i32,
    pub(crate) band: ScoreBand,
    pub(crate) explanation: Vec<ScoreContribution>,
    pub(crate) calculated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct CriterionInput {
    pub(crate) key: Option<String>,
    pub(crate) label: String,
    pub(crate) kind: String,
    pub(crate) filter: Option<Value>,
    pub(crate) signal: Option<String>,
    pub(crate) points: i64,
    pub(crate) max_points: Option<i64>,
    pub(crate) decay_days: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct CreateScoringModelCommand {
    pub(crate) record_type: String,
    pub(crate) key: Option<String>,
    pub(crate) name: String,
    pub(crate) description: Option<String>,
    pub(crate) is_primary: Option<bool>,
    pub(crate) is_active: Option<bool>,
    pub(crate) warm_from: Option<i64>,
    pub(crate) hot_from: Option<i64>,
    pub(crate) criteria: Vec<CriterionInput>,
    pub(crate) ip: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct UpdateScoringModelCommand {
    pub(crate) name: Option<String>,
    pub(crate) description: Option<Option<String>>,
    pub(crate) is_primary: Option<bool>,
    pub(crate) is_active: Option<bool>,
    pub(crate) warm_from: Option<i64>,
    pub(crate) hot_from: Option<i64>,
    pub(crate) criteria: Option<Vec<CriterionInput>>,
    pub(crate) expected_updated_at: Option<DateTime<Utc>>,
    pub(crate) ip: Option<String>,
}

pub(crate) const LEAD_FORM_SUBMITTED: &str = "lead_form.submitted";

pub(crate) const SIGNAL_EVENT_TYPES: [&str; 7] = [
    EMAIL_OPENED,
    EMAIL_CLICKED,
    EMAIL_RECEIVED,
    MEETING_BOOKED,
    LEAD_FORM_SUBMITTED,
    CALL_LOGGED,
    CALL_UPDATED,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum BehaviorSignal {
    EmailOpened,
    EmailClicked,
    EmailReplied,
    MeetingBooked,
    FormSubmitted,
    CallConnected,
}

impl BehaviorSignal {
    pub(crate) const ALL: [Self; 6] = [
        Self::EmailOpened,
        Self::EmailClicked,
        Self::EmailReplied,
        Self::MeetingBooked,
        Self::FormSubmitted,
        Self::CallConnected,
    ];

    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::EmailOpened => "email_opened",
            Self::EmailClicked => "email_clicked",
            Self::EmailReplied => "email_replied",
            Self::MeetingBooked => "meeting_booked",
            Self::FormSubmitted => "form_submitted",
            Self::CallConnected => "call_connected",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|signal| signal.as_str() == value)
    }

    pub(crate) fn values() -> String {
        Self::ALL
            .iter()
            .map(BehaviorSignal::as_str)
            .collect::<Vec<_>>()
            .join(", ")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SignalHit {
    pub(crate) signal: BehaviorSignal,
    pub(crate) source_key: String,
    pub(crate) records: Vec<RecordRef>,
    pub(crate) occurred_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SignalOccurrence {
    pub(crate) record_id: Uuid,
    pub(crate) signal: BehaviorSignal,
    pub(crate) occurred_at: DateTime<Utc>,
}

fn text<'a>(payload: &'a Value, key: &str) -> Option<&'a str> {
    payload.get(key).and_then(Value::as_str)
}

fn source(payload: &Value, key: &str, event: &DomainEvent) -> String {
    payload
        .get(key)
        .and_then(|value| match value {
            Value::String(text) => Some(text.clone()),
            Value::Null => None,
            other => Some(other.to_string()),
        })
        .filter(|value| !value.is_empty())
        .map(|value| format!("{key}:{value}"))
        .unwrap_or_else(|| format!("event:{}", event.id))
}

fn payload_record(value: &Value) -> Option<RecordRef> {
    let record_type = RecordType::parse(value.get("record_type")?.as_str()?).ok()?;
    let record_id = Uuid::parse_str(value.get("record_id")?.as_str()?).ok()?;
    Some(RecordRef::new(record_type, record_id))
}

pub(crate) fn event_records(event: &DomainEvent) -> Vec<RecordRef> {
    let mut records: Vec<RecordRef> = Vec::new();
    let from_payload = event
        .payload
        .get("records")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(payload_record);
    for record in event.record.iter().cloned().chain(from_payload) {
        if record.record_type != RecordType::ACTIVITY && !records.contains(&record) {
            records.push(record);
        }
    }
    records
}

pub(crate) fn signal_hit(event: &DomainEvent) -> Option<SignalHit> {
    let payload = &event.payload;
    let (signal, source_key) = match event.event_type.as_str() {
        EMAIL_OPENED => (
            BehaviorSignal::EmailOpened,
            source(payload, "email_id", event),
        ),
        EMAIL_CLICKED => (
            BehaviorSignal::EmailClicked,
            source(payload, "email_id", event),
        ),
        EMAIL_RECEIVED => {
            if text(payload, "direction") != Some("inbound")
                || text(payload, "kind").is_some_and(|kind| kind != "message")
            {
                return None;
            }
            (
                BehaviorSignal::EmailReplied,
                source(payload, "email_id", event),
            )
        }
        MEETING_BOOKED => (
            BehaviorSignal::MeetingBooked,
            source(payload, "booking_id", event),
        ),
        LEAD_FORM_SUBMITTED => (
            BehaviorSignal::FormSubmitted,
            source(payload, "submission_id", event),
        ),
        CALL_LOGGED | CALL_UPDATED => {
            if payload.get("is_connected").and_then(Value::as_bool) != Some(true) {
                return None;
            }
            (
                BehaviorSignal::CallConnected,
                source(payload, "activity_id", event),
            )
        }
        _ => return None,
    };
    let mut records = event_records(event);
    if event.event_type == MEETING_BOOKED || event.event_type == LEAD_FORM_SUBMITTED {
        let contact = text(payload, "contact_id")
            .and_then(|id| Uuid::parse_str(id).ok())
            .map(|id| RecordRef::new(RecordType::CONTACT, id));
        if let Some(contact) = contact
            && !records.contains(&contact)
        {
            records.push(contact);
        }
    }
    if records.is_empty() {
        return None;
    }
    Some(SignalHit {
        signal,
        source_key,
        records,
        occurred_at: event.occurred_at,
    })
}

const SECONDS_PER_DAY: f64 = 86_400.0;

pub(crate) fn decayed_points(
    points: i32,
    decay_days: Option<i32>,
    occurred_at: DateTime<Utc>,
    now: DateTime<Utc>,
) -> f64 {
    let points = f64::from(points);
    let Some(days) = decay_days.filter(|days| *days > 0) else {
        return points;
    };
    let age = ((now - occurred_at).num_seconds().max(0) as f64) / SECONDS_PER_DAY;
    let remaining = 1.0 - age / f64::from(days);
    if remaining <= 0.0 {
        0.0
    } else {
        points * remaining
    }
}

fn capped(total: f64, points: i32, max_points: Option<i32>) -> f64 {
    match max_points {
        Some(cap) if points >= 0 => total.min(f64::from(cap)),
        Some(cap) => total.max(-f64::from(cap)),
        None => total,
    }
}

fn rounded(value: f64) -> i32 {
    let value = value.round();
    if value >= f64::from(i32::MAX) {
        i32::MAX
    } else if value <= f64::from(i32::MIN) {
        i32::MIN
    } else {
        value as i32
    }
}

pub(crate) fn calculate_score(
    model: &ScoringModel,
    record_id: Uuid,
    matched_profiles: &HashSet<Uuid>,
    occurrences: &[SignalOccurrence],
    now: DateTime<Utc>,
) -> RecordScore {
    let mut explanation = Vec::new();
    let mut total: i64 = 0;
    for criterion in &model.criteria {
        let (points, count) = match &criterion.rule {
            CriterionRule::Profile { .. } => {
                if !matched_profiles.contains(&criterion.id) {
                    continue;
                }
                (criterion.points, 1)
            }
            CriterionRule::Behavior {
                signal,
                max_points,
                decay_days,
            } => {
                let mut sum = 0.0;
                let mut count = 0u32;
                for occurrence in occurrences
                    .iter()
                    .filter(|occurrence| occurrence.signal == *signal)
                {
                    let value =
                        decayed_points(criterion.points, *decay_days, occurrence.occurred_at, now);
                    if value != 0.0 {
                        sum += value;
                        count += 1;
                    }
                }
                if count == 0 {
                    continue;
                }
                (rounded(capped(sum, criterion.points, *max_points)), count)
            }
        };
        if points == 0 {
            continue;
        }
        total += i64::from(points);
        explanation.push(ScoreContribution {
            criterion_id: criterion.id,
            key: criterion.key.clone(),
            label: criterion.label.clone(),
            kind: criterion.rule.kind(),
            points,
            occurrences: count,
        });
    }
    let score =
        i32::try_from(total.clamp(i64::from(i32::MIN), i64::from(i32::MAX))).unwrap_or_default();
    RecordScore {
        model_id: model.id,
        record_type: model.record_type.clone(),
        record_id,
        score,
        band: model.bands.band_for(score),
        explanation,
        calculated_at: now,
    }
}
