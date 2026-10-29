use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::UserId;

pub const MAX_RELATIONSHIP_KEY_LENGTH: usize = 48;
pub const MAX_RELATIONSHIP_LABEL_LENGTH: usize = 80;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Cardinality {
    OneToOne,
    OneToMany,
    ManyToMany,
}

impl Cardinality {
    pub const ALL: [Self; 3] = [Self::OneToOne, Self::OneToMany, Self::ManyToMany];

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::OneToOne => "one_to_one",
            Self::OneToMany => "one_to_many",
            Self::ManyToMany => "many_to_many",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|cardinality| cardinality.as_str() == value)
    }

    pub fn single_target_from_source(&self) -> bool {
        matches!(self, Self::OneToOne)
    }

    pub fn single_source_from_target(&self) -> bool {
        matches!(self, Self::OneToOne | Self::OneToMany)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OnDelete {
    Restrict,
    Unlink,
}

impl OnDelete {
    pub const ALL: [Self; 2] = [Self::Restrict, Self::Unlink];

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Restrict => "restrict",
            Self::Unlink => "unlink",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|action| action.as_str() == value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RelationSide {
    From,
    To,
}

impl RelationSide {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::From => "from",
            Self::To => "to",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelationshipDefinition {
    pub id: Uuid,
    pub key: String,
    pub inverse_key: String,
    pub from_type: String,
    pub to_type: String,
    pub cardinality: Cardinality,
    pub from_label: String,
    pub to_label: String,
    pub required: bool,
    pub on_delete: OnDelete,
    pub created_by: Option<UserId>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl RelationshipDefinition {
    pub fn relation_key(&self, side: RelationSide) -> &str {
        match side {
            RelationSide::From => &self.key,
            RelationSide::To => &self.inverse_key,
        }
    }

    pub fn target_type(&self, side: RelationSide) -> &str {
        match side {
            RelationSide::From => &self.to_type,
            RelationSide::To => &self.from_type,
        }
    }

    pub fn label(&self, side: RelationSide) -> &str {
        match side {
            RelationSide::From => &self.from_label,
            RelationSide::To => &self.to_label,
        }
    }

    pub fn single(&self, side: RelationSide) -> bool {
        match side {
            RelationSide::From => self.cardinality.single_target_from_source(),
            RelationSide::To => self.cardinality.single_source_from_target(),
        }
    }

    pub fn sides_of(&self, record_type: &str) -> Vec<RelationSide> {
        let mut sides = Vec::new();
        if self.from_type == record_type {
            sides.push(RelationSide::From);
        }
        if self.to_type == record_type {
            sides.push(RelationSide::To);
        }
        sides
    }
}
