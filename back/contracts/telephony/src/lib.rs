use aphel_contract::{Contract, ContractDescriptor, KeyEntry, KeyKind, contract};

pub const CONTRACT: Contract = contract!("telephony", "1.0.0");
pub const CALL_LOGGED: &str = "call.logged";
pub const CALL_UPDATED: &str = "call.updated";
pub const DESCRIPTOR: ContractDescriptor = ContractDescriptor {
    contract: CONTRACT,
    entries: &[KeyEntry::new(
        "telephony.providers",
        KeyKind::ExtensionPoint,
        "Telephony providers",
    )],
};
