use aphel_sdk::catalog::{CatalogContributor, HookEntry};

const RECORD_HOOK_ROWS: [(&str, &str, &str); 9] = [
    (
        "record.saving",
        "filter",
        "The draft `values`/`custom_fields`; return changes",
    ),
    (
        "record.validating",
        "veto",
        "`values` and `changes` of the write",
    ),
    (
        "record.saved",
        "action",
        "`values` and `changes`, inside the transaction",
    ),
    ("record.deleting", "veto", "The record being deleted"),
    (
        "record.archiving",
        "veto",
        "The custom record or company being archived",
    ),
    (
        "record.committed",
        "action",
        "`values` and `changes`, after the commit (errors only logged)",
    ),
    (
        "record.deleted",
        "action",
        "The deleted record, inside the transaction",
    ),
    (
        "record.restored",
        "action",
        "The restored record, inside the transaction",
    ),
    (
        "record.purged",
        "action",
        "The purged record, inside the transaction",
    ),
];

pub(crate) struct PluginCatalog;

impl CatalogContributor for PluginCatalog {
    fn hooks(&self) -> Vec<HookEntry> {
        RECORD_HOOK_ROWS
            .iter()
            .map(|(key, kind, description)| HookEntry {
                key: (*key).to_owned(),
                module: crate::KEY,
                kind: (*kind).to_owned(),
                record_type: Some("any".to_owned()),
                args: None,
                description: (*description).to_owned(),
                listeners: Vec::new(),
                enabled: true,
            })
            .collect()
    }
}
