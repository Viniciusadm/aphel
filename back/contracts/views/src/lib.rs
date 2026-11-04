use aphel_contract::{
    Contract, ContractDescriptor, KeyEntry, KeyKind, ReferenceKind, contract, reference,
};
use serde::{Deserialize, Serialize};
pub const CONTRACT: Contract = contract!("views", "1.0.0");
pub const VIEW: ReferenceKind = reference!("views.view");
pub const DESCRIPTOR: ContractDescriptor = ContractDescriptor {
    contract: CONTRACT,
    entries: &[
        KeyEntry::new("views.view", KeyKind::Reference, "A saved record view"),
        KeyEntry::new(
            "views.reader",
            KeyKind::ExtensionPoint,
            "Scoped saved view definitions",
        ),
    ],
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ViewVisibility {
    Private,
    Team,
    Organization,
}

impl ViewVisibility {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Private => "private",
            Self::Team => "team",
            Self::Organization => "organization",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "private" => Some(Self::Private),
            "team" => Some(Self::Team),
            "organization" => Some(Self::Organization),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ViewType {
    Table,
    Kanban,
    Calendar,
}

impl ViewType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Table => "table",
            Self::Kanban => "kanban",
            Self::Calendar => "calendar",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "table" => Some(Self::Table),
            "kanban" => Some(Self::Kanban),
            "calendar" => Some(Self::Calendar),
            _ => None,
        }
    }
}
