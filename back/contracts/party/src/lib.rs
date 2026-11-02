use aphel_contract::{
    Contract, ContractDescriptor, ExtensionPoint, KeyEntry, KeyKind, contract, extension_point,
};
use serde::{Deserialize, Serialize};

pub const CONTRACT: Contract = contract!("party", "1.0.0");
pub const CONTACT_POINTS: ExtensionPoint<ContactPoints> = extension_point!("party.contact_points");
pub const DESCRIPTOR: ContractDescriptor = ContractDescriptor {
    contract: CONTRACT,
    entries: &[KeyEntry::new(
        "party.contact_points",
        KeyKind::ExtensionPoint,
        "Entity field mappings for party email, phone and web contact points",
    )],
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContactPointKind {
    Email,
    Phone,
    Web,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContactPoint {
    pub field: String,
    pub kind: ContactPointKind,
}
impl ContactPoint {
    pub fn new(field: impl Into<String>, kind: ContactPointKind) -> Self {
        Self {
            field: field.into(),
            kind,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "ContactPointMapping")]
pub struct ContactPoints {
    columns: Vec<String>,
    points: Vec<ContactPoint>,
}
#[derive(Default, Deserialize)]
#[serde(default)]
struct ContactPointMapping {
    columns: Vec<String>,
    points: Vec<ContactPoint>,
}
impl From<ContactPointMapping> for ContactPoints {
    fn from(mapping: ContactPointMapping) -> Self {
        if mapping.points.is_empty() {
            Self {
                columns: mapping.columns,
                points: Vec::new(),
            }
        } else {
            Self::new(mapping.points)
        }
    }
}
impl ContactPoints {
    pub fn new(points: impl IntoIterator<Item = ContactPoint>) -> Self {
        let points: Vec<_> = points.into_iter().collect();
        let mut columns = Vec::new();
        for point in &points {
            if !columns.contains(&point.field) {
                columns.push(point.field.clone());
            }
        }
        Self { columns, points }
    }
    pub fn columns(&self) -> &[String] {
        &self.columns
    }
    pub fn fields(&self, kind: ContactPointKind) -> impl Iterator<Item = &str> {
        self.points
            .iter()
            .filter(move |point| point.kind == kind)
            .map(|point| point.field.as_str())
    }
}
