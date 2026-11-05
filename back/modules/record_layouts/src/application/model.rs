use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use aphel_kernel::domain::UserId;
use aphel_sdk::platform::application::query::Filter;
use aphel_sdk::platform::application::shared::RecordType;

pub const FORM_LAYOUT_RECORD_TYPE: RecordType = RecordType::from_static("form_layout");

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LayoutContext {
    Create,
    Edit,
    View,
}

impl LayoutContext {
    pub const ALL: [LayoutContext; 3] = [Self::Create, Self::Edit, Self::View];

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Create => "create",
            Self::Edit => "edit",
            Self::View => "view",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "create" => Some(Self::Create),
            "edit" => Some(Self::Edit),
            "view" => Some(Self::View),
            _ => None,
        }
    }

    pub fn accepts_required_rules(&self) -> bool {
        *self != Self::View
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FormLayout {
    pub tabs: Vec<LayoutTab>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LayoutTab {
    pub id: String,
    #[serde(default)]
    pub label: String,
    pub sections: Vec<LayoutSection>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LayoutSection {
    pub id: String,
    #[serde(default)]
    pub label: String,
    pub columns: Vec<LayoutColumn>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LayoutColumn {
    #[serde(default)]
    pub fields: Vec<LayoutField>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LayoutField {
    pub key: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub visible_when: Option<Filter>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub required_when: Option<Filter>,
}

impl LayoutField {
    pub fn new(key: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            visible_when: None,
            required_when: None,
        }
    }

    pub fn visible_when(mut self, filter: Filter) -> Self {
        self.visible_when = Some(filter);
        self
    }

    pub fn required_when(mut self, filter: Filter) -> Self {
        self.required_when = Some(filter);
        self
    }
}

impl FormLayout {
    pub fn single(fields: Vec<LayoutField>) -> Self {
        Self {
            tabs: vec![LayoutTab {
                id: "main".to_owned(),
                label: String::new(),
                sections: vec![LayoutSection {
                    id: "main".to_owned(),
                    label: String::new(),
                    columns: vec![LayoutColumn { fields }],
                }],
            }],
        }
    }

    pub fn fields(&self) -> impl Iterator<Item = &LayoutField> {
        self.tabs
            .iter()
            .flat_map(|tab| tab.sections.iter())
            .flat_map(|section| section.columns.iter())
            .flat_map(|column| column.fields.iter())
    }

    pub fn contains(&self, key: &str) -> bool {
        self.fields().any(|field| field.key == key)
    }

    pub fn has_required_rules(&self) -> bool {
        self.fields().any(|field| field.required_when.is_some())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct StoredFormLayout {
    pub id: Uuid,
    pub record_type: RecordType,
    pub context: LayoutContext,
    pub layout: FormLayout,
    pub updated_by: Option<UserId>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FormLayoutEntry {
    pub record_type: RecordType,
    pub context: LayoutContext,
    pub layout: Option<FormLayout>,
    pub updated_by: Option<UserId>,
    pub updated_at: Option<DateTime<Utc>>,
}

impl FormLayoutEntry {
    pub fn empty(record_type: RecordType, context: LayoutContext) -> Self {
        Self {
            record_type,
            context,
            layout: None,
            updated_by: None,
            updated_at: None,
        }
    }
}

impl From<StoredFormLayout> for FormLayoutEntry {
    fn from(stored: StoredFormLayout) -> Self {
        Self {
            record_type: stored.record_type,
            context: stored.context,
            layout: Some(stored.layout),
            updated_by: stored.updated_by,
            updated_at: Some(stored.updated_at),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SaveLayoutCommand {
    pub layout: FormLayout,
    pub ip: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct RulesSource {
    pub layout: Option<FormLayout>,
    pub timezone: Option<String>,
}
