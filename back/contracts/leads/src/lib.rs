use aphel_contract::{ContractDescriptor, HookKind, KeyEntry, KeyKind, contract};
pub const KEY: &str = "leads";
pub const LIFECYCLE_STAGE_FIELD: &str = "lifecycle_stage";
pub const LEAD_STAGE: &str = "lead";
pub const QUALIFYING: &str = "leads.qualifying";
pub const CONVERTING: &str = "leads.converting";
pub const DESCRIPTOR: ContractDescriptor = ContractDescriptor {
    contract: contract!("leads", "1.0.0"),
    entries: &[
        KeyEntry::new(
            QUALIFYING,
            KeyKind::Hook(HookKind::Veto),
            "Before lead qualification",
        ),
        KeyEntry::new(
            CONVERTING,
            KeyKind::Hook(HookKind::Filter),
            "Enrich owner-managed lead conversion values",
        ),
        KeyEntry::new(
            "leads.converter",
            KeyKind::ExtensionPoint,
            "Transactional lead conversion and pending commits",
        ),
    ],
};
