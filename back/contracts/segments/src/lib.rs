use aphel_contract::{ContractDescriptor, KeyEntry, KeyKind, contract};
pub const KEY: &str = "segments";
pub const RECORD_TYPE: &str = "segments.segment";
pub const DESCRIPTOR: ContractDescriptor = ContractDescriptor {
    contract: contract!("segments", "1.0.0"),
    entries: &[
        KeyEntry::new(RECORD_TYPE, KeyKind::Reference, "A saved record filter"),
        KeyEntry::new(
            "segments.target_reader",
            KeyKind::ExtensionPoint,
            "Readable saved filter target metadata",
        ),
    ],
};
