use aphel_contract::{ContractDescriptor, KeyEntry, KeyKind, contract};

pub const DESCRIPTOR: ContractDescriptor = ContractDescriptor {
    contract: contract!("reminders", "1.0.0"),
    entries: &[
        KeyEntry::new(
            "reminders.list",
            KeyKind::Operation,
            "Reminders of the acting user sorted by remind_at (needs a user actor): { status?, record_type?, record_id?, page?, per_page? } -> { data, total }",
        ),
        KeyEntry::new(
            "reminders.create",
            KeyKind::Operation,
            "Create a reminder for the acting user (needs a user actor): { remind_at, note?, record_type?, record_id? }",
        ),
        KeyEntry::new(
            "reminders.delete",
            KeyKind::Operation,
            "Delete a reminder of the acting user (needs a user actor): { id }",
        ),
    ],
};
