use std::fs;
use std::path::Path;

use serde::Deserialize;

use crate::{Failure, Generated, back_dir, write_if_changed};

const TOKENS_FILE: &str = "modules/wasm_runtime/tokens.json";
const WEB_TOKENS_CSS: &str = "../web/src/sdk/tokens.css";
const WEB_TOKENS_TS: &str = "../web/src/sdk/tokens.generated.ts";
const TOKENS_DOC: &str = "../docs/ui/tokens.md";

#[derive(Debug, Deserialize)]
struct TokenFile {
    tokens: Vec<Token>,
}

#[derive(Debug, Deserialize)]
struct Token {
    name: String,
    kind: String,
    group: String,
    light: String,
    dark: Option<String>,
    org_light: Option<String>,
    org_dark: Option<String>,
}

impl Token {
    fn light_value(&self) -> String {
        organization_value(&self.light, self.org_light.as_deref())
    }

    fn dark_value(&self) -> Option<String> {
        self.dark
            .as_deref()
            .map(|value| organization_value(value, self.org_dark.as_deref()))
    }
}

fn organization_value(value: &str, organization_property: Option<&str>) -> String {
    match organization_property {
        Some(property) => format!("var(--{property}, {value})"),
        None => value.to_owned(),
    }
}

pub fn sync(check: bool) -> Result<bool, Failure> {
    let back = back_dir();
    let tokens = read_tokens(&back.join(TOKENS_FILE))?;
    let generated = vec![
        Generated {
            path: back.join(WEB_TOKENS_CSS),
            content: tokens_css(&tokens),
        },
        Generated {
            path: back.join(WEB_TOKENS_TS),
            content: tokens_typescript(&tokens)?,
        },
        Generated {
            path: back.join(TOKENS_DOC),
            content: tokens_markdown(&tokens),
        },
    ];
    if check {
        let outdated: Vec<_> = generated
            .iter()
            .filter(|file| fs::read_to_string(&file.path).ok().as_deref() != Some(&file.content))
            .collect();
        if outdated.is_empty() {
            println!("the token files match {TOKENS_FILE}");
            return Ok(true);
        }
        for file in outdated {
            eprintln!("outdated: {}", file.path.display());
        }
        eprintln!("run `make modules` (cargo run -p xtask -- themes sync)");
        return Ok(false);
    }
    for file in &generated {
        write_if_changed(&file.path, &file.content)?;
    }
    println!("{} tokens generated from {TOKENS_FILE}", tokens.len());
    Ok(true)
}

fn read_tokens(path: &Path) -> Result<Vec<Token>, Failure> {
    let file: TokenFile = serde_json::from_str(&fs::read_to_string(path)?)?;
    let mut seen = std::collections::BTreeSet::new();
    for token in &file.tokens {
        if !seen.insert(token.name.as_str()) {
            return Err(format!("duplicate token {}", token.name).into());
        }
    }
    Ok(file.tokens)
}

fn tokens_css(tokens: &[Token]) -> String {
    let mut css = String::from(":root {\n  color-scheme: light;\n");
    for token in tokens {
        css.push_str(&format!("  --{}: {};\n", token.name, token.light_value()));
    }
    css.push_str("}\n\n[data-theme='dark'] {\n  color-scheme: dark;\n");
    for token in tokens {
        if let Some(value) = token.dark_value() {
            css.push_str(&format!("  --{}: {};\n", token.name, value));
        }
    }
    css.push_str("}\n");
    css
}

fn tokens_typescript(tokens: &[Token]) -> Result<String, Failure> {
    let mut source = String::from(
        "export type TokenKind = 'color' | 'length' | 'shadow' | 'duration' | 'font'\n\n\
         export type TokenGroup = 'color' | 'shape' | 'size' | 'depth' | 'type' | 'motion'\n\n\
         export interface TokenDefinition {\n  name: string\n  kind: TokenKind\n  group: TokenGroup\n  light: string\n  dark?: string\n}\n\n\
         export const TOKENS: readonly TokenDefinition[] = [\n",
    );
    for token in tokens {
        source.push_str(&format!(
            "  {{ name: {}, kind: {}, group: {}, light: {}",
            quoted(&token.name)?,
            quoted(&token.kind)?,
            quoted(&token.group)?,
            quoted(&token.light)?,
        ));
        if let Some(dark) = &token.dark {
            source.push_str(&format!(", dark: {}", quoted(dark)?));
        }
        source.push_str(" },\n");
    }
    source.push_str("]\n");
    Ok(source)
}

fn quoted(value: &str) -> Result<String, Failure> {
    let escaped = value.replace('\\', "\\\\").replace('\'', "\\'");
    Ok(format!("'{escaped}'"))
}

fn tokens_markdown(tokens: &[Token]) -> String {
    let mut markdown = String::from(
        "# Design tokens\n\nGenerated from `back/modules/wasm_runtime/tokens.json` by `make modules`. Do not edit by hand.\n\n\
         | Token | Group | Kind | Default | Dark |\n| --- | --- | --- | --- | --- |\n",
    );
    for token in tokens {
        markdown.push_str(&format!(
            "| `--{}` | {} | {} | `{}` | {} |\n",
            token.name,
            token.group,
            token.kind,
            token.light_value(),
            token
                .dark_value()
                .map_or_else(|| "same".to_owned(), |value| format!("`{value}`")),
        ));
    }
    markdown
}
