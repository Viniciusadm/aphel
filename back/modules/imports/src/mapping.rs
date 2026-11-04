use std::collections::BTreeSet;

use crate::ImportTarget;

type TargetMatcher = fn(&ImportTarget, &str) -> bool;

pub use aphel_sdk::imports::normalize_label;

pub(crate) fn suggest_mapping(headers: &[String], targets: &[ImportTarget]) -> Vec<Option<String>> {
    let normalized: Vec<String> = headers
        .iter()
        .map(|header| normalize_label(header))
        .collect();
    let mut used = BTreeSet::new();
    let mut suggestions: Vec<Option<String>> = vec![None; headers.len()];

    let passes: [TargetMatcher; 3] = [
        |target, header| normalize_label(&target.label) == header,
        |target, header| normalize_label(&target.key) == header,
        |target, header| {
            target
                .aliases
                .iter()
                .any(|alias| normalize_label(alias) == header)
        },
    ];
    for matches in passes {
        for (index, header) in normalized.iter().enumerate() {
            if suggestions[index].is_some() || header.is_empty() {
                continue;
            }
            if let Some(target) = targets
                .iter()
                .find(|target| !used.contains(&target.key) && matches(target, header))
            {
                used.insert(target.key.clone());
                suggestions[index] = Some(target.key.clone());
            }
        }
    }
    suggestions
}
