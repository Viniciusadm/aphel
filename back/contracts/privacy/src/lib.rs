#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubjectKind {
    Email,
    Phone,
    Document,
    Text,
}

impl SubjectKind {
    pub const ALL: [Self; 4] = [Self::Email, Self::Phone, Self::Document, Self::Text];

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Email => "email",
            Self::Phone => "phone",
            Self::Document => "document",
            Self::Text => "text",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.as_str() == value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataSubjectAction {
    Export,
    Anonymize,
    Delete,
}

impl DataSubjectAction {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Export => "export",
            Self::Anonymize => "anonymize",
            Self::Delete => "delete",
        }
    }
}

#[derive(
    Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub struct SourceTarget {
    pub source: String,
    pub id: String,
}
pub const DESCRIPTOR: aphel_contract::ContractDescriptor = aphel_contract::ContractDescriptor {
    contract: aphel_contract::contract!("privacy", "1.0.0"),
    entries: &[
        aphel_contract::KeyEntry::new(
            "privacy.source",
            aphel_contract::KeyKind::ExtensionPoint,
            "Owner-managed personal content discovery, export and erasure",
        ),
        aphel_contract::KeyEntry::new(
            "privacy.data_subject",
            aphel_contract::KeyKind::ExtensionPoint,
            "External data subject lifecycle handlers",
        ),
    ],
};

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct EntityPrivacyPolicy {
    pub protected_fields: Vec<String>,
}
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct RetentionMapping {
    pub recency_fields: Vec<String>,
}
