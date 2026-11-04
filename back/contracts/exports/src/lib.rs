use aphel_contract::{Contract, ContractDescriptor, KeyEntry, KeyKind, contract};
use aphel_contract_notifications::NotificationKind;
pub const CONTRACT: Contract = contract!("exports", "1.0.0");
pub const DESCRIPTOR: ContractDescriptor = ContractDescriptor {
    contract: CONTRACT,
    entries: &[KeyEntry::new(
        "exports.request",
        KeyKind::ExtensionPoint,
        "Scoped queued record exports",
    )],
};
pub const EXPORT_READY_NOTIFICATION: NotificationKind =
    NotificationKind::from_static("export_ready");
pub const EXPORT_FAILED_NOTIFICATION: NotificationKind =
    NotificationKind::from_static("export_failed");
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    Csv,
    Xlsx,
}

impl ExportFormat {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Csv => "csv",
            Self::Xlsx => "xlsx",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "csv" => Some(Self::Csv),
            "xlsx" => Some(Self::Xlsx),
            _ => None,
        }
    }
}
