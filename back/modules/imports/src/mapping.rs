use std::collections::BTreeSet;

use crate::ImportTarget;

type TargetMatcher = fn(&ImportTarget, &str) -> bool;

pub fn normalize_label(text: &str) -> String {
    let mut normalized = String::with_capacity(text.len());
    let mut pending_space = false;
    for character in text.chars().flat_map(char::to_lowercase) {
        let folded = fold_accent(character);
        if folded.is_ascii_alphanumeric() {
            if pending_space && !normalized.is_empty() {
                normalized.push(' ');
            }
            pending_space = false;
            normalized.push(folded);
        } else {
            pending_space = true;
        }
    }
    normalized
}

fn fold_accent(character: char) -> char {
    match character {
        'á' | 'à' | 'â' | 'ã' | 'ä' | 'å' => 'a',
        'é' | 'è' | 'ê' | 'ë' => 'e',
        'í' | 'ì' | 'î' | 'ï' => 'i',
        'ó' | 'ò' | 'ô' | 'õ' | 'ö' => 'o',
        'ú' | 'ù' | 'û' | 'ü' => 'u',
        'ç' => 'c',
        'ñ' => 'n',
        'ý' | 'ÿ' => 'y',
        other => other,
    }
}

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
