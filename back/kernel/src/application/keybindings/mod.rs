use std::collections::BTreeMap;

use crate::errors::FieldViolation;

pub const KEYBINDINGS_UPDATED: &str = "organization.keybindings_updated";
pub const MAX_KEYBINDING_ACTIONS: usize = 500;
pub const MAX_KEYBINDING_ACTION_ID_LENGTH: usize = 96;
pub const MAX_CHORDS_PER_ACTION: usize = 4;
pub const MAX_SEQUENCE_STEPS: usize = 2;

const MODIFIERS: [&str; 5] = ["mod", "ctrl", "alt", "shift", "meta"];
const NAMED_KEYS: [&str; 16] = [
    "enter",
    "escape",
    "space",
    "tab",
    "backspace",
    "delete",
    "insert",
    "home",
    "end",
    "pageup",
    "pagedown",
    "arrowup",
    "arrowdown",
    "arrowleft",
    "arrowright",
    "plus",
];

pub type Keybindings = BTreeMap<String, Vec<String>>;

pub fn is_keybinding_action_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_KEYBINDING_ACTION_ID_LENGTH
        && value.as_bytes()[0].is_ascii_alphanumeric()
        && value.bytes().all(|c| {
            c.is_ascii_lowercase()
                || c.is_ascii_digit()
                || matches!(c, b'_' | b'.' | b':' | b'-' | b'/')
        })
}

fn is_function_key(key: &str) -> bool {
    key.strip_prefix('f')
        .and_then(|number| number.parse::<u8>().ok())
        .is_some_and(|number| (1..=24).contains(&number) && !key.starts_with("f0"))
}

fn normalize_key(key: &str) -> Option<String> {
    if NAMED_KEYS.contains(&key) || is_function_key(key) {
        return Some(key.to_owned());
    }
    let mut chars = key.chars();
    let single = chars.next()?;
    if chars.next().is_some() || single.is_whitespace() || single == '+' || single.is_control() {
        return None;
    }
    Some(single.to_lowercase().collect())
}

fn normalize_stroke(stroke: &str) -> Option<String> {
    let tokens: Vec<&str> = stroke.split('+').collect();
    let (key, modifiers) = tokens.split_last()?;
    let mut present = [false; MODIFIERS.len()];
    for modifier in modifiers {
        let index = MODIFIERS
            .iter()
            .position(|candidate| candidate == modifier)?;
        if present[index] {
            return None;
        }
        present[index] = true;
    }
    let key = normalize_key(key)?;
    let mut parts: Vec<String> = MODIFIERS
        .iter()
        .zip(present)
        .filter(|(_, on)| *on)
        .map(|(name, _)| (*name).to_owned())
        .collect();
    parts.push(key);
    Some(parts.join("+"))
}

pub fn normalize_chord(chord: &str) -> Option<String> {
    let lowered = chord.trim().to_lowercase();
    let steps: Vec<&str> = lowered.split(' ').collect();
    if steps.is_empty() || steps.len() > MAX_SEQUENCE_STEPS {
        return None;
    }
    let normalized: Option<Vec<String>> = steps.into_iter().map(normalize_stroke).collect();
    Some(normalized?.join(" "))
}

pub fn normalize_keybindings(
    field: &str,
    keybindings: Keybindings,
    violations: &mut Vec<FieldViolation>,
) -> Keybindings {
    if keybindings.len() > MAX_KEYBINDING_ACTIONS {
        violations.push(FieldViolation::new(
            field,
            format!("must have at most {MAX_KEYBINDING_ACTIONS} actions"),
        ));
        return Keybindings::new();
    }
    let mut normalized = Keybindings::new();
    for (action, chords) in keybindings {
        let path = format!("{field}.{action}");
        if !is_keybinding_action_id(&action) {
            violations.push(FieldViolation::new(path, "is not a valid action id"));
            continue;
        }
        if chords.len() > MAX_CHORDS_PER_ACTION {
            violations.push(FieldViolation::new(
                path,
                format!("must have at most {MAX_CHORDS_PER_ACTION} key combinations"),
            ));
            continue;
        }
        let mut seen: Vec<String> = Vec::new();
        for chord in &chords {
            match normalize_chord(chord) {
                Some(chord) if !seen.contains(&chord) => seen.push(chord),
                Some(_) => {}
                None => violations.push(FieldViolation::new(
                    path.clone(),
                    format!("\"{chord}\" is not a valid key combination"),
                )),
            }
        }
        normalized.insert(action, seen);
    }
    normalized
}

pub fn keybindings_from_json(value: &serde_json::Value) -> Keybindings {
    value
        .as_object()
        .map(|entries| {
            entries
                .iter()
                .filter_map(|(action, chords)| {
                    let chords = chords
                        .as_array()?
                        .iter()
                        .filter_map(|chord| chord.as_str().map(str::to_owned))
                        .collect();
                    Some((action.clone(), chords))
                })
                .collect()
        })
        .unwrap_or_default()
}

pub fn keybindings_to_json(keybindings: &Keybindings) -> serde_json::Value {
    serde_json::json!(keybindings)
}
