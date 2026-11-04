use aphel_contract::{ContractDescriptor, KeyEntry, KeyKind, contract};
use aphel_contract_notifications::NotificationKind;
pub const TASK_DUE_NOTIFICATION: NotificationKind = NotificationKind::from_static("task_due");
pub const DESCRIPTOR: ContractDescriptor = ContractDescriptor {
    contract: contract!("tasks", "1.0.0"),
    entries: &[KeyEntry::new(
        "tasks.reminder_source",
        KeyKind::ExtensionPoint,
        "Owned task reminder selection and delivery receipts",
    )],
};
