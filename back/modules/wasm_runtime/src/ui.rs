use std::collections::{BTreeMap, HashSet};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use aphel_kernel::errors::FieldViolation;

use crate::ui_tokens::{definition, render_value, wrap_organization_override};

const MAX_LABEL_LENGTH: usize = 60;
const MAX_DESCRIPTION_LENGTH: usize = 300;
const MAX_TOKENS: usize = 400;
const PREVIEW_TOKENS: [&str; 4] = ["primary", "background", "foreground", "accent"];

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TokenSets {
    #[serde(default)]
    pub shared: BTreeMap<String, Value>,
    #[serde(default)]
    pub light: BTreeMap<String, Value>,
    #[serde(default)]
    pub dark: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UiSpec {
    pub label: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub tokens: TokenSets,
}

impl UiSpec {
    pub fn validate(&self, violations: &mut Vec<FieldViolation>) {
        let label = self.label.trim().chars().count();
        if label == 0 || label > MAX_LABEL_LENGTH {
            violations.push(FieldViolation::new(
                "ui.label",
                format!("must have 1 to {MAX_LABEL_LENGTH} characters"),
            ));
        }
        if self
            .description
            .as_deref()
            .is_some_and(|text| text.chars().count() > MAX_DESCRIPTION_LENGTH)
        {
            violations.push(FieldViolation::new(
                "ui.description",
                format!("must have at most {MAX_DESCRIPTION_LENGTH} characters"),
            ));
        }
        let total = self.tokens.shared.len() + self.tokens.light.len() + self.tokens.dark.len();
        if total > MAX_TOKENS {
            violations.push(FieldViolation::new(
                "ui.tokens",
                format!("can set at most {MAX_TOKENS} tokens"),
            ));
            return;
        }
        let no_bundled_fonts = HashSet::new();
        for (set, values) in [
            ("shared", &self.tokens.shared),
            ("light", &self.tokens.light),
            ("dark", &self.tokens.dark),
        ] {
            for (name, value) in values {
                let path = format!("ui.tokens.{set}.{name}");
                match definition(name) {
                    None => violations.push(FieldViolation::new(path, "is not a design token")),
                    Some(token) => {
                        if let Err(message) = render_value(token, value, &no_bundled_fonts) {
                            violations.push(FieldViolation::new(path, message));
                        }
                    }
                }
            }
        }
    }

    pub fn css(&self) -> String {
        let families = HashSet::new();
        let mut css = String::new();
        css.push_str(&block("html:root", &self.tokens.shared, &families, false));
        css.push_str(&block(
            "html:root:not([data-theme='dark'])",
            &self.tokens.light,
            &families,
            false,
        ));
        css.push_str(&block(
            "html[data-theme='dark']",
            &self.tokens.dark,
            &families,
            true,
        ));
        css
    }

    pub fn preview(&self) -> BTreeMap<String, String> {
        PREVIEW_TOKENS
            .iter()
            .filter_map(|name| {
                let value = self
                    .tokens
                    .light
                    .get(*name)
                    .or_else(|| self.tokens.shared.get(*name))?
                    .as_str()?;
                Some(((*name).to_owned(), value.to_owned()))
            })
            .collect()
    }

    pub fn has_tokens(&self) -> bool {
        !(self.tokens.shared.is_empty()
            && self.tokens.light.is_empty()
            && self.tokens.dark.is_empty())
    }
}

fn block(
    selector: &str,
    values: &BTreeMap<String, Value>,
    families: &HashSet<String>,
    dark: bool,
) -> String {
    let declarations: String = values
        .iter()
        .filter_map(|(name, value)| {
            let token = definition(name)?;
            let rendered = render_value(token, value, families).ok()?;
            let property = if dark {
                token.org_dark.as_deref()
            } else {
                token.org_light.as_deref()
            };
            Some(format!(
                "--{name}:{};",
                wrap_organization_override(&rendered, property)
            ))
        })
        .collect();
    if declarations.is_empty() {
        return String::new();
    }
    format!("{selector}{{{declarations}}}\n")
}

pub fn probe(failures: &mut Vec<String>) {
    let spec = |value: Value| serde_json::from_value::<UiSpec>(value);
    let violations = |spec: &UiSpec| {
        let mut violations = Vec::new();
        spec.validate(&mut violations);
        violations
    };
    let valid = spec(serde_json::json!({
        "label": "Probe",
        "tokens": { "light": { "background": "#101828", "radius": "0.5rem" } }
    }));
    match valid {
        Ok(valid) => {
            if !violations(&valid).is_empty() {
                failures.push("ui plugin: a valid token set must be accepted".into());
            }
            let css = valid.css();
            if !css.contains("--background:#101828")
                || !css.contains("html:root:not([data-theme='dark'])")
            {
                failures
                    .push("ui plugin: accepted tokens must render as CSS custom properties".into());
            }
        }
        Err(_) => failures.push("ui plugin: a valid spec must deserialize".into()),
    }
    for (name, value) in [
        (
            "unknown token",
            serde_json::json!({"label": "x", "tokens": {"light": {"not-a-token": "#fff"}}}),
        ),
        (
            "url in value",
            serde_json::json!({"label": "x", "tokens": {"light": {"background": "url(https://example.invalid/a.png)"}}}),
        ),
        (
            "declaration break",
            serde_json::json!({"label": "x", "tokens": {"light": {"background": "#fff;}body{display:none"}}}),
        ),
        (
            "not a color",
            serde_json::json!({"label": "x", "tokens": {"light": {"background": "red"}}}),
        ),
        (
            "empty label",
            serde_json::json!({"label": " ", "tokens": {}}),
        ),
    ] {
        match spec(value) {
            Ok(spec) if violations(&spec).is_empty() => {
                failures.push(format!("ui plugin: {name} must be rejected"));
            }
            _ => {}
        }
    }
    if spec(serde_json::json!({"label": "x", "script": "alert(1)"})).is_ok() {
        failures.push("ui plugin: unknown spec properties must be rejected".into());
    }
    let example = crate::manifest::PluginManifest::parse(
        "key = \"probe\"\nname = \"Probe\"\nversion = \"1.0.0\"\napi = 1\n[ui]\nlabel = \"Probe\"\n[ui.tokens.light]\nbackground = \"#101828\"\n",
    )
    .and_then(|manifest| {
        manifest.validate(
            &[],
            aphel_sdk::platform::modules::builder::APHEL_VERSION,
            crate::manifest::HostCatalog {
                hooks: &[],
                operations: &[],
            },
        )?;
        Ok(manifest)
    });
    if !example.is_ok_and(|manifest| manifest.ui.is_some()) {
        failures.push("ui plugin: a manifest with `[ui]` must parse and validate".into());
    }
}
