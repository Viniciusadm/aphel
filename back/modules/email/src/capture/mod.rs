use std::collections::HashSet;

use serde::Serialize;

use crate::domain::{address_domain, normalize_address};
use crate::sync::{BlockKind, BlockedEntry, CaptureSettings};
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::application::shared::RecordType;

pub(crate) const EMAIL_CAPTURE_SETTINGS_UPDATED: &str = "email_capture_settings.updated";
pub(crate) const EMAIL_BLOCKLIST_ADDED: &str = "email_blocklist.added";
pub(crate) const EMAIL_BLOCKLIST_REMOVED: &str = "email_blocklist.removed";
pub(crate) const EMAIL_BLOCK_NOT_FOUND: &str = "email_block_not_found";
pub(crate) const EMAIL_BLOCK_EXISTS: &str = "email_block_exists";
pub(crate) const EMAIL_BLOCK_RECORD_TYPE: RecordType = RecordType::from_static("email.email_block");
pub(crate) const MAX_BLOCK_REASON_LENGTH: usize = 500;

pub(crate) const PERSONAL_DOMAINS: &[&str] = &[
    "gmail.com",
    "googlemail.com",
    "outlook.com",
    "outlook.com.br",
    "hotmail.com",
    "hotmail.com.br",
    "live.com",
    "msn.com",
    "yahoo.com",
    "yahoo.com.br",
    "ymail.com",
    "icloud.com",
    "me.com",
    "mac.com",
    "aol.com",
    "uol.com.br",
    "bol.com.br",
    "terra.com.br",
    "ig.com.br",
    "globo.com",
    "globomail.com",
    "r7.com",
    "zipmail.com.br",
    "protonmail.com",
    "proton.me",
    "pm.me",
    "gmx.com",
    "gmx.net",
    "mail.com",
    "zoho.com",
    "yandex.com",
    "tutanota.com",
];

pub(crate) fn is_personal_domain(domain: &str) -> bool {
    PERSONAL_DOMAINS.contains(&domain.trim().to_lowercase().as_str())
}

pub(crate) fn normalize_domain(value: &str) -> Result<String, &'static str> {
    let domain = value
        .trim()
        .trim_start_matches('@')
        .trim_start_matches('.')
        .trim_end_matches('.')
        .to_lowercase();
    if domain.is_empty() {
        return Err("must not be empty");
    }
    if domain.len() > 253
        || !domain.contains('.')
        || domain.contains("..")
        || !domain
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '.' | '-'))
    {
        return Err("must be a valid domain, e.g. example.com");
    }
    Ok(domain)
}

pub(crate) fn normalize_block_value(kind: BlockKind, value: &str) -> Result<String, &'static str> {
    match kind {
        BlockKind::Domain => normalize_domain(value),
        BlockKind::Address => normalize_address(value),
    }
}

fn domain_matches(domain: &str, candidates: &HashSet<String>) -> bool {
    let mut current = domain;
    loop {
        if candidates.contains(current) {
            return true;
        }
        match current.split_once('.') {
            Some((_, rest)) if rest.contains('.') => current = rest,
            _ => return false,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct ParticipantFilter {
    internal_addresses: HashSet<String>,
    internal_domains: HashSet<String>,
    blocked_addresses: HashSet<String>,
    blocked_domains: HashSet<String>,
}

impl ParticipantFilter {
    pub(crate) fn new(internal_addresses: Vec<String>, blocklist: &[BlockedEntry]) -> Self {
        let internal_addresses: HashSet<String> = internal_addresses
            .into_iter()
            .filter_map(|address| normalize_address(&address).ok())
            .collect();
        let internal_domains = internal_addresses
            .iter()
            .filter_map(|address| address_domain(address))
            .filter(|domain| !is_personal_domain(domain))
            .map(str::to_owned)
            .collect();
        let mut blocked_addresses = HashSet::new();
        let mut blocked_domains = HashSet::new();
        for entry in blocklist {
            match entry.kind {
                BlockKind::Address => blocked_addresses.insert(entry.value.clone()),
                BlockKind::Domain => blocked_domains.insert(entry.value.clone()),
            };
        }
        Self {
            internal_addresses,
            internal_domains,
            blocked_addresses,
            blocked_domains,
        }
    }

    pub(crate) fn is_internal(&self, address: &str) -> bool {
        self.internal_addresses.contains(address)
            || address_domain(address)
                .is_some_and(|domain| domain_matches(domain, &self.internal_domains))
    }

    pub(crate) fn is_blocked(&self, address: &str) -> bool {
        self.blocked_addresses.contains(address)
            || address_domain(address)
                .is_some_and(|domain| domain_matches(domain, &self.blocked_domains))
    }

    pub(crate) fn is_external(&self, address: &str) -> bool {
        !self.is_internal(address) && !self.is_blocked(address)
    }
}

pub(crate) fn company_name_for_domain(domain: &str) -> String {
    let label = domain
        .split('.')
        .find(|label| !label.is_empty() && *label != "www")
        .unwrap_or(domain);
    let mut characters = label.chars();
    match characters.next() {
        Some(first) => first.to_uppercase().chain(characters).collect(),
        None => domain.to_owned(),
    }
}

pub(crate) fn split_display_name(name: Option<&str>, address: &str) -> (String, Option<String>) {
    let cleaned = name
        .map(|name| name.trim().trim_matches('"').trim())
        .filter(|name| !name.is_empty() && !name.contains('@'));
    match cleaned {
        Some(name) => match name.split_once(char::is_whitespace) {
            Some((first, last)) if !last.trim().is_empty() => {
                (first.to_owned(), Some(last.trim().to_owned()))
            }
            _ => (name.to_owned(), None),
        },
        None => (
            address
                .split('@')
                .next()
                .filter(|local| !local.is_empty())
                .unwrap_or(address)
                .to_owned(),
            None,
        ),
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct UpdateCaptureSettingsCommand {
    pub(crate) create_contacts: Option<bool>,
    pub(crate) create_companies: Option<bool>,
    pub(crate) link_open_deals: Option<bool>,
    pub(crate) initial_history_days: Option<u32>,
    pub(crate) ip: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct AddBlockCommand {
    pub(crate) kind: String,
    pub(crate) value: String,
    pub(crate) reason: Option<String>,
    pub(crate) ip: Option<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct CaptureSnapshot {
    create_contacts: bool,
    create_companies: bool,
    link_open_deals: bool,
    initial_history_days: u32,
}

impl From<&CaptureSettings> for CaptureSnapshot {
    fn from(settings: &CaptureSettings) -> Self {
        Self {
            create_contacts: settings.create_contacts,
            create_companies: settings.create_companies,
            link_open_deals: settings.link_open_deals,
            initial_history_days: settings.initial_history_days,
        }
    }
}

#[derive(Debug, Serialize)]
pub(crate) struct BlockSnapshot<'a> {
    kind: &'static str,
    value: &'a str,
    reason: Option<&'a str>,
}

impl<'a> From<&'a BlockedEntry> for BlockSnapshot<'a> {
    fn from(entry: &'a BlockedEntry) -> Self {
        Self {
            kind: entry.kind.as_str(),
            value: &entry.value,
            reason: entry.reason.as_deref(),
        }
    }
}

pub(crate) fn block_not_found() -> AppError {
    AppError::not_found_code(EMAIL_BLOCK_NOT_FOUND, "blocklist entry was not found")
}
