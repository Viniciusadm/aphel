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
pub const IMAGE_WIDTHS: &[u32] = &[320, 640, 960, 1280, 1920];
pub const SITEMAP_PATH: &str = "/sitemap.xml";
pub const ROBOTS_PATH: &str = "/robots.txt";
pub const THEME_LAYOUT: &str = "site/theme/layout.html";
pub const THEME_NOT_FOUND: &str = "site/theme/not_found.html";
pub const THEME_ERROR: &str = "site/theme/error.html";
pub const THEME_ASSETS_DIR: &str = "site/theme/assets";
pub const HEAD_PARTIAL: &str = "aphel/head.html";
pub const ISLANDS_DIR: &str = "site/islands";
pub const SITE_API_HEADER: &str = "x-aphel-site";
const MAX_ISLANDS: usize = 30;
const MAX_CSP_SOURCES: usize = 10;
const MAX_MENU_ITEMS: usize = 20;
const MAX_LABEL_LENGTH: usize = 60;
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
    #[serde(default)]
    pub menu: Vec<SiteMenuItem>,
    #[serde(default)]
    pub theme: Option<SiteTheme>,
    #[serde(default)]
    pub islands: Vec<String>,
    #[serde(default)]
    pub csp: SiteCsp,
    #[serde(default)]
    pub account: Vec<SiteMenuItem>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct SiteCsp {
    pub script: Vec<String>,
    pub style: Vec<String>,
    pub img: Vec<String>,
    pub font: Vec<String>,
    pub connect: Vec<String>,
    pub frame: Vec<String>,
}

impl SiteCsp {
    pub fn directives(&self) -> [(&'static str, &[String]); 6] {
        [
            ("script-src", &self.script),
            ("style-src", &self.style),
            ("img-src", &self.img),
            ("font-src", &self.font),
            ("connect-src", &self.connect),
            ("frame-src", &self.frame),
        ]
    }

    pub fn is_empty(&self) -> bool {
        self.directives()
            .iter()
            .all(|(_, sources)| sources.is_empty())
    }
}

pub fn is_csp_source(source: &str) -> bool {
    let Some(host) = source.strip_prefix("https://") else {
        return false;
    };
    let host = host.strip_prefix("*.").unwrap_or(host);
    let (name, port) = match host.rsplit_once(':') {
        Some((name, port)) => (name, Some(port)),
        None => (host, None),
    };
    name.contains('.')
        && name.split('.').all(|label| {
            !label.is_empty()
                && label.chars().all(|character| {
                    character.is_ascii_lowercase() || character.is_ascii_digit() || character == '-'
                })
        })
        && port.is_none_or(|port| port.parse::<u16>().is_ok())
}

pub fn is_island_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= MAX_KEY_LENGTH
        && name
            .chars()
            .next()
            .is_some_and(|first| first.is_ascii_lowercase())
        && name.chars().all(|character| {
            character.is_ascii_lowercase()
                || character.is_ascii_digit()
                || matches!(character, '-' | '_')
        })
}

#[derive(Clone)]
pub struct SiteApiRequest {
    pub owner: String,
    pub method: axum::http::Method,
    pub path: String,
    pub query: Option<String>,
    pub headers: axum::http::HeaderMap,
    pub body: bytes::Bytes,
    pub client: Option<String>,
    pub customer: Option<SiteCustomer>,
}

#[async_trait]
pub trait SiteApiHandler: Send + Sync {
    async fn handle(
        &self,
        state: &aphel_kernel::api::state::AppState,
        request: SiteApiRequest,
    ) -> Result<Option<axum::response::Response>, AppError>;
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SiteMenuItem {
    pub key: String,
    pub label: String,
    pub page: String,
    #[serde(default)]
    pub labels: BTreeMap<String, String>,
}

impl SiteMenuItem {
    pub fn label_for(&self, language: &str) -> &str {
        self.labels
            .iter()
            .find(|(candidate, _)| candidate.eq_ignore_ascii_case(language))
            .map(|(_, label)| label.as_str())
            .unwrap_or(&self.label)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SiteTheme {
    pub name: String,
    pub description: Option<String>,
}

pub fn menu_item_id(owner: &str, key: &str) -> String {
    format!("{owner}.{key}")
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SitePage {
    pub key: String,
    pub path: String,
    pub template: String,
    pub source: PageSource,
    pub title: Option<String>,
    #[serde(default)]
    pub seo: SitePageSeo,
    #[serde(default)]
    pub data: Option<String>,
    #[serde(default)]
    pub requires_customer: bool,
    #[serde(default)]
    pub customer_field: Option<String>,
}

impl SitePage {
    pub fn needs_customer(&self) -> bool {
        self.requires_customer || self.customer_field.is_some()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SiteCustomer {
    pub id: String,
    pub email: String,
    pub name: String,
    pub email_verified: bool,
}

pub const CUSTOMER_RECORD_TYPE: &str = "site.customer";
pub const RESERVED_PAGE_PREFIXES: &[&str] = &["/conta", "/account"];

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SitePageSeo {
    pub description: Option<String>,
    pub image: Option<String>,
    pub noindex: bool,
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
    #[serde(default)]
    pub images: bool,
    #[serde(default)]
    pub translated: Vec<String>,
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

    async fn asset(&self, owner: &str, path: &str) -> Result<Option<(String, Vec<u8>)>, AppError>;

    async fn island(&self, owner: &str, name: &str) -> Result<Option<Vec<u8>>, AppError>;
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
        for field in &exposure.translated {
            if !exposure.exposes(field) {
                problems.push(format!(
                    "site.public {}: the translated field {field} must also be listed in fields",
                    exposure.entity
                ));
            }
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
        if let Some(data) = &page.data {
            match PathPattern::parse(data) {
                Err(problem) => problems.push(format!("{label}: data {problem}")),
                Ok(data_pattern) => {
                    for param in data_pattern.params() {
                        if !pattern.params().contains(&param) {
                            problems.push(format!(
                                "{label}: data uses {{{param}}}, which is not a parameter of the page path"
                            ));
                        }
                    }
                }
            }
        }
        let shape = pattern.shape();
        if RESERVED_PAGE_PREFIXES
            .iter()
            .any(|prefix| shape == *prefix || shape.starts_with(&format!("{prefix}/")))
        {
            problems.push(format!(
                "{label}: {} is reserved for the customer account pages",
                page.path
            ));
        }
        if let Some(field) = &page.customer_field {
            let exposed = page.source.entity().and_then(|entity| {
                bundle
                    .exposures
                    .iter()
                    .find(|exposure| exposure.entity == entity)
            });
            match exposed {
                Some(exposure) if exposure.exposes(field) => {}
                Some(_) => problems.push(format!(
                    "{label}: customer_field {field} must be a public field of the entity"
                )),
                None => problems.push(format!(
                    "{label}: customer_field needs a page with an entity"
                )),
            }
        }
        if [SITEMAP_PATH, ROBOTS_PATH].contains(&pattern.shape().as_str()) {
            problems.push(format!("{label}: {} is reserved for the site", page.path));
        }
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
                if !pattern.params().is_empty() && page.data.is_none() {
                    problems.push(format!(
                        "{label}: a page without entity has parameters only when it declares data"
                    ));
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
    let mut menu_keys = BTreeSet::new();
    if bundle.menu.len() > MAX_MENU_ITEMS {
        problems.push(format!("site.menu: at most {MAX_MENU_ITEMS} items"));
    }
    for item in &bundle.menu {
        let label = format!("site.menu {}", item.key);
        if item.key.len() > MAX_KEY_LENGTH || !is_identifier(&item.key) {
            problems.push(format!(
                "{label}: the key uses lowercase letters, digits and _"
            ));
        }
        if !menu_keys.insert(item.key.as_str()) {
            problems.push(format!("{label}: the key is repeated"));
        }
        if item.label.trim().is_empty() || item.label.chars().count() > MAX_LABEL_LENGTH {
            problems.push(format!(
                "{label}: the label must have between 1 and {MAX_LABEL_LENGTH} characters"
            ));
        }
        match bundle.pages.iter().find(|page| page.key == item.page) {
            None => problems.push(format!("{label}: the page {} does not exist", item.page)),
            Some(page) => {
                if PathPattern::parse(&page.path).is_ok_and(|pattern| !pattern.params().is_empty())
                {
                    problems.push(format!(
                        "{label}: the page {} has parameters and cannot be a menu item",
                        item.page
                    ));
                }
            }
        }
    }
    if bundle.islands.len() > MAX_ISLANDS {
        problems.push(format!("site/islands: at most {MAX_ISLANDS} islands"));
    }
    for island in &bundle.islands {
        if !is_island_name(island) {
            problems.push(format!(
                "site/islands/{island}.js: names use lowercase letters, digits, - and _"
            ));
        }
    }
    for (directive, sources) in bundle.csp.directives() {
        if sources.len() > MAX_CSP_SOURCES {
            problems.push(format!(
                "site.csp {directive}: at most {MAX_CSP_SOURCES} sources"
            ));
        }
        for source in sources {
            if !is_csp_source(source) {
                problems.push(format!(
                    "site.csp {directive}: {source} must be an https origin such as https://js.example.com or https://*.example.com"
                ));
            }
        }
    }
    for item in &bundle.account {
        let label = format!("site.account {}", item.key);
        match bundle.pages.iter().find(|page| page.key == item.page) {
            None => problems.push(format!("{label}: the page {} does not exist", item.page)),
            Some(page) if !page.needs_customer() => problems.push(format!(
                "{label}: the page {} must declare requires_customer or customer_field",
                item.page
            )),
            Some(page) => {
                if PathPattern::parse(&page.path).is_ok_and(|pattern| !pattern.params().is_empty())
                {
                    problems.push(format!(
                        "{label}: the page {} has parameters and cannot be linked",
                        item.page
                    ));
                }
            }
        }
        if item.label.trim().is_empty() || item.label.chars().count() > MAX_LABEL_LENGTH {
            problems.push(format!(
                "{label}: the label must have between 1 and {MAX_LABEL_LENGTH} characters"
            ));
        }
    }
    if let Some(theme) = &bundle.theme {
        if theme.name.trim().is_empty() || theme.name.chars().count() > MAX_LABEL_LENGTH {
            problems.push(format!(
                "site.theme: the name must have between 1 and {MAX_LABEL_LENGTH} characters"
            ));
        }
        if !templates.contains(THEME_LAYOUT) {
            problems.push(format!("site.theme: the package needs {THEME_LAYOUT}"));
        }
    }
    problems.extend(check_templates(bundle));
    problems
}

pub fn join_template_path(name: &str, parent: &str, theme_owner: Option<&str>) -> String {
    if name == LAYOUT_TEMPLATE || (name.contains('/') && !name.starts_with("site/")) {
        return name.to_owned();
    }
    let shared = parent == LAYOUT_TEMPLATE || parent.starts_with("aphel/");
    match (parent.split_once('/'), theme_owner) {
        (_, Some(theme)) if shared => template_name(theme, name),
        (Some((owner, _)), _) if !shared => template_name(owner, name),
        _ => name.to_owned(),
    }
}

fn check_templates(bundle: &SiteBundle) -> Vec<String> {
    let mut environment = minijinja::Environment::new();
    environment
        .set_path_join_callback(|name, parent| join_template_path(name, parent, None).into());
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
