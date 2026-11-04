use aphel_contract_crm::CrmRecordTypes;
use std::collections::{BTreeMap, BTreeSet};

use chrono::{DateTime, Duration, NaiveDate, Utc};
use uuid::Uuid;

use aphel_kernel::application::shared::RecordType;

pub use aphel_sdk::leads::{LEAD_STAGE, LIFECYCLE_STAGE_FIELD};
pub(crate) const QUALIFIED_STAGE: &str = "qualified";
pub(crate) const OPPORTUNITY_STAGE: &str = "opportunity";
pub(crate) const CUSTOMER_STAGE: &str = "customer";
pub(crate) const FORMER_CUSTOMER_STAGE: &str = "former_customer";

pub(crate) const MAX_DISQUALIFICATION_REASONS: usize = 50;
pub(crate) const MAX_DISQUALIFICATION_REASON_NAME_LENGTH: usize = 140;
pub(crate) const MAX_DISQUALIFICATION_NOTE_LENGTH: usize = 2000;
pub(crate) const MAX_STAGE_LENGTH: usize = 64;

pub(crate) fn is_lifecycle_record_type(record_type: &RecordType) -> bool {
    *record_type == RecordType::CONTACT || *record_type == RecordType::COMPANY
}

pub(crate) fn parse_lifecycle_record_type(value: &str) -> Option<RecordType> {
    [RecordType::CONTACT, RecordType::COMPANY]
        .into_iter()
        .find(|record_type| record_type.as_str() == value)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) enum TransitionTrigger {
    DealCreated,
    DealWon,
    AllDealsLost,
}

impl TransitionTrigger {
    pub(crate) const ALL: [Self; 3] = [Self::DealCreated, Self::DealWon, Self::AllDealsLost];

    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::DealCreated => "deal_created",
            Self::DealWon => "deal_won",
            Self::AllDealsLost => "all_deals_lost",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|trigger| trigger.as_str() == value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TransitionRule {
    pub(crate) trigger: TransitionTrigger,
    pub(crate) enabled: bool,
    pub(crate) target_stage: String,
    pub(crate) from_stages: Vec<String>,
    pub(crate) from_empty: bool,
}

impl TransitionRule {
    pub(crate) fn default_for(trigger: TransitionTrigger) -> Self {
        let stages = |values: &[&str]| values.iter().map(|value| (*value).to_owned()).collect();
        match trigger {
            TransitionTrigger::DealCreated => Self {
                trigger,
                enabled: true,
                target_stage: OPPORTUNITY_STAGE.to_owned(),
                from_stages: stages(&[LEAD_STAGE, QUALIFIED_STAGE, FORMER_CUSTOMER_STAGE]),
                from_empty: true,
            },
            TransitionTrigger::DealWon => Self {
                trigger,
                enabled: true,
                target_stage: CUSTOMER_STAGE.to_owned(),
                from_stages: stages(&[
                    LEAD_STAGE,
                    QUALIFIED_STAGE,
                    OPPORTUNITY_STAGE,
                    FORMER_CUSTOMER_STAGE,
                ]),
                from_empty: true,
            },
            TransitionTrigger::AllDealsLost => Self {
                trigger,
                enabled: true,
                target_stage: QUALIFIED_STAGE.to_owned(),
                from_stages: stages(&[OPPORTUNITY_STAGE]),
                from_empty: false,
            },
        }
    }

    pub(crate) fn applies_from(&self, current: Option<&str>) -> bool {
        match current {
            None => self.from_empty,
            Some(stage) if stage == self.target_stage => false,
            Some(stage) => self.from_stages.iter().any(|from| from == stage),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LifecycleSettings {
    pub(crate) rules: Vec<TransitionRule>,
    pub(crate) disqualified_stage: Option<String>,
    pub(crate) updated_at: Option<DateTime<Utc>>,
}

impl Default for LifecycleSettings {
    fn default() -> Self {
        Self {
            rules: TransitionTrigger::ALL
                .into_iter()
                .map(TransitionRule::default_for)
                .collect(),
            disqualified_stage: None,
            updated_at: None,
        }
    }
}

impl LifecycleSettings {
    pub(crate) fn rule(&self, trigger: TransitionTrigger) -> TransitionRule {
        self.rules
            .iter()
            .find(|rule| rule.trigger == trigger)
            .cloned()
            .unwrap_or_else(|| TransitionRule::default_for(trigger))
    }

    pub(crate) fn normalized(mut self) -> Self {
        self.rules = TransitionTrigger::ALL
            .into_iter()
            .map(|trigger| self.rule(trigger))
            .collect();
        self
    }

    pub(crate) fn converted_stage(&self) -> String {
        self.rule(TransitionTrigger::DealCreated).target_stage
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DefaultDisqualificationReason {
    pub(crate) key: &'static str,
    pub(crate) name: &'static str,
}

pub(crate) const DEFAULT_DISQUALIFICATION_REASONS: [DefaultDisqualificationReason; 6] = [
    DefaultDisqualificationReason {
        key: "no_budget",
        name: "Sem orçamento",
    },
    DefaultDisqualificationReason {
        key: "no_fit",
        name: "Fora do perfil",
    },
    DefaultDisqualificationReason {
        key: "unresponsive",
        name: "Sem resposta",
    },
    DefaultDisqualificationReason {
        key: "competitor",
        name: "Escolheu um concorrente",
    },
    DefaultDisqualificationReason {
        key: "invalid_contact",
        name: "Dados inválidos",
    },
    DefaultDisqualificationReason {
        key: "other",
        name: "Outro",
    },
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DisqualificationReason {
    pub(crate) id: Uuid,
    pub(crate) key: String,
    pub(crate) name: String,
    pub(crate) is_active: bool,
    pub(crate) position: i32,
    pub(crate) created_at: DateTime<Utc>,
    pub(crate) updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LeadDisqualification {
    pub(crate) id: Uuid,
    pub(crate) contact_id: Uuid,
    pub(crate) reason_id: Option<Uuid>,
    pub(crate) reason_name: String,
    pub(crate) note: Option<String>,
    pub(crate) from_stage: Option<String>,
    pub(crate) actor_type: String,
    pub(crate) actor_id: Option<String>,
    pub(crate) at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StageChange {
    pub(crate) record_type: RecordType,
    pub(crate) record_id: Uuid,
    pub(crate) from_stage: Option<String>,
    pub(crate) to_stage: Option<String>,
    pub(crate) actor_type: String,
    pub(crate) actor_id: Option<String>,
    pub(crate) at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StageHistoryEntry {
    pub(crate) id: Uuid,
    pub(crate) from_stage: Option<String>,
    pub(crate) to_stage: String,
    pub(crate) entered_at: DateTime<Utc>,
    pub(crate) left_at: Option<DateTime<Utc>>,
    pub(crate) actor_type: String,
    pub(crate) actor_id: Option<String>,
    pub(crate) actor_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageOption {
    pub value: String,
    pub(crate) label: String,
    pub(crate) color: Option<String>,
    pub(crate) position: i32,
    pub is_active: bool,
}

pub(crate) use aphel_sdk::deals::{DealOutcomes, DealParties};

pub(crate) const DEFAULT_REPORT_DAYS: i64 = 90;
pub(crate) const MAX_REPORT_DAYS: i64 = 731;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct HistorySpan {
    pub(crate) record_id: Uuid,
    pub(crate) to_stage: String,
    pub(crate) entered_at: DateTime<Utc>,
    pub(crate) left_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DisqualificationCount {
    pub(crate) reason_id: Option<Uuid>,
    pub(crate) reason_name: String,
    pub(crate) count: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct StageReport {
    pub(crate) stage: String,
    pub(crate) label: String,
    pub(crate) color: Option<String>,
    pub(crate) is_active: bool,
    pub(crate) current: u64,
    pub(crate) entered: u64,
    pub(crate) next_stage: Option<String>,
    pub(crate) converted: u64,
    pub(crate) conversion_rate: Option<f64>,
    pub(crate) completed: u64,
    pub(crate) average_days: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct LifecycleReport {
    pub(crate) record_type: RecordType,
    pub(crate) from: NaiveDate,
    pub(crate) to: NaiveDate,
    pub(crate) stages: Vec<StageReport>,
    pub(crate) disqualifications: Vec<DisqualificationCount>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ReportWindow {
    pub(crate) from: NaiveDate,
    pub(crate) to: NaiveDate,
}

impl ReportWindow {
    pub(crate) fn start(&self) -> DateTime<Utc> {
        self.from.and_hms_opt(0, 0, 0).unwrap_or_default().and_utc()
    }

    pub(crate) fn end(&self) -> DateTime<Utc> {
        self.to.and_hms_opt(0, 0, 0).unwrap_or_default().and_utc() + Duration::days(1)
    }

    fn contains(&self, at: DateTime<Utc>) -> bool {
        at >= self.start() && at < self.end()
    }
}

pub(crate) struct ReportInput<'a> {
    pub(crate) options: &'a [StageOption],
    pub(crate) window: &'a ReportWindow,
    pub(crate) spans: &'a [HistorySpan],
    pub(crate) current: &'a [(String, u64)],
}

fn ordered_stages(input: &ReportInput<'_>) -> Vec<StageOption> {
    let mut stages: Vec<StageOption> = input
        .options
        .iter()
        .filter(|option| option.is_active)
        .cloned()
        .collect();
    stages.sort_by_key(|option| option.position);
    let mut known: BTreeSet<String> = stages.iter().map(|option| option.value.clone()).collect();
    let extra = input
        .spans
        .iter()
        .map(|span| span.to_stage.as_str())
        .chain(input.current.iter().map(|(stage, _)| stage.as_str()));
    let mut next_position = stages.last().map_or(0, |option| option.position + 1);
    for value in extra {
        if known.insert(value.to_owned()) {
            let option = input.options.iter().find(|option| option.value == value);
            stages.push(StageOption {
                value: value.to_owned(),
                label: option.map_or_else(|| value.to_owned(), |option| option.label.clone()),
                color: option.and_then(|option| option.color.clone()),
                position: next_position,
                is_active: false,
            });
            next_position += 1;
        }
    }
    stages
}

pub(crate) fn build_stage_reports(input: &ReportInput<'_>) -> Vec<StageReport> {
    let stages = ordered_stages(input);
    let rank: BTreeMap<&str, usize> = stages
        .iter()
        .enumerate()
        .map(|(index, option)| (option.value.as_str(), index))
        .collect();
    let mut by_record: BTreeMap<Uuid, Vec<&HistorySpan>> = BTreeMap::new();
    for span in input.spans {
        by_record.entry(span.record_id).or_default().push(span);
    }
    for spans in by_record.values_mut() {
        spans.sort_by_key(|span| span.entered_at);
    }
    stages
        .iter()
        .enumerate()
        .map(|(index, option)| {
            let mut entered = BTreeSet::new();
            let mut converted = BTreeSet::new();
            let mut completed = 0u64;
            let mut total_days = 0f64;
            for (record_id, spans) in &by_record {
                for (position, span) in spans.iter().enumerate() {
                    if span.to_stage != option.value || !input.window.contains(span.entered_at) {
                        continue;
                    }
                    entered.insert(*record_id);
                    if spans[position + 1..].iter().any(|later| {
                        rank.get(later.to_stage.as_str())
                            .is_some_and(|later_rank| *later_rank > index)
                    }) {
                        converted.insert(*record_id);
                    }
                    if let Some(left_at) = span.left_at {
                        completed += 1;
                        total_days +=
                            (left_at - span.entered_at).num_seconds().max(0) as f64 / 86_400.0;
                    }
                }
            }
            let entered_count = entered.len() as u64;
            let converted_count = converted.len() as u64;
            StageReport {
                stage: option.value.clone(),
                label: option.label.clone(),
                color: option.color.clone(),
                is_active: option.is_active,
                current: input
                    .current
                    .iter()
                    .find(|(stage, _)| *stage == option.value)
                    .map_or(0, |(_, count)| *count),
                entered: entered_count,
                next_stage: stages.get(index + 1).map(|next| next.value.clone()),
                converted: converted_count,
                conversion_rate: (entered_count > 0 && index + 1 < stages.len())
                    .then(|| converted_count as f64 / entered_count as f64),
                completed,
                average_days: (completed > 0).then(|| total_days / completed as f64),
            }
        })
        .collect()
}
