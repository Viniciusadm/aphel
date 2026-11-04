use aphel_contract::{ContractDescriptor, KeyEntry, KeyKind, contract};
pub const DESCRIPTOR: ContractDescriptor = ContractDescriptor {
    contract: contract!("sequences", "1.0.0"),
    entries: &[KeyEntry::new(
        "sequences.enrollment",
        KeyKind::ExtensionPoint,
        "Scoped sequence enrollment",
    )],
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EnrollmentSource {
    Manual,
    Bulk,
    Automation,
}

impl EnrollmentSource {
    pub const ALL: [Self; 3] = [Self::Manual, Self::Bulk, Self::Automation];

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Manual => "manual",
            Self::Bulk => "bulk",
            Self::Automation => "automation",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|source| source.as_str() == value)
    }
}
