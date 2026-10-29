use std::collections::BTreeSet;

use serde_json::Value;

use crate::application::config_packages::{PackageIssue, PlanChange, is_config_key};

pub fn diff(current: &[(String, Value)], proposed: &[(String, Value)]) -> Vec<PlanChange> {
    proposed
        .iter()
        .filter_map(|(property, value)| {
            let old = current
                .iter()
                .find(|(name, _)| name == property)
                .map_or(Value::Null, |(_, value)| value.clone());
            (&old != value).then(|| PlanChange::new(property.clone(), old, value.clone()))
        })
        .collect()
}

pub fn check_key(
    key: &str,
    path: &str,
    seen: &mut BTreeSet<String>,
    issues: &mut Vec<PackageIssue>,
) {
    if !is_config_key(key) {
        issues.push(PackageIssue::new(
            path,
            "invalid_key",
            "must be snake_case: lowercase letters, digits and single underscores, starting with a letter (max 64)",
        ));
    } else if !seen.insert(key.to_owned()) {
        issues.push(PackageIssue::new(
            path,
            "duplicated_key",
            "is declared more than once",
        ));
    }
}
