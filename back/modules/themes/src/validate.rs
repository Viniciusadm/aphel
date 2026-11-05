use std::collections::HashSet;
use std::sync::LazyLock;

use aphel_sdk::platform::application::files::sha256_hex;
use aphel_sdk::platform::modules::builder::APHEL_VERSION;
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use regex::Regex;
use serde::Serialize;
use serde_json::Value;

use crate::package::{FORMAT, FORMAT_VERSION, FontFile, ThemePackage, TokenSets};
use crate::tokens::{definition, render_value};

pub(crate) const MAX_FONTS: usize = 6;
pub(crate) const MAX_FONT_BYTES: usize = 400 * 1024;
pub(crate) const MAX_PACKAGE_BYTES: usize = 2 * 1024 * 1024;
const WOFF2_MAGIC: &[u8] = b"wOF2";
const RESERVED_KEYS: [&str; 4] = ["validate", "activation", "default", "new"];

static THEME_KEY: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[a-z0-9]+(?:[._-][a-z0-9]+)*$").expect("valid regex"));
static MIN_VERSION: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\d+\.\d+\.\d+$").expect("valid regex"));
static FONT_FAMILY: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[A-Za-z0-9][A-Za-z0-9 _-]{0,63}$").expect("valid regex"));

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct Issue {
    pub(crate) path: String,
    pub(crate) message: String,
}

impl Issue {
    fn new(path: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct DecodedFont {
    pub(crate) family: String,
    pub(crate) weight: u16,
    pub(crate) style: String,
    pub(crate) sha256: String,
    pub(crate) bytes: Vec<u8>,
}

#[derive(Debug)]
pub(crate) struct Validation {
    pub(crate) issues: Vec<Issue>,
    pub(crate) warnings: Vec<Issue>,
    pub(crate) fonts: Vec<DecodedFont>,
    pub(crate) tokens: TokenSets,
}

pub(crate) fn validate(package: &ThemePackage) -> Validation {
    let mut issues = Vec::new();
    let mut warnings = Vec::new();
    check_identity(package, &mut issues);
    let fonts = decode_fonts(&package.fonts, &mut issues);
    let families: HashSet<String> = fonts.iter().map(|font| font.family.clone()).collect();
    let tokens = TokenSets {
        shared: clean_set(
            "shared",
            &package.tokens.shared,
            &families,
            &mut issues,
            &mut warnings,
        ),
        light: clean_set(
            "light",
            &package.tokens.light,
            &families,
            &mut issues,
            &mut warnings,
        ),
        dark: clean_set(
            "dark",
            &package.tokens.dark,
            &families,
            &mut issues,
            &mut warnings,
        ),
    };
    Validation {
        issues,
        warnings,
        fonts,
        tokens,
    }
}

fn check_identity(package: &ThemePackage, issues: &mut Vec<Issue>) {
    if package.format != FORMAT {
        issues.push(Issue::new("format", format!("must be {FORMAT}")));
    }
    if package.format_version != FORMAT_VERSION {
        issues.push(Issue::new(
            "format_version",
            format!("only version {FORMAT_VERSION} is supported"),
        ));
    }
    if RESERVED_KEYS.contains(&package.id.as_str()) {
        issues.push(Issue::new("id", "is reserved"));
    }
    if package.id.len() > 64 || !THEME_KEY.is_match(&package.id) {
        issues.push(Issue::new(
            "id",
            "must use lowercase letters, digits, `.`, `_` and `-`, up to 64 characters",
        ));
    }
    for (path, value, limit) in [
        ("name", package.name.as_str(), 140),
        ("author", package.author.as_str(), 140),
        ("license", package.license.as_str(), 64),
    ] {
        let length = value.trim().chars().count();
        if length == 0 || length > limit {
            issues.push(Issue::new(
                path,
                format!("must have 1 to {limit} characters"),
            ));
        }
    }
    if package
        .description
        .as_deref()
        .is_some_and(|description| description.chars().count() > 2000)
    {
        issues.push(Issue::new(
            "description",
            "must have at most 2000 characters",
        ));
    }
    if semver::Version::parse(&package.version).is_err() {
        issues.push(Issue::new("version", "must be a semantic version"));
    }
    if let Some(minimum) = &package.min_aphel_version {
        check_minimum_version(minimum, issues);
    }
}

fn check_minimum_version(minimum: &str, issues: &mut Vec<Issue>) {
    let parsed = MIN_VERSION
        .is_match(minimum)
        .then(|| semver::Version::parse(minimum).ok())
        .flatten();
    match (parsed, semver::Version::parse(APHEL_VERSION)) {
        (None, _) => issues.push(Issue::new("min_aphel_version", "must look like 0.1.0")),
        (Some(required), Ok(current)) if required > current => issues.push(Issue::new(
            "min_aphel_version",
            format!("requires Aphel {required} or newer"),
        )),
        _ => {}
    }
}

fn decode_fonts(files: &[FontFile], issues: &mut Vec<Issue>) -> Vec<DecodedFont> {
    if files.len() > MAX_FONTS {
        issues.push(Issue::new(
            "fonts",
            format!("at most {MAX_FONTS} font files"),
        ));
        return Vec::new();
    }
    let mut decoded = Vec::new();
    let mut faces = HashSet::new();
    let mut total = 0usize;
    for (position, file) in files.iter().enumerate() {
        let path = format!("fonts[{position}]");
        let Some(font) = decode_font(file, &path, issues) else {
            continue;
        };
        if !faces.insert((font.family.clone(), font.weight, font.style.clone())) {
            issues.push(Issue::new(&path, "duplicate family, weight and style"));
            continue;
        }
        total += font.bytes.len();
        decoded.push(font);
    }
    if total > MAX_PACKAGE_BYTES {
        issues.push(Issue::new(
            "fonts",
            "the fonts are larger than 2 MiB together",
        ));
    }
    decoded
}

fn decode_font(file: &FontFile, path: &str, issues: &mut Vec<Issue>) -> Option<DecodedFont> {
    let before = issues.len();
    if !FONT_FAMILY.is_match(&file.family) {
        issues.push(Issue::new(
            format!("{path}.family"),
            "font names use letters, digits, spaces, `_` and `-`",
        ));
    }
    if !(100..=900).contains(&file.weight) {
        issues.push(Issue::new(
            format!("{path}.weight"),
            "must be between 100 and 900",
        ));
    }
    if !matches!(file.style.as_str(), "normal" | "italic") {
        issues.push(Issue::new(
            format!("{path}.style"),
            "must be normal or italic",
        ));
    }
    if file.format != "woff2" {
        issues.push(Issue::new(
            format!("{path}.format"),
            "only woff2 is supported",
        ));
    }
    let bytes = STANDARD.decode(file.data.trim()).ok();
    match &bytes {
        None => issues.push(Issue::new(format!("{path}.data"), "must be base64")),
        Some(bytes) if bytes.len() > MAX_FONT_BYTES => issues.push(Issue::new(
            format!("{path}.data"),
            format!("must have at most {} KB", MAX_FONT_BYTES / 1024),
        )),
        Some(bytes) if !bytes.starts_with(WOFF2_MAGIC) => {
            issues.push(Issue::new(format!("{path}.data"), "is not a woff2 file"));
        }
        Some(_) => {}
    }
    let bytes = bytes?;
    (issues.len() == before).then(|| DecodedFont {
        family: file.family.clone(),
        weight: file.weight,
        style: file.style.clone(),
        sha256: sha256_hex(&bytes),
        bytes,
    })
}

fn clean_set(
    set: &str,
    values: &std::collections::BTreeMap<String, Value>,
    families: &HashSet<String>,
    issues: &mut Vec<Issue>,
    warnings: &mut Vec<Issue>,
) -> std::collections::BTreeMap<String, Value> {
    let mut cleaned = std::collections::BTreeMap::new();
    for (name, value) in values {
        let path = format!("tokens.{set}.{name}");
        let Some(token) = definition(name) else {
            warnings.push(Issue::new(path, "unknown token, ignored"));
            continue;
        };
        match render_value(token, value, families) {
            Ok(_) => {
                cleaned.insert(name.clone(), value.clone());
            }
            Err(message) => issues.push(Issue::new(path, message)),
        }
    }
    cleaned
}
