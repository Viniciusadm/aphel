use aphel_contract::{ContractDescriptor, KeyEntry, KeyKind, contract};
use aphel_contract_notifications::NotificationKind;
pub const TASK_DUE_NOTIFICATION: NotificationKind = NotificationKind::from_static("task_due");
pub const DESCRIPTOR: ContractDescriptor = ContractDescriptor {
    contract: contract!("tasks", "1.0.0"),
    entries: &[
        KeyEntry::new(
            "tasks.create",
            KeyKind::Operation,
            "Create a task: { title, activity_type_id, due_at?, owner_id?, record_type?, record_id?, description?, duration_minutes? } -> { id }",
        ),
        KeyEntry::new(
            "tasks.types",
            KeyKind::Operation,
            "Active task types: {} -> { items: [{ id, key, name }] }",
        ),
        KeyEntry::new(
            "tasks.complete",
            KeyKind::Operation,
            "Mark an open task as done now: { id }",
        ),
        KeyEntry::new(
            "tasks.remote_calendars",
            KeyKind::ExtensionPoint,
            "Owner remote calendar connection, availability and synchronization execution",
        ),
        KeyEntry::new(
            "tasks.reminder_source",
            KeyKind::ExtensionPoint,
            "Owned task reminder selection and delivery receipts",
        ),
        KeyEntry::new(
            "tasks.calendar_api",
            KeyKind::ExtensionPoint,
            "Native remote calendar provider protocol and snapshots",
        ),
    ],
};
