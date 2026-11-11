use std::collections::{BTreeMap, BTreeSet};

use aphel_kernel::errors::AppError;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const DESCRIPTOR: aphel_contract::ContractDescriptor = aphel_contract::ContractDescriptor {
    contract: aphel_contract::contract!("site", "1.0.0"),
    entries: &[],
};

pub const LAYOUT_TEMPLATE: &str = "layout.html";
pub const MAX_PER_PAGE: u64 = 50;
pub const ALWAYS_PUBLIC_FIELDS: &[&str] = &["id", "created_at", "updated_at"];
const MAX_SEGMENTS: usize = 8;
const MAX_KEY_LENGTH: usize = 40;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SiteBundle {
    pub owner: String,
    pub revision: String,
    pub pages: Vec<SitePage>,
    pub exposures: Vec<SiteExposure>,
    pub templates: Vec<SiteTemplate>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SitePage {
    pub key: String,
    pub path: String,
    pub template: String,
    pub source: PageSource,
    pub title: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PageSource {
    Static,
    Record {
        entity: String,
        match_field: String,
    },
    List {
        entity: String,
        sort: Option<String>,
        per_page: u64,
    },
}

impl PageSource {
    pub fn entity(&self) -> Option<&str> {
        match self {
            Self::Static => None,
            Self::Record { entity, .. } | Self::List { entity, .. } => Some(entity),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SiteExposure {
    pub entity: String,
    pub fields: Vec<String>,
    pub conditions: BTreeMap<String, Value>,
}

impl SiteExposure {
    pub fn exposes(&self, field: &str) -> bool {
        ALWAYS_PUBLIC_FIELDS.contains(&field) || self.fields.iter().any(|exposed| exposed == field)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SiteTemplate {
    pub name: String,
    pub source: String,
}

#[async_trait]
pub trait SiteContentSource: Send + Sync {
    async fn bundles(&self) -> Result<Vec<SiteBundle>, AppError>;
}

pub fn template_name(owner: &str, path: &str) -> String {
    format!("{owner}/{path}")
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Segment {
    Literal(String),
    Param(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathPattern {
    segments: Vec<Segment>,
}

fn is_identifier(value: &str) -> bool {
    let mut characters = value.chars();
    characters
        .next()
        .is_some_and(|first| first.is_ascii_lowercase() || first == '_')
        && characters.all(|character| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || character == '_'
        })
}

fn is_literal(value: &str) -> bool {
    !value.starts_with('_')
        && value.chars().all(|character| {
            character.is_ascii_lowercase()
                || character.is_ascii_digit()
                || matches!(character, '-' | '_' | '.')
        })
}

impl PathPattern {
    pub fn parse(path: &str) -> Result<Self, String> {
        let Some(rest) = path.strip_prefix('/') else {
            return Err(format!("{path}: must start with /"));
        };
        let rest = rest.trim_end_matches('/');
        let mut segments = Vec::new();
        if !rest.is_empty() {
            for part in rest.split('/') {
                let segment = match part
                    .strip_prefix('{')
                    .and_then(|inner| inner.strip_suffix('}'))
                {
                    Some(name) if is_identifier(name) => Segment::Param(name.to_owned()),
                    Some(_) => {
                        return Err(format!(
                            "{path}: parameters look like {{slug}} (lowercase letters, digits, _)"
                        ));
                    }
                    None if !part.is_empty() && is_literal(part) => {
                        Segment::Literal(part.to_owned())
                    }
                    None => {
                        return Err(format!(
                            "{path}: segments use lowercase letters, digits, - _ . and cannot start with _"
                        ));
                    }
                };
                segments.push(segment);
            }
        }
        if segments.len() > MAX_SEGMENTS {
            return Err(format!("{path}: at most {MAX_SEGMENTS} segments"));
        }
        let pattern = Self { segments };
        let params = pattern.params();
        if params.iter().collect::<BTreeSet<_>>().len() != params.len() {
            return Err(format!("{path}: repeats a parameter"));
        }
        Ok(pattern)
    }

    pub fn params(&self) -> Vec<&str> {
        self.segments
            .iter()
            .filter_map(|segment| match segment {
                Segment::Param(name) => Some(name.as_str()),
                Segment::Literal(_) => None,
            })
            .collect()
    }

    pub fn shape(&self) -> String {
        let parts: Vec<&str> = self
            .segments
            .iter()
            .map(|segment| match segment {
                Segment::Literal(literal) => literal.as_str(),
                Segment::Param(_) => "{}",
            })
            .collect();
        format!("/{}", parts.join("/"))
    }

    pub fn specificity(&self) -> Vec<u8> {
        self.segments
            .iter()
            .map(|segment| match segment {
                Segment::Literal(_) => 2,
                Segment::Param(_) => 1,
            })
            .collect()
    }

    pub fn matches(&self, path: &str) -> Option<BTreeMap<String, String>> {
        let rest = path.strip_prefix('/')?.trim_end_matches('/');
        let parts: Vec<&str> = if rest.is_empty() {
            Vec::new()
        } else {
            rest.split('/').collect()
        };
        if parts.len() != self.segments.len() {
            return None;
        }
        let mut params = BTreeMap::new();
        for (segment, part) in self.segments.iter().zip(parts) {
            match segment {
                Segment::Literal(literal) if literal == part => {}
                Segment::Literal(_) => return None,
                Segment::Param(_) if part.is_empty() => return None,
                Segment::Param(name) => {
                    params.insert(name.clone(), part.to_owned());
                }
            }
        }
        Some(params)
    }

    pub fn build(&self, params: &BTreeMap<String, String>) -> Option<String> {
        let mut parts = Vec::with_capacity(self.segments.len());
        for segment in &self.segments {
            match segment {
                Segment::Literal(literal) => parts.push(literal.clone()),
                Segment::Param(name) => {
                    let value = params.get(name).filter(|value| !value.is_empty())?;
                    parts.push(encode_segment(value));
                }
            }
        }
        Some(format!("/{}", parts.join("/")))
    }
}

fn encode_segment(value: &str) -> String {
    value
        .bytes()
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (byte as char).to_string()
            }
            other => format!("%{other:02X}"),
        })
        .collect()
}

pub fn decode_segment(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        let hex = bytes
            .get(index + 1..index + 3)
            .and_then(|hex| std::str::from_utf8(hex).ok())
            .and_then(|hex| u8::from_str_radix(hex, 16).ok());
        match (bytes[index], hex) {
            (b'%', Some(byte)) => {
                decoded.push(byte);
                index += 3;
            }
            (byte, _) => {
                decoded.push(byte);
                index += 1;
            }
        }
    }
    String::from_utf8_lossy(&decoded).into_owned()
}

fn sort_fields(sort: &str) -> Vec<&str> {
    sort.split(',')
        .map(|item| item.trim().trim_start_matches(['-', '+']))
        .filter(|item| !item.is_empty())
        .collect()
}

pub fn check_bundle(bundle: &SiteBundle) -> Vec<String> {
    let mut problems = Vec::new();
    let mut keys = BTreeSet::new();
    let mut shapes = BTreeMap::new();
    let templates: BTreeSet<&str> = bundle
        .templates
        .iter()
        .map(|template| template.name.as_str())
        .collect();
    let mut exposed = BTreeSet::new();
    for exposure in &bundle.exposures {
        if !exposed.insert(exposure.entity.as_str()) {
            problems.push(format!(
                "site.public: {} is declared more than once",
                exposure.entity
            ));
        }
        for field in exposure.conditions.keys() {
            if !exposure.exposes(field) {
                problems.push(format!(
                    "site.public {}: the condition field {field} must also be listed in fields",
                    exposure.entity
                ));
            }
        }
    }
    for page in &bundle.pages {
        let label = format!("site.pages {}", page.key);
        if page.key.len() > MAX_KEY_LENGTH || !is_identifier(&page.key) {
            problems.push(format!(
                "{label}: the key uses lowercase letters, digits and _"
            ));
        }
        if !keys.insert(page.key.as_str()) {
            problems.push(format!("{label}: the key is repeated"));
        }
        let pattern = match PathPattern::parse(&page.path) {
            Ok(pattern) => pattern,
            Err(problem) => {
                problems.push(format!("{label}: {problem}"));
                continue;
            }
        };
        if let Some(other) = shapes.insert(pattern.shape(), page.key.as_str()) {
            problems.push(format!("{label}: the path overlaps page {other}"));
        }
        if !templates.contains(page.template.as_str()) {
            problems.push(format!(
                "{label}: the template {} is not in the package",
                page.template
            ));
        }
        let exposure = page.source.entity().map(|entity| {
            bundle
                .exposures
                .iter()
                .find(|exposure| exposure.entity == entity)
        });
        match (&page.source, exposure) {
            (PageSource::Static, _) => {
                if !pattern.params().is_empty() {
                    problems.push(format!("{label}: a page without entity has no parameters"));
                }
            }
            (_, Some(None)) => problems.push(format!(
                "{label}: the entity {} is not listed in site.public",
                page.source.entity().unwrap_or_default()
            )),
            (PageSource::Record { match_field, .. }, Some(Some(exposure))) => {
                if pattern.params() != [match_field.as_str()] {
                    problems.push(format!(
                        "{label}: the path must have exactly the parameter {{{match_field}}}"
                    ));
                }
                if !exposure.exposes(match_field) {
                    problems.push(format!("{label}: the field {match_field} is not public"));
                }
            }
            (PageSource::List { sort, per_page, .. }, Some(Some(exposure))) => {
                if !pattern.params().is_empty() {
                    problems.push(format!("{label}: a list page has no parameters"));
                }
                if *per_page == 0 || *per_page > MAX_PER_PAGE {
                    problems.push(format!(
                        "{label}: per_page must be between 1 and {MAX_PER_PAGE}"
                    ));
                }
                for field in sort.as_deref().map(sort_fields).unwrap_or_default() {
                    if !exposure.exposes(field) {
                        problems.push(format!("{label}: the sort field {field} is not public"));
                    }
                }
            }
            (_, None) => {}
        }
    }
    problems.extend(check_templates(bundle));
    problems
}

pub fn join_template_path(name: &str, parent: &str) -> String {
    if name == LAYOUT_TEMPLATE || (name.contains('/') && !name.starts_with("site/")) {
        return name.to_owned();
    }
    match parent.split_once('/') {
        Some((owner, _)) if parent != LAYOUT_TEMPLATE => template_name(owner, name),
        _ => name.to_owned(),
    }
}

fn check_templates(bundle: &SiteBundle) -> Vec<String> {
    let mut environment = minijinja::Environment::new();
    environment.set_path_join_callback(|name, parent| join_template_path(name, parent).into());
    let mut problems = Vec::new();
    for template in &bundle.templates {
        if let Err(error) = environment.add_template_owned(
            template_name(&bundle.owner, &template.name),
            template.source.clone(),
        ) {
            problems.push(format!("{}: {error}", template.name));
        }
    }
    problems
}
