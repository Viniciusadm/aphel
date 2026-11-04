use aphel_contract::{ContractDescriptor, KeyEntry, KeyKind, contract};
pub const KEY: &str = "sso";
pub const DESCRIPTOR: ContractDescriptor = ContractDescriptor {
    contract: contract!("sso", "1.0.0"),
    entries: &[
        KeyEntry::new(
            "sso.identity_flows",
            KeyKind::ExtensionPoint,
            "Owner-managed scoped identity login and token exchange flows",
        ),
        KeyEntry::new(
            "sso.oauth_protocol",
            KeyKind::ExtensionPoint,
            "Enabled OAuth token and discovery protocol executor",
        ),
        KeyEntry::new(
            "sso.oauth_states",
            KeyKind::ExtensionPoint,
            "Provider-partitioned single-use OAuth nonce and PKCE state",
        ),
    ],
};
