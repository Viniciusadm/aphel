use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use uuid::Uuid;

use aphel_kernel::domain::{TeamId, UserId};
use aphel_sdk::platform::application::organization_admin::ListWindow;
use aphel_sdk::platform::application::shared::RecordType;

pub(crate) const GOAL_NOT_FOUND: &str = "goal_not_found";
pub(crate) const GOAL_ALREADY_EXISTS: &str = "goal_already_exists";
pub(crate) const GOAL_CREATED: &str = "goal.created";
pub(crate) const GOAL_UPDATED: &str = "goal.updated";
pub(crate) const GOAL_DELETED: &str = "goal.deleted";
pub(crate) const GOAL_RECORD_TYPE: RecordType = RecordType::from_static("goals.goal");
pub(crate) const MAX_GOAL_TARGET: &str = "9999999999999999.99";
pub(crate) const DATE_FORMAT: &str = "%Y-%m-%d";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum GoalMetric {
    WonAmount,
    WonCount,
    ActivitiesDone,
    NewContacts,
}

impl GoalMetric {
    pub(crate) const ALL: [Self; 4] = [
        Self::WonAmount,
        Self::WonCount,
        Self::ActivitiesDone,
        Self::NewContacts,
    ];

    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::WonAmount => "won_amount",
            Self::WonCount => "won_count",
            Self::ActivitiesDone => "activities_done",
            Self::NewContacts => "new_contacts",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|metric| metric.as_str() == value)
    }

    pub(crate) fn module(&self) -> &'static str {
        match self {
            Self::WonAmount | Self::WonCount => "deals",
            Self::ActivitiesDone => "tasks",
            Self::NewContacts => "people",
        }
    }

    pub(crate) fn is_count(&self) -> bool {
        !matches!(self, Self::WonAmount)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum GoalPeriod {
    Month,
    Quarter,
    Year,
}

impl GoalPeriod {
    pub(crate) const ALL: [Self; 3] = [Self::Month, Self::Quarter, Self::Year];

    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Month => "month",
            Self::Quarter => "quarter",
            Self::Year => "year",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|period| period.as_str() == value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum GoalCreditMode {
    Owner,
    Split,
}

impl GoalCreditMode {
    pub(crate) const ALL: [Self; 2] = [Self::Owner, Self::Split];

    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Owner => "owner",
            Self::Split => "split",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|mode| mode.as_str() == value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum GoalOwnerKind {
    Organization,
    User,
    Team,
}

impl GoalOwnerKind {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Organization => "organization",
            Self::User => "user",
            Self::Team => "team",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum GoalOwner {
    Organization,
    User(UserId),
    Team(TeamId),
}

impl GoalOwner {
    pub(crate) fn kind(&self) -> GoalOwnerKind {
        match self {
            Self::Organization => GoalOwnerKind::Organization,
            Self::User(_) => GoalOwnerKind::User,
            Self::Team(_) => GoalOwnerKind::Team,
        }
    }

    pub(crate) fn user_id(&self) -> Option<&UserId> {
        match self {
            Self::User(user_id) => Some(user_id),
            _ => None,
        }
    }

    pub(crate) fn team_id(&self) -> Option<&TeamId> {
        match self {
            Self::Team(team_id) => Some(team_id),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Goal {
    pub(crate) id: Uuid,
    pub(crate) owner: GoalOwner,
    pub(crate) owner_name: Option<String>,
    pub(crate) metric: GoalMetric,
    pub(crate) period: GoalPeriod,
    pub(crate) starts_on: NaiveDate,
    pub(crate) ends_on: NaiveDate,
    pub(crate) target: Decimal,
    pub(crate) credit_mode: GoalCreditMode,
    pub(crate) created_by: Option<UserId>,
    pub(crate) created_at: DateTime<Utc>,
    pub(crate) updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum GoalVisibility {
    All,
    Limited {
        user_ids: Vec<UserId>,
        team_ids: Vec<TeamId>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GoalListFilter {
    pub(crate) visibility: GoalVisibility,
    pub(crate) owner_type: Option<&'static str>,
    pub(crate) user_id: Option<UserId>,
    pub(crate) team_id: Option<TeamId>,
    pub(crate) metric: Option<GoalMetric>,
    pub(crate) period: Option<GoalPeriod>,
    pub(crate) active_on: Option<NaiveDate>,
    pub(crate) window: ListWindow,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GoalOwnerOption {
    pub(crate) id: String,
    pub(crate) name: String,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct GoalOwnerInput {
    pub(crate) owner_type: GoalOwnerKind,
    pub(crate) user_id: Option<String>,
    pub(crate) team_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CreateGoalCommand {
    pub(crate) owner: GoalOwnerInput,
    pub(crate) metric: GoalMetric,
    pub(crate) period: GoalPeriod,
    pub(crate) starts_on: String,
    pub(crate) target: String,
    pub(crate) credit_mode: Option<GoalCreditMode>,
    pub(crate) ip: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct UpdateGoalCommand {
    pub(crate) owner: Option<GoalOwnerInput>,
    pub(crate) metric: Option<GoalMetric>,
    pub(crate) period: Option<GoalPeriod>,
    pub(crate) starts_on: Option<String>,
    pub(crate) target: Option<String>,
    pub(crate) credit_mode: Option<GoalCreditMode>,
    pub(crate) ip: Option<String>,
    pub(crate) expected_updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct GoalListQuery {
    pub(crate) owner_type: Option<GoalOwnerKind>,
    pub(crate) user_id: Option<String>,
    pub(crate) team_id: Option<String>,
    pub(crate) metric: Option<GoalMetric>,
    pub(crate) period: Option<GoalPeriod>,
    pub(crate) active_on: Option<String>,
    pub(crate) current: bool,
    pub(crate) with_progress: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GoalStatus {
    Upcoming,
    OnTrack,
    Behind,
    Achieved,
    Missed,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct GoalProgress {
    pub(crate) actual: Decimal,
    pub(crate) ratio: f64,
    pub(crate) expected_ratio: f64,
    pub(crate) status: GoalStatus,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct GoalView {
    pub(crate) goal: Goal,
    pub(crate) currency: Option<String>,
    pub(crate) can_manage: bool,
    pub(crate) progress: Option<GoalProgress>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GoalOwnerOptions {
    pub(crate) organization: bool,
    pub(crate) users: Vec<GoalOwnerOption>,
    pub(crate) teams: Vec<GoalOwnerOption>,
}

pub(crate) fn elapsed_ratio(goal: &Goal, today: NaiveDate) -> f64 {
    if today < goal.starts_on {
        return 0.0;
    }
    if today > goal.ends_on {
        return 1.0;
    }
    let total = (goal.ends_on - goal.starts_on).num_days() + 1;
    let elapsed = (today - goal.starts_on).num_days() + 1;
    if total <= 0 {
        1.0
    } else {
        elapsed as f64 / total as f64
    }
}

fn rounded(value: f64) -> f64 {
    (value * 10_000.0).round() / 10_000.0
}

pub(crate) fn progress_of(goal: &Goal, actual: Decimal, today: NaiveDate) -> GoalProgress {
    let ratio = if goal.target.is_zero() {
        1.0
    } else {
        (actual / goal.target).to_f64().unwrap_or_default()
    };
    let expected_ratio = elapsed_ratio(goal, today);
    let status = if actual >= goal.target {
        GoalStatus::Achieved
    } else if today < goal.starts_on {
        GoalStatus::Upcoming
    } else if today > goal.ends_on {
        GoalStatus::Missed
    } else if ratio >= expected_ratio {
        GoalStatus::OnTrack
    } else {
        GoalStatus::Behind
    };
    GoalProgress {
        actual,
        ratio: rounded(ratio),
        expected_ratio: rounded(expected_ratio),
        status,
    }
}
