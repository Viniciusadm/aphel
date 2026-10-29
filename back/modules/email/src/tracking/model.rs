use std::collections::HashMap;

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::compose::html::{append_pixel, rewrite_links};
use crate::domain::normalize_address;
use crate::tracking::TrackingToken;
use aphel_kernel::application::files::sha256_hex;
use aphel_kernel::application::secrets::SecretKey;
use aphel_kernel::domain::UserId;
use aphel_kernel::domain::access::{AccessScope, PermissionSet, RoleKey};
use aphel_kernel::errors::AppError;

pub(crate) const TRACKING_KEY_NAME: &str = "email_tracking_key";
pub(crate) const EMAIL_TRACKING_UPDATED: &str = "email_tracking.updated";
pub const EMAIL_TRACKING_OBJECTION_ADDED: &str = "email_tracking.objection_added";
pub(crate) const EMAIL_TRACKING_OBJECTION_REMOVED: &str = "email_tracking.objection_removed";
pub(crate) const EMAIL_TRACKING_OBJECTION_EXISTS: &str = "email_tracking_objection_exists";
pub(crate) const EMAIL_TRACKING_OBJECTION_NOT_FOUND: &str = "email_tracking_objection_not_found";
pub(crate) const EMAIL_TRACKING_LINK_NOT_FOUND: &str = "email_tracking_link_not_found";
pub(crate) const MAX_OBJECTION_REASON_LENGTH: usize = 500;
const MAX_TRACKED_LINKS: usize = 500;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct TrackingSettings {
    pub(crate) opens_enabled: bool,
    pub(crate) clicks_enabled: bool,
    pub(crate) default_on: bool,
}

impl TrackingSettings {
    pub(crate) fn any_enabled(&self) -> bool {
        self.opens_enabled || self.clicks_enabled
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TrackingObjection {
    pub(crate) id: Uuid,
    pub(crate) label: String,
    pub(crate) reason: Option<String>,
    pub(crate) created_by: Option<UserId>,
    pub(crate) created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NewTrackingObjection {
    pub(crate) id: Uuid,
    pub(crate) address_hash: String,
    pub(crate) label: String,
    pub(crate) reason: Option<String>,
    pub(crate) created_by: Option<UserId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TrackingKind {
    Open,
    Click(u16),
}

pub fn address_hash(address: &str) -> String {
    let normalized = normalize_address(address).unwrap_or_else(|_| address.trim().to_lowercase());
    sha256_hex(normalized.as_bytes())
}

pub(crate) fn tracking_scope() -> AccessScope {
    AccessScope::new(
        UserId::system(),
        RoleKey::system(),
        Vec::new(),
        PermissionSet::default(),
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TrackedHtml {
    pub(crate) html: String,
    pub(crate) urls: Vec<String>,
}

pub(crate) fn link_not_found() -> AppError {
    AppError::not_found_code(EMAIL_TRACKING_LINK_NOT_FOUND, "link was not found")
}

pub(crate) fn objection_not_found() -> AppError {
    AppError::not_found_code(
        EMAIL_TRACKING_OBJECTION_NOT_FOUND,
        "tracking objection was not found",
    )
}

pub(crate) fn tracking_key_reference() -> SecretKey {
    SecretKey::organization(TRACKING_KEY_NAME)
}

pub(crate) fn tracked_html(
    original: &str,
    settings: &TrackingSettings,
    base_url: &str,
    key: &[u8],
    email_id: Uuid,
) -> Result<TrackedHtml, AppError> {
    let mut urls: Vec<String> = Vec::new();
    let mut html = original.to_owned();
    if settings.clicks_enabled {
        let mut positions: HashMap<String, u16> = HashMap::new();
        html = rewrite_links(&html, |url| {
            let position = match positions.get(url) {
                Some(position) => *position,
                None => {
                    if urls.len() >= MAX_TRACKED_LINKS {
                        return None;
                    }
                    let position = u16::try_from(urls.len()).ok()?;
                    urls.push(url.to_owned());
                    positions.insert(url.to_owned(), position);
                    position
                }
            };
            TrackingToken {
                email_id,
                kind: TrackingKind::Click(position),
            }
            .url(base_url, key)
            .ok()
        });
    }
    if settings.opens_enabled {
        let pixel = TrackingToken {
            email_id,
            kind: TrackingKind::Open,
        }
        .url(base_url, key)?;
        html = append_pixel(&html, &pixel);
    }
    Ok(TrackedHtml { html, urls })
}
