use aphel_kernel::application::fields::EntityDefinition;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Dataset {
    pub(crate) key: &'static str,
    pub(crate) table: &'static str,
    pub(crate) folder: &'static str,
    pub(crate) before_records: bool,
    pub(crate) exclude: Option<(&'static str, &'static str)>,
    pub(crate) upsert: bool,
    pub(crate) warning: Option<&'static str>,
}

impl Dataset {
    const fn new(key: &'static str, folder: &'static str, before_records: bool) -> Self {
        Self {
            key,
            table: key,
            folder,
            before_records,
            exclude: None,
            upsert: false,
            warning: None,
        }
    }

    const fn excluding(mut self, column: &'static str, value: &'static str) -> Self {
        self.exclude = Some((column, value));
        self
    }

    const fn upsert(mut self) -> Self {
        self.upsert = true;
        self
    }

    const fn warning(mut self, message: &'static str) -> Self {
        self.warning = Some(message);
        self
    }

    pub(crate) fn path(&self) -> String {
        format!("{}/{}.jsonl", self.folder, self.key)
    }

    pub(crate) fn excludes(&self, row: &serde_json::Map<String, serde_json::Value>) -> bool {
        self.exclude.is_some_and(|(column, value)| {
            row.get(column).and_then(serde_json::Value::as_str) == Some(value)
        })
    }
}

pub(crate) const DOMAIN_EVENTS_DATASET: &str = "domain_events";
pub(crate) const RECORD_SHARES_DATASET: &str = "record_shares";
pub(crate) const WEBHOOK_ENDPOINTS_DATASET: &str = "webhook_endpoints";
pub(crate) const WEBHOOK_DISABLED_REASON: &str = "manual";
pub(crate) const WEBHOOK_SECRET_WARNING: &str = "webhook_endpoints: signing secrets are not part of the backup; restored endpoints stay disabled until a new secret is generated";

pub(crate) const DATASETS: &[Dataset] = &[
    Dataset::new("teams", "data", true),
    Dataset::new("team_memberships", "data", true),
    Dataset::new("workflow_teams", "data", true),
    Dataset::new("goals", "data", true),
    Dataset::new("segments", "data", true),
    Dataset::new("reports", "data", true),
    Dataset::new("booking_availability", "data", true),
    Dataset::new("meeting_types", "data", true),
    Dataset::new(WEBHOOK_ENDPOINTS_DATASET, "data", true).warning(WEBHOOK_SECRET_WARNING),
    Dataset::new("task_recurrences", "data", true),
    Dataset::new("people_organizations", "relations", false),
    Dataset::new("deal_contacts", "relations", false),
    Dataset::new("record_relations", "relations", false),
    Dataset::new("taggings", "relations", false),
    Dataset::new("task_links", "relations", false),
    Dataset::new("attachments", "relations", false),
    Dataset::new(RECORD_SHARES_DATASET, "relations", false),
    Dataset::new("record_followers", "relations", false),
    Dataset::new("favorites", "relations", false).excluding("favorite_type", "view"),
    Dataset::new("deal_members", "relations", false),
    Dataset::new("deal_renewals", "relations", false),
    Dataset::new("line_items", "data", false),
    Dataset::new("quote_items", "data", false),
    Dataset::new("quote_counters", "data", false).upsert(),
    Dataset::new("activity_calls", "data", false),
    Dataset::new("meeting_bookings", "data", false),
    Dataset::new("reminders", "data", false),
    Dataset::new("lead_disqualifications", "history", false),
    Dataset::new("notes", "history", false),
    Dataset::new(DOMAIN_EVENTS_DATASET, "history", false),
    Dataset::new("deal_stage_history", "history", false),
    Dataset::new("lifecycle_stage_history", "history", false),
];

pub(crate) fn dataset(key: &str) -> Option<&'static Dataset> {
    DATASETS.iter().find(|dataset| dataset.key == key)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum ConfigKind {
    Pipeline,
    PipelineStage,
    LostReason,
    Tag,
    ActivityType,
    Relationship,
}

impl ConfigKind {
    pub(crate) const ALL: [Self; 6] = [
        Self::Pipeline,
        Self::PipelineStage,
        Self::LostReason,
        Self::Tag,
        Self::ActivityType,
        Self::Relationship,
    ];

    pub(crate) const FALLBACK: [Self; 5] = [
        Self::Pipeline,
        Self::PipelineStage,
        Self::LostReason,
        Self::Tag,
        Self::ActivityType,
    ];

    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Pipeline => "pipeline",
            Self::PipelineStage => "pipeline_stage",
            Self::LostReason => "lost_reason",
            Self::Tag => "tag",
            Self::ActivityType => "activity_type",
            Self::Relationship => "relationship",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.as_str() == value)
    }

    pub(crate) fn table(&self) -> &'static str {
        match self {
            Self::Pipeline => "workflow",
            Self::PipelineStage => "workflow_stages",
            Self::LostReason => "lost_reasons",
            Self::Tag => "tags",
            Self::ActivityType => "task_types",
            Self::Relationship => "relationship_definitions",
        }
    }

    pub(crate) fn items_path(&self) -> String {
        format!("{}/{}.jsonl", super::CONFIG_ITEMS_DIR, self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ColumnRole {
    OwnId,
    User,
    Config(ConfigKind),
    Reference,
    Clear,
    Value,
}

const USER_COLUMNS: &[&str] = &[
    "owner_id",
    "user_id",
    "author_id",
    "created_by",
    "updated_by",
    "uploaded_by",
    "invited_by",
    "merged_by",
    "requested_by",
    "assigned_to",
    "added_by",
    "sent_by",
    "done_by",
    "shared_by",
    "host_user_id",
];

pub(crate) fn column_role(table: &str, column: &str) -> ColumnRole {
    if column == "id" {
        return ColumnRole::OwnId;
    }
    if USER_COLUMNS.contains(&column) {
        return ColumnRole::User;
    }
    match (table, column) {
        (_, "pipeline_id") => ColumnRole::Config(ConfigKind::Pipeline),
        (_, "stage_id" | "from_stage_id") => ColumnRole::Config(ConfigKind::PipelineStage),
        (_, "lost_reason_id") => ColumnRole::Config(ConfigKind::LostReason),
        (_, "activity_type_id") => ColumnRole::Config(ConfigKind::ActivityType),
        (_, "tag_id") => ColumnRole::Config(ConfigKind::Tag),
        (_, "relationship_id") => ColumnRole::Config(ConfigKind::Relationship),
        ("custom_records", "object_id") => ColumnRole::Value,
        ("domain_events", "actor_id") => ColumnRole::Value,
        ("lifecycle_stage_history" | "lead_disqualifications", "actor_id") => ColumnRole::User,
        ("record_shares", "grantee_id") => ColumnRole::Value,
        ("meeting_types", "distribution_rule_id")
        | ("deal_members", "role_id")
        | ("activity_calls", "session_id")
        | ("lead_disqualifications", "reason_id") => ColumnRole::Clear,
        (_, column) if column.ends_with("_id") => ColumnRole::Reference,
        _ => ColumnRole::Value,
    }
}

pub(crate) fn record_priority(definition: &EntityDefinition) -> u8 {
    definition.capabilities.restore_order
}

pub(crate) fn record_path(record_type: &str) -> String {
    format!("{}/{record_type}.jsonl", super::RECORDS_DIR)
}
