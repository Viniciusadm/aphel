use std::cmp::Reverse;

use chrono::{DateTime, Utc};
use uuid::Uuid;

use aphel_kernel::domain::UserId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum OnboardingStatus {
    #[default]
    Pending,
    Completed,
    Skipped,
    Dismissed,
}

impl OnboardingStatus {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Completed => "completed",
            Self::Skipped => "skipped",
            Self::Dismissed => "dismissed",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "pending" => Some(Self::Pending),
            "completed" => Some(Self::Completed),
            "skipped" => Some(Self::Skipped),
            "dismissed" => Some(Self::Dismissed),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BusinessType {
    Freelancer,
    Agency,
    RealEstate,
    Consultancy,
    Saas,
    Sales,
    Other,
}

impl BusinessType {
    pub(crate) const ALL: [Self; 7] = [
        Self::Freelancer,
        Self::Agency,
        Self::RealEstate,
        Self::Consultancy,
        Self::Saas,
        Self::Sales,
        Self::Other,
    ];

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Freelancer => "freelancer",
            Self::Agency => "agency",
            Self::RealEstate => "real_estate",
            Self::Consultancy => "consultancy",
            Self::Saas => "saas",
            Self::Sales => "sales",
            Self::Other => "other",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|kind| kind.as_str() == value)
            .or(match value {
                "software" => Some(Self::Saas),
                "consulting" => Some(Self::Consultancy),
                "b2b_sales" => Some(Self::Sales),
                _ => None,
            })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Offering {
    Product,
    Service,
    Recurring,
}

impl Offering {
    pub(crate) const ALL: [Self; 3] = [Self::Product, Self::Service, Self::Recurring];

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Product => "product",
            Self::Service => "service",
            Self::Recurring => "recurring",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|offering| offering.as_str() == value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TeamSize {
    Solo,
    Small,
    Medium,
    Large,
}

impl TeamSize {
    pub(crate) const ALL: [Self; 4] = [Self::Solo, Self::Small, Self::Medium, Self::Large];

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Solo => "solo",
            Self::Small => "small",
            Self::Medium => "medium",
            Self::Large => "large",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|size| size.as_str() == value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct OnboardingAnswers {
    pub(crate) business_type: Option<BusinessType>,
    pub(crate) offering: Vec<Offering>,
    pub(crate) team_size: Option<TeamSize>,
}

impl OnboardingAnswers {
    pub(crate) fn is_empty(&self) -> bool {
        self.business_type.is_none() && self.offering.is_empty() && self.team_size.is_none()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct OnboardingState {
    pub(crate) status: OnboardingStatus,
    pub(crate) answers: OnboardingAnswers,
    pub(crate) installer_answers: OnboardingAnswers,
    pub(crate) template_key: Option<String>,
    pub(crate) install_id: Option<Uuid>,
    pub(crate) completed_by: Option<UserId>,
    pub(crate) completed_at: Option<DateTime<Utc>>,
    pub(crate) skipped_by: Option<UserId>,
    pub(crate) skipped_at: Option<DateTime<Utc>>,
    pub(crate) updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct SaveAnswersCommand {
    pub(crate) business_type: Option<BusinessType>,
    pub(crate) offering: Vec<Offering>,
    pub(crate) team_size: Option<TeamSize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CompleteOnboardingCommand {
    pub template_key: Option<String>,
    pub install_id: Option<String>,
    pub ip: Option<String>,
}

pub(crate) const B2B_TEMPLATE: &str = "sales";
pub(crate) const AGENCY_TEMPLATE: &str = "agency";
pub(crate) const SAAS_TEMPLATE: &str = "saas";
pub(crate) const REAL_ESTATE_TEMPLATE: &str = "real_estate";
pub(crate) const FREELANCER_TEMPLATE: &str = "freelancer";

pub(crate) const RECOMMENDABLE_TEMPLATES: [&str; 5] = [
    B2B_TEMPLATE,
    AGENCY_TEMPLATE,
    SAAS_TEMPLATE,
    REAL_ESTATE_TEMPLATE,
    FREELANCER_TEMPLATE,
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TemplateSuggestion {
    pub(crate) key: &'static str,
    pub(crate) reasons: Vec<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Recommendation {
    pub(crate) recommended: TemplateSuggestion,
    pub(crate) alternatives: Vec<TemplateSuggestion>,
}

struct Signal {
    reason: &'static str,
    template: &'static str,
    weight: u32,
}

const fn signal(reason: &'static str, template: &'static str, weight: u32) -> Signal {
    Signal {
        reason,
        template,
        weight,
    }
}

fn business_signals(business_type: BusinessType) -> Vec<Signal> {
    match business_type {
        BusinessType::Freelancer => vec![signal("business_freelancer", FREELANCER_TEMPLATE, 10)],
        BusinessType::Agency => vec![signal("business_agency", AGENCY_TEMPLATE, 10)],
        BusinessType::RealEstate => vec![signal("business_real_estate", REAL_ESTATE_TEMPLATE, 10)],
        BusinessType::Saas => vec![signal("business_saas", SAAS_TEMPLATE, 10)],
        BusinessType::Sales => vec![signal("business_sales", B2B_TEMPLATE, 10)],
        BusinessType::Consultancy => vec![
            signal("business_consultancy", AGENCY_TEMPLATE, 6),
            signal("business_consultancy", FREELANCER_TEMPLATE, 4),
        ],
        BusinessType::Other => vec![signal("business_other", B2B_TEMPLATE, 2)],
    }
}

fn offering_signals(offering: Offering) -> Vec<Signal> {
    match offering {
        Offering::Product => vec![signal("offering_product", B2B_TEMPLATE, 3)],
        Offering::Service => vec![
            signal("offering_service", AGENCY_TEMPLATE, 2),
            signal("offering_service", FREELANCER_TEMPLATE, 2),
        ],
        Offering::Recurring => vec![signal("offering_recurring", SAAS_TEMPLATE, 3)],
    }
}

fn team_signals(team_size: TeamSize) -> Vec<Signal> {
    match team_size {
        TeamSize::Solo => vec![signal("team_solo", FREELANCER_TEMPLATE, 6)],
        TeamSize::Small => vec![signal("team_small", AGENCY_TEMPLATE, 1)],
        TeamSize::Medium => vec![signal("team_medium", B2B_TEMPLATE, 1)],
        TeamSize::Large => vec![signal("team_large", B2B_TEMPLATE, 2)],
    }
}

fn signals(answers: &OnboardingAnswers) -> Vec<Signal> {
    let mut signals = Vec::new();
    if let Some(business_type) = answers.business_type {
        signals.extend(business_signals(business_type));
    }
    for offering in &answers.offering {
        signals.extend(offering_signals(*offering));
    }
    if let Some(team_size) = answers.team_size {
        signals.extend(team_signals(team_size));
    }
    signals
}

pub(crate) fn recommend(answers: &OnboardingAnswers) -> Option<Recommendation> {
    answers.business_type?;
    let signals = signals(answers);
    let mut ranked: Vec<(u32, usize, TemplateSuggestion)> = RECOMMENDABLE_TEMPLATES
        .iter()
        .enumerate()
        .map(|(precedence, key)| {
            let mut matching: Vec<&Signal> = signals
                .iter()
                .filter(|signal| signal.template == *key)
                .collect();
            matching.sort_by_key(|signal| Reverse(signal.weight));
            let score = matching.iter().map(|signal| signal.weight).sum();
            let suggestion = TemplateSuggestion {
                key,
                reasons: matching.iter().map(|signal| signal.reason).collect(),
            };
            (score, precedence, suggestion)
        })
        .collect();
    ranked.sort_by_key(|(score, precedence, _)| (Reverse(*score), *precedence));
    let mut suggestions = ranked.into_iter().map(|(_, _, suggestion)| suggestion);
    let recommended = suggestions.next()?;
    Some(Recommendation {
        recommended,
        alternatives: suggestions.collect(),
    })
}
