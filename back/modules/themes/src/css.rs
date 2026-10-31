use std::collections::{BTreeMap, HashSet};

use serde_json::Value;

use crate::package::TokenSets;
use crate::store::StoredFont;
use crate::tokens::{definition, render_value, wrap_organization_override};

pub(crate) const FONT_PATH_PREFIX: &str = "/api/v1/public/themes/fonts/";

pub(crate) fn build(tokens: &TokenSets, fonts: &[StoredFont]) -> String {
    let families: HashSet<String> = fonts.iter().map(|font| font.family.clone()).collect();
    let mut css = String::new();
    for font in fonts {
        css.push_str(&format!(
            "@font-face{{font-family:'{}';font-style:{};font-weight:{};font-display:swap;\
             src:url('{FONT_PATH_PREFIX}{}.woff2') format('woff2');}}\n",
            font.family, font.style, font.weight, font.sha256
        ));
    }
    css.push_str(&block("html:root", &tokens.shared, &families, Mode::Light));
    css.push_str(&block(
        "html:root:not([data-theme='dark'])",
        &tokens.light,
        &families,
        Mode::Light,
    ));
    css.push_str(&block(
        "html[data-theme='dark']",
        &tokens.dark,
        &families,
        Mode::Dark,
    ));
    css
}

#[derive(Clone, Copy)]
enum Mode {
    Light,
    Dark,
}

fn block(
    selector: &str,
    values: &BTreeMap<String, Value>,
    families: &HashSet<String>,
    mode: Mode,
) -> String {
    let declarations: String = values
        .iter()
        .filter_map(|(name, value)| {
            let token = definition(name)?;
            let rendered = render_value(token, value, families).ok()?;
            let property = match mode {
                Mode::Light => token.org_light.as_deref(),
                Mode::Dark => token.org_dark.as_deref(),
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
