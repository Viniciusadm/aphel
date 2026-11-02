#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AutomationStatus {
    Draft,
    Active,
    Paused,
}

impl AutomationStatus {
    pub const ALL: [Self; 3] = [Self::Draft, Self::Active, Self::Paused];

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Active => "active",
            Self::Paused => "paused",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|status| status.as_str() == value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RunStatus {
    Running,
    Waiting,
    Succeeded,
    Failed,
    Skipped,
    Cancelled,
}

impl RunStatus {
    pub const ALL: [Self; 6] = [
        Self::Running,
        Self::Waiting,
        Self::Succeeded,
        Self::Failed,
        Self::Skipped,
        Self::Cancelled,
    ];

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Waiting => "waiting",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Skipped => "skipped",
            Self::Cancelled => "cancelled",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|status| status.as_str() == value)
    }

    pub fn is_finished(&self) -> bool {
        matches!(
            self,
            Self::Succeeded | Self::Failed | Self::Skipped | Self::Cancelled
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StepStatus {
    Succeeded,
    Failed,
    Skipped,
    Planned,
    Waiting,
}

impl StepStatus {
    pub const ALL: [Self; 5] = [
        Self::Succeeded,
        Self::Failed,
        Self::Skipped,
        Self::Planned,
        Self::Waiting,
    ];

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Skipped => "skipped",
            Self::Planned => "planned",
            Self::Waiting => "waiting",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|status| status.as_str() == value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StepKind {
    Condition,
    Branch,
    Action,
    Wait,
    WaitUntil,
}

impl StepKind {
    pub const ALL: [Self; 5] = [
        Self::Condition,
        Self::Branch,
        Self::Action,
        Self::Wait,
        Self::WaitUntil,
    ];

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Condition => "condition",
            Self::Branch => "branch",
            Self::Action => "action",
            Self::Wait => "wait",
            Self::WaitUntil => "wait_until",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.as_str() == value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SkipReason {
    MaxDepth,
    CausationCycle,
    RateLimited,
    RecordNotFound,
    RecordNotAccessible,
    PublisherUnavailable,
    AutomationInactive,
    RecordDeleted,
    AutomationPaused,
    VersionPublished,
}

impl SkipReason {
    pub const ALL: [Self; 10] = [
        Self::MaxDepth,
        Self::CausationCycle,
        Self::RateLimited,
        Self::RecordNotFound,
        Self::RecordNotAccessible,
        Self::PublisherUnavailable,
        Self::AutomationInactive,
        Self::RecordDeleted,
        Self::AutomationPaused,
        Self::VersionPublished,
    ];

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::MaxDepth => "max_depth",
            Self::CausationCycle => "causation_cycle",
            Self::RateLimited => "rate_limited",
            Self::RecordNotFound => "record_not_found",
            Self::RecordNotAccessible => "record_not_accessible",
            Self::PublisherUnavailable => "publisher_unavailable",
            Self::AutomationInactive => "automation_inactive",
            Self::RecordDeleted => "record_deleted",
            Self::AutomationPaused => "automation_paused",
            Self::VersionPublished => "version_published",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|reason| reason.as_str() == value)
    }

    pub fn is_loop_protection(&self) -> bool {
        matches!(
            self,
            Self::MaxDepth | Self::CausationCycle | Self::RateLimited
        )
    }
}
