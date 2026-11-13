use aphel_kernel::errors::FieldViolation;
use aphel_sdk::platform::application::organization_profile::{
    OrganizationConfig, OrganizationConfigExtension,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

const SETTINGS_KEY: &str = "site";
const MAX_TAGLINE_CHARS: usize = 160;
const MAX_LANGUAGES: usize = 5;
const MAX_NAVIGATION_ITEMS: usize = 30;
const MAX_NAVIGATION_CHILDREN: usize = 15;
const MAX_TARGET_CHARS: usize = 200;
pub(crate) const SITE_LANGUAGES: &[&str] = &["pt-BR", "en", "es", "fr", "de", "it"];

pub(crate) fn language_slug(language: &str) -> String {
    language.to_ascii_lowercase()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct SiteSettings {
    pub indexing: bool,
    pub accounts: bool,
    pub theme: Option<String>,
    pub tagline: Option<String>,
    pub navigation: Vec<NavigationItem>,
    pub languages: Vec<String>,
    pub link_people: bool,
    pub people_owner: Option<String>,
    pub home: Option<String>,
    pub scrollbar: String,
    pub header: SiteHeader,
    pub footer: SiteFooter,
    pub translations: BTreeMap<String, SiteTexts>,
}

pub(crate) const HEADER_ALIGNMENTS: &[&str] = &["theme", "left", "center"];
pub(crate) const FOOTER_LAYOUTS: &[&str] = &["theme", "columns", "center"];
pub(crate) const SOCIAL_NETWORKS: &[(&str, &str, &[&str])] = &[
    ("instagram", "Instagram", &["instagram.com"]),
    ("facebook", "Facebook", &["facebook.com", "fb.com"]),
    ("whatsapp", "WhatsApp", &["wa.me", "whatsapp.com"]),
    ("youtube", "YouTube", &["youtube.com", "youtu.be"]),
    ("tiktok", "TikTok", &["tiktok.com"]),
    ("linkedin", "LinkedIn", &["linkedin.com"]),
    ("x", "X", &["x.com", "twitter.com"]),
];
const MAX_FOOTER_TEXT_CHARS: usize = 500;
const MAX_FOOTER_LINKS: usize = 8;
const MAX_SOCIAL_LINKS: usize = 8;
const MAX_ADDRESS_CHARS: usize = 200;
const MAX_EMAIL_CHARS: usize = 200;
const MAX_CTA_LABEL_CHARS: usize = 40;
const MIN_PHONE_DIGITS: usize = 8;
const MAX_PHONE_DIGITS: usize = 15;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct SiteHeader {
    pub align: String,
    pub cta: Option<SiteCta>,
}

impl Default for SiteHeader {
    fn default() -> Self {
        Self {
            align: HEADER_ALIGNMENTS[0].to_owned(),
            cta: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct SiteCta {
    pub label: String,
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct SiteFooter {
    pub text: Option<String>,
    pub contact: SiteContact,
    pub social: Vec<SocialLink>,
    pub links: Vec<SiteLink>,
    pub credit: bool,
    pub layout: String,
}

impl Default for SiteFooter {
    fn default() -> Self {
        Self {
            text: None,
            contact: SiteContact::default(),
            social: Vec::new(),
            links: Vec::new(),
            credit: true,
            layout: FOOTER_LAYOUTS[0].to_owned(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct SiteContact {
    pub phone: Option<String>,
    pub whatsapp: Option<String>,
    pub email: Option<String>,
    pub address: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct SocialLink {
    pub network: String,
    pub url: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct SiteTexts {
    pub tagline: Option<String>,
    pub footer_text: Option<String>,
    pub cta_label: Option<String>,
    pub links: BTreeMap<String, String>,
}

pub(crate) fn filled(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

pub(crate) fn phone_digits(phone: &str) -> String {
    phone.chars().filter(char::is_ascii_digit).collect()
}

fn valid_phone(phone: &str) -> bool {
    let digits = phone_digits(phone).len();
    (MIN_PHONE_DIGITS..=MAX_PHONE_DIGITS).contains(&digits)
        && phone
            .chars()
            .all(|character| character.is_ascii_digit() || " +().-".contains(character))
}

fn valid_email(email: &str) -> bool {
    let Some((local, domain)) = email.split_once('@') else {
        return false;
    };
    email.chars().count() <= MAX_EMAIL_CHARS
        && !local.is_empty()
        && domain.contains('.')
        && !domain.starts_with('.')
        && !domain.ends_with('.')
        && !email.chars().any(|character| {
            character.is_whitespace() || character.is_control() || "<>\"'".contains(character)
        })
        && !domain.contains('@')
}

fn https_host(url: &str) -> Option<String> {
    let rest = url.strip_prefix("https://")?;
    let host = rest.split(['/', '?', '#']).next()?;
    (!host.is_empty() && !host.contains(['@', ':'])).then(|| host.to_ascii_lowercase())
}

pub(crate) fn social_network(
    network: &str,
) -> Option<(&'static str, &'static str, &'static [&'static str])> {
    SOCIAL_NETWORKS
        .iter()
        .copied()
        .find(|(key, _, _)| *key == network)
}

fn valid_social(link: &SocialLink) -> bool {
    let url = link.url.trim();
    let Some((_, _, domains)) = social_network(&link.network) else {
        return false;
    };
    let Some(host) = https_host(url) else {
        return false;
    };
    valid_link_url(url)
        && domains
            .iter()
            .any(|domain| host == *domain || host.ends_with(&format!(".{domain}")))
}

fn too_long(value: Option<&str>, limit: usize) -> bool {
    value.is_some_and(|value| value.chars().count() > limit)
}

pub(crate) const SCROLLBAR_STYLES: &[&str] = &["default", "themed"];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct SiteLink {
    pub id: String,
    pub label: String,
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct NavigationItem {
    pub id: String,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub target: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub children: Vec<NavigationItem>,
}

const MAX_LINKS: usize = MAX_NAVIGATION_ITEMS + MAX_NAVIGATION_ITEMS * MAX_NAVIGATION_CHILDREN;
const MAX_LINK_ID_CHARS: usize = 40;
const MAX_LINK_LABEL_CHARS: usize = 60;
const MAX_LINK_URL_CHARS: usize = 500;

pub(crate) fn valid_link_url(url: &str) -> bool {
    let site_path = url.starts_with('/') && !url.starts_with("//");
    let external = url.starts_with("https://") || url.starts_with("http://");
    (site_path || external)
        && url.len() <= MAX_LINK_URL_CHARS
        && !url
            .chars()
            .any(|character| character.is_whitespace() || character.is_control())
}

fn valid_link(link: &SiteLink) -> bool {
    let label = link.label.trim();
    valid_id(&link.id)
        && !label.is_empty()
        && label.chars().count() <= MAX_LINK_LABEL_CHARS
        && valid_link_url(link.url.trim())
}

fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= MAX_LINK_ID_CHARS
        && id.chars().all(|character| {
            character.is_ascii_lowercase()
                || character.is_ascii_digit()
                || character == '_'
                || character == '-'
        })
}

fn navigation_problem(item: &NavigationItem, depth: usize) -> Option<String> {
    let label = item.label.trim();
    let target = item
        .target
        .as_deref()
        .map(str::trim)
        .filter(|target| !target.is_empty());
    let url = item
        .url
        .as_deref()
        .map(str::trim)
        .filter(|url| !url.is_empty());
    if !valid_id(&item.id) {
        return Some(format!(
            "{} needs an id of lowercase letters, digits, - or _",
            item.id
        ));
    }
    if label.chars().count() > MAX_LINK_LABEL_CHARS {
        return Some(format!(
            "{}: the label has at most {MAX_LINK_LABEL_CHARS} characters",
            item.id
        ));
    }
    match (target, url) {
        (Some(_), Some(_)) => {
            return Some(format!(
                "{}: choose a page or an address, not both",
                item.id
            ));
        }
        (Some(target), None) => {
            if target.len() > MAX_TARGET_CHARS || crate::targets::Target::parse(target).is_none() {
                return Some(format!("{}: unknown page", item.id));
            }
        }
        (None, Some(url)) => {
            if !valid_link_url(url) || label.is_empty() {
                return Some(format!(
                    "{}: needs a label and a site path (/page) or an address starting with https://",
                    item.id
                ));
            }
        }
        (None, None) => {
            if label.is_empty() || item.children.is_empty() {
                return Some(format!(
                    "{}: a group without a page or address needs a label and items",
                    item.id
                ));
            }
        }
    }
    if depth > 0 && !item.children.is_empty() {
        return Some(format!("{}: items nest only one level", item.id));
    }
    if item.children.len() > MAX_NAVIGATION_CHILDREN {
        return Some(format!(
            "{}: at most {MAX_NAVIGATION_CHILDREN} items in a group",
            item.id
        ));
    }
    item.children
        .iter()
        .find_map(|child| navigation_problem(child, depth + 1))
}

pub(crate) fn navigation_items(items: &[NavigationItem]) -> impl Iterator<Item = &NavigationItem> {
    items
        .iter()
        .flat_map(|item| std::iter::once(item).chain(item.children.iter()))
}

impl Default for SiteSettings {
    fn default() -> Self {
        Self {
            indexing: true,
            accounts: false,
            theme: None,
            tagline: None,
            navigation: Vec::new(),
            languages: Vec::new(),
            link_people: false,
            people_owner: None,
            home: None,
            scrollbar: SCROLLBAR_STYLES[0].to_owned(),
            header: SiteHeader::default(),
            footer: SiteFooter::default(),
            translations: BTreeMap::new(),
        }
    }
}

pub(crate) fn site_settings(config: &OrganizationConfig) -> SiteSettings {
    config
        .extensions
        .get(SETTINGS_KEY)
        .cloned()
        .and_then(|value| serde_json::from_value(value).ok())
        .unwrap_or_default()
}

pub(crate) struct SiteConfiguration;

impl OrganizationConfigExtension for SiteConfiguration {
    fn defaults(&self) -> (String, Value) {
        (
            SETTINGS_KEY.to_owned(),
            serde_json::to_value(SiteSettings::default()).unwrap_or(Value::Null),
        )
    }

    fn validate(&self, values: &std::collections::BTreeMap<String, Value>) -> Vec<FieldViolation> {
        let Some(value) = values.get(SETTINGS_KEY) else {
            return Vec::new();
        };
        let settings = match serde_path_to_error::deserialize::<_, SiteSettings>(value) {
            Ok(settings) => settings,
            Err(error) => {
                let path = error.path().to_string();
                let field = match path.as_str() {
                    "." | "" => SETTINGS_KEY.to_owned(),
                    path => format!("{SETTINGS_KEY}.{path}"),
                };
                return vec![FieldViolation::new(
                    field,
                    format!("invalid value: {}", error.inner()),
                )];
            }
        };
        let mut violations = Vec::new();
        if settings
            .tagline
            .as_deref()
            .is_some_and(|tagline| tagline.chars().count() > MAX_TAGLINE_CHARS)
        {
            violations.push(FieldViolation::new(
                "site.tagline",
                format!("must have at most {MAX_TAGLINE_CHARS} characters"),
            ));
        }
        if settings.languages.len() > MAX_LANGUAGES
            || settings
                .languages
                .iter()
                .any(|language| !SITE_LANGUAGES.contains(&language.as_str()))
        {
            violations.push(FieldViolation::new(
                "site.languages",
                format!(
                    "must have at most {MAX_LANGUAGES} of {}",
                    SITE_LANGUAGES.join(", ")
                ),
            ));
        }
        let mut navigation_ids = std::collections::BTreeSet::new();
        let problem = if settings.navigation.len() > MAX_NAVIGATION_ITEMS {
            Some(format!("must have at most {MAX_NAVIGATION_ITEMS} items"))
        } else if let Some(item) = navigation_items(&settings.navigation)
            .find(|item| !navigation_ids.insert(item.id.as_str()))
        {
            Some(format!("{} is repeated", item.id))
        } else {
            settings
                .navigation
                .iter()
                .find_map(|item| navigation_problem(item, 0))
        };
        if let Some(problem) = problem {
            violations.push(FieldViolation::new("site.navigation", problem));
        }
        if !SCROLLBAR_STYLES.contains(&settings.scrollbar.as_str()) {
            violations.push(FieldViolation::new(
                "site.scrollbar",
                format!("must be one of {}", SCROLLBAR_STYLES.join(", ")),
            ));
        }
        if settings.home.as_deref().is_some_and(|home| {
            home.len() > MAX_TARGET_CHARS || crate::targets::Target::parse(home).is_none()
        }) {
            violations.push(FieldViolation::new(
                "site.home",
                "must be a page or record of the site",
            ));
        }
        violations.extend(frame_violations(&settings));
        violations
    }
}

pub(crate) fn frame_violations(settings: &SiteSettings) -> Vec<FieldViolation> {
    let mut violations = Vec::new();
    let mut fail =
        |field: &str, message: String| violations.push(FieldViolation::new(field, message));
    if !HEADER_ALIGNMENTS.contains(&settings.header.align.as_str()) {
        fail(
            "site.header.align",
            format!("must be one of {}", HEADER_ALIGNMENTS.join(", ")),
        );
    }
    if !FOOTER_LAYOUTS.contains(&settings.footer.layout.as_str()) {
        fail(
            "site.footer.layout",
            format!("must be one of {}", FOOTER_LAYOUTS.join(", ")),
        );
    }
    if let Some(cta) = &settings.header.cta {
        let label = cta.label.trim();
        if label.is_empty()
            || label.chars().count() > MAX_CTA_LABEL_CHARS
            || !valid_link_url(cta.url.trim())
        {
            fail(
                "site.header.cta",
                format!(
                    "needs a text of up to {MAX_CTA_LABEL_CHARS} characters and a site path (/page) or an address starting with https://"
                ),
            );
        }
    }
    let footer = &settings.footer;
    if too_long(footer.text.as_deref(), MAX_FOOTER_TEXT_CHARS) {
        fail(
            "site.footer.text",
            format!("must have at most {MAX_FOOTER_TEXT_CHARS} characters"),
        );
    }
    let contact = &footer.contact;
    for (field, value) in [
        ("site.footer.contact.phone", &contact.phone),
        ("site.footer.contact.whatsapp", &contact.whatsapp),
    ] {
        if filled(value.as_deref()).is_some_and(|phone| !valid_phone(phone)) {
            fail(
                field,
                format!(
                    "must have from {MIN_PHONE_DIGITS} to {MAX_PHONE_DIGITS} digits with the country code"
                ),
            );
        }
    }
    if filled(contact.email.as_deref()).is_some_and(|email| !valid_email(email)) {
        fail(
            "site.footer.contact.email",
            "is not a valid e-mail address".to_owned(),
        );
    }
    if too_long(contact.address.as_deref(), MAX_ADDRESS_CHARS) {
        fail(
            "site.footer.contact.address",
            format!("must have at most {MAX_ADDRESS_CHARS} characters"),
        );
    }
    if footer.social.len() > MAX_SOCIAL_LINKS
        || footer.social.iter().any(|link| !valid_social(link))
    {
        fail(
            "site.footer.social",
            format!(
                "must have at most {MAX_SOCIAL_LINKS} links, each an address starting with https:// on the site of its network ({})",
                SOCIAL_NETWORKS
                    .iter()
                    .map(|(key, _, _)| *key)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        );
    }
    let mut link_ids = std::collections::BTreeSet::new();
    if footer.links.len() > MAX_FOOTER_LINKS
        || footer
            .links
            .iter()
            .any(|link| !valid_link(link) || !link_ids.insert(link.id.as_str()))
    {
        fail(
            "site.footer.links",
            format!(
                "must have at most {MAX_FOOTER_LINKS} links with a unique id, a label of up to {MAX_LINK_LABEL_CHARS} characters and a site path (/page) or an address starting with https://"
            ),
        );
    }
    for (language, texts) in &settings.translations {
        if !settings.languages.contains(language) {
            fail(
                "site.translations",
                format!("{language} is not an active site language"),
            );
            continue;
        }
        let field = |name: &str| format!("site.translations.{language}.{name}");
        if too_long(texts.tagline.as_deref(), MAX_TAGLINE_CHARS) {
            fail(
                &field("tagline"),
                format!("must have at most {MAX_TAGLINE_CHARS} characters"),
            );
        }
        if too_long(texts.footer_text.as_deref(), MAX_FOOTER_TEXT_CHARS) {
            fail(
                &field("footer_text"),
                format!("must have at most {MAX_FOOTER_TEXT_CHARS} characters"),
            );
        }
        if too_long(texts.cta_label.as_deref(), MAX_CTA_LABEL_CHARS) {
            fail(
                &field("cta_label"),
                format!("must have at most {MAX_CTA_LABEL_CHARS} characters"),
            );
        }
        if texts.links.len() > MAX_LINKS + MAX_FOOTER_LINKS
            || texts
                .links
                .values()
                .any(|label| label.chars().count() > MAX_LINK_LABEL_CHARS)
        {
            fail(
                &field("links"),
                format!("labels must have at most {MAX_LINK_LABEL_CHARS} characters"),
            );
        }
    }
    violations
}
