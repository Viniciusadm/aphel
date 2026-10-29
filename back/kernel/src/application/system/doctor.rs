use std::fmt::Write;

use chrono::{DateTime, SecondsFormat, Utc};

use crate::application::system::model::{InstalledModuleInfo, InstanceHeartbeat, MigrationStatus};

pub const REDACTED: &str = "[redacted]";

const PLAIN_SETTINGS: &[&str] = &[
    "HTTP_HOST",
    "HTTP_PORT",
    "RUN_MIGRATIONS",
    "RUST_LOG",
    "WORKER_CONCURRENCY",
    "WORKER_POLL_INTERVAL_MS",
    "WORKER_JOB_TIMEOUT_SECS",
    "SCHEDULER_INTERVAL_SECS",
    "WORKER_HEARTBEAT_INTERVAL_SECS",
    "HEALTH_REQUIRE_WORKER",
    "HEALTH_WORKER_MAX_SILENCE_SECS",
    "HEALTH_SCHEDULER_MAX_DELAY_SECS",
    "HEALTH_QUEUE_MAX_DELAY_SECS",
    "UPGRADE_MIN_FREE_DISK_MB",
    "WEBHOOKS_ALLOW_PRIVATE_NETWORKS",
    "DEMO_MODE",
];

const URL_SETTINGS: &[&str] = &["DATABASE_URL"];

const PRESENCE_SETTINGS: &[&str] = &[
    "APP_ENCRYPTION_KEY",
    "APP_ENCRYPTION_PREVIOUS_KEYS",
    "METRICS_TOKEN",
];

const SENSITIVE_HINTS: &[&str] = &[
    "PASSWORD",
    "SECRET",
    "TOKEN",
    "KEY",
    "CREDENTIAL",
    "PRIVATE",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettingValue {
    Plain(String),
    Unset,
    Configured,
    NotConfigured,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingLine {
    pub name: String,
    pub value: SettingValue,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CollectedSettings {
    pub lines: Vec<SettingLine>,
    pub sensitive_values: Vec<String>,
}

pub fn collect_settings(
    lookup: impl Fn(&str) -> Option<String>,
    extra_names: &[String],
) -> CollectedSettings {
    let mut collected = CollectedSettings::default();
    let read = |name: &str| lookup(name).filter(|value| !value.trim().is_empty());

    for name in URL_SETTINGS {
        let value = match read(name) {
            Some(raw) => {
                collected.sensitive_values.extend(url_credentials(&raw));
                SettingValue::Plain(sanitize_url(&raw))
            }
            None => SettingValue::Unset,
        };
        collected.lines.push(SettingLine {
            name: (*name).to_owned(),
            value,
        });
    }
    for name in PLAIN_SETTINGS {
        let value = match read(name) {
            Some(raw) if looks_like_url_with_credentials(&raw) => {
                collected.sensitive_values.extend(url_credentials(&raw));
                SettingValue::Plain(sanitize_url(&raw))
            }
            Some(raw) => SettingValue::Plain(raw.trim().to_owned()),
            None => SettingValue::Unset,
        };
        collected.lines.push(SettingLine {
            name: (*name).to_owned(),
            value,
        });
    }
    let mut presence_names: Vec<String> = PRESENCE_SETTINGS
        .iter()
        .map(|name| (*name).to_owned())
        .collect();
    for name in extra_names {
        let known = PRESENCE_SETTINGS.contains(&name.as_str())
            || PLAIN_SETTINGS.contains(&name.as_str())
            || URL_SETTINGS.contains(&name.as_str());
        if !known && is_sensitive_name(name) {
            presence_names.push(name.clone());
        }
    }
    presence_names.sort();
    presence_names.dedup();
    for name in presence_names {
        let value = match read(&name) {
            Some(raw) => {
                collected.sensitive_values.push(raw.trim().to_owned());
                if name == "APP_ENCRYPTION_PREVIOUS_KEYS" {
                    collected.sensitive_values.extend(
                        raw.split(',')
                            .map(str::trim)
                            .filter(|key| !key.is_empty())
                            .map(str::to_owned),
                    );
                }
                SettingValue::Configured
            }
            None => SettingValue::NotConfigured,
        };
        collected.lines.push(SettingLine { name, value });
    }
    collected
        .sensitive_values
        .retain(|value| value.chars().count() >= 3);
    collected
}

pub fn is_sensitive_name(name: &str) -> bool {
    let upper = name.to_ascii_uppercase();
    SENSITIVE_HINTS.iter().any(|hint| upper.contains(hint))
}

fn looks_like_url_with_credentials(value: &str) -> bool {
    value.contains("://") && value.contains('@')
}

pub fn sanitize_url(raw: &str) -> String {
    let trimmed = raw.trim();
    let Some((scheme, rest)) = trimmed.split_once("://") else {
        return if trimmed.contains('@') {
            REDACTED.to_owned()
        } else {
            trimmed.to_owned()
        };
    };
    let without_query = rest.split(['?', '#']).next().unwrap_or_default();
    let (authority, path) = match without_query.find('/') {
        Some(index) => without_query.split_at(index),
        None => (without_query, ""),
    };
    let host = authority
        .rsplit_once('@')
        .map_or(authority, |(_, host)| host);
    let had_credentials = authority.contains('@');
    let had_query = rest.len() != without_query.len();
    let mut sanitized = format!("{scheme}://{host}{path}");
    if had_credentials || had_query {
        sanitized.push_str(" (credentials and parameters removed)");
    }
    sanitized
}

fn url_credentials(raw: &str) -> Vec<String> {
    let mut values = Vec::new();
    let Some((_, rest)) = raw.trim().split_once("://") else {
        return values;
    };
    let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
    if let Some((credentials, _)) = authority.rsplit_once('@') {
        let (user, password) = credentials.split_once(':').unwrap_or((credentials, ""));
        for part in [user, password] {
            if !part.is_empty() {
                values.push(part.to_owned());
                let decoded = percent_decode(part);
                if decoded != part {
                    values.push(decoded);
                }
            }
        }
    }
    if let Some((_, query)) = rest.split_once('?') {
        for pair in query.split('&') {
            if let Some((_, value)) = pair.split_once('=')
                && !value.is_empty()
            {
                values.push(value.to_owned());
            }
        }
    }
    values
}

fn percent_decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%'
            && index + 2 < bytes.len()
            && let Ok(hex) = std::str::from_utf8(&bytes[index + 1..index + 3])
            && let Ok(byte) = u8::from_str_radix(hex, 16)
        {
            decoded.push(byte);
            index += 3;
            continue;
        }
        decoded.push(bytes[index]);
        index += 1;
    }
    String::from_utf8_lossy(&decoded).into_owned()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DoctorStatus {
    Ok,
    Warning,
    Fail,
    Skipped,
}

impl DoctorStatus {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Warning => "warning",
            Self::Fail => "FAIL",
            Self::Skipped => "skipped",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DoctorCheck {
    pub name: String,
    pub status: DoctorStatus,
    pub detail: String,
}

impl DoctorCheck {
    pub fn new(name: impl Into<String>, status: DoctorStatus, detail: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            status,
            detail: detail.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DoctorReport {
    pub generated_at: DateTime<Utc>,
    pub version: String,
    pub os: String,
    pub arch: String,
    pub mysql_version: Option<String>,
    pub settings: Vec<SettingLine>,
    pub checks: Vec<DoctorCheck>,
    pub migrations: Option<MigrationStatus>,
    pub modules: Vec<InstalledModuleInfo>,
    pub heartbeats: Vec<InstanceHeartbeat>,
}

pub fn render_markdown(report: &DoctorReport, sensitive_values: &[String]) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "# Aphel doctor report\n");
    let _ = writeln!(out, "- Generated at: {}", timestamp(report.generated_at));
    let _ = writeln!(out, "- Aphel version: {}", report.version);
    let _ = writeln!(out, "- OS / architecture: {} / {}", report.os, report.arch);
    let _ = writeln!(
        out,
        "- MySQL server: {}",
        report.mysql_version.as_deref().unwrap_or("unknown")
    );

    let _ = writeln!(out, "\n## Services\n");
    let _ = writeln!(out, "| Check | Status | Detail |");
    let _ = writeln!(out, "| --- | --- | --- |");
    for check in &report.checks {
        let _ = writeln!(
            out,
            "| {} | {} | {} |",
            check.name,
            check.status.label(),
            table_cell(&check.detail)
        );
    }

    let _ = writeln!(out, "\n## Configuration\n");
    let _ = writeln!(
        out,
        "Only non-secret settings are shown verbatim; secrets appear as configured / not configured.\n"
    );
    let _ = writeln!(out, "| Variable | Value |");
    let _ = writeln!(out, "| --- | --- |");
    for line in &report.settings {
        let value = match &line.value {
            SettingValue::Plain(value) => format!("`{}`", table_cell(value)),
            SettingValue::Unset => "(default)".to_owned(),
            SettingValue::Configured => "configured".to_owned(),
            SettingValue::NotConfigured => "not configured".to_owned(),
        };
        let _ = writeln!(out, "| {} | {} |", line.name, value);
    }

    let _ = writeln!(out, "\n## Migrations\n");
    match &report.migrations {
        None => {
            let _ = writeln!(out, "Not available (database unreachable).");
        }
        Some(status) => {
            let core = &status.core;
            let _ = writeln!(
                out,
                "- Core: {} applied of {} known, latest applied {}, {} pending",
                core.applied,
                core.known,
                core.latest_applied
                    .map_or_else(|| "none".to_owned(), |version| version.to_string()),
                core.pending.len()
            );
            for entry in &core.pending {
                let _ = writeln!(out, "  - pending {} {}", entry.version, entry.description);
            }
            for problem in status.blocking_problems() {
                let _ = writeln!(out, "  - problem: {problem}");
            }
            for module in &status.modules {
                let _ = writeln!(
                    out,
                    "- Module `{}`: {} applied, {} pending",
                    module.key,
                    module.applied,
                    module.pending.len()
                );
            }
        }
    }

    let _ = writeln!(out, "\n## Modules (compiled in)\n");
    if report.modules.is_empty() {
        let _ = writeln!(out, "None.");
    } else {
        let _ = writeln!(out, "| Key | Version | Compatibility | Enabled |");
        let _ = writeln!(out, "| --- | --- | --- | --- |");
        for module in &report.modules {
            let compatibility = match &module.compatibility_detail {
                Some(detail) => format!("{} ({detail})", module.compatibility),
                None => module.compatibility.clone(),
            };
            let _ = writeln!(
                out,
                "| {} | {} | {} | {} |",
                module.key,
                module.version,
                compatibility,
                if module.enabled { "yes" } else { "no" }
            );
        }
    }

    let _ = writeln!(out, "\n## Workers\n");
    if report.heartbeats.is_empty() {
        let _ = writeln!(out, "No worker heartbeat recorded.");
    } else {
        for heartbeat in &report.heartbeats {
            let _ = writeln!(
                out,
                "- {} {} (version {}), started {}, last seen {}",
                heartbeat.role,
                heartbeat.instance_id,
                heartbeat.version,
                timestamp(heartbeat.started_at),
                timestamp(heartbeat.last_seen_at)
            );
        }
    }

    redact(&out, sensitive_values)
}

pub fn redact(text: &str, sensitive_values: &[String]) -> String {
    let mut values: Vec<&String> = sensitive_values
        .iter()
        .filter(|value| value.chars().count() >= 3)
        .collect();
    values.sort_by_key(|value| std::cmp::Reverse(value.len()));
    let mut redacted = text.to_owned();
    for value in values {
        redacted = if value.chars().count() >= STRONG_SECRET_MIN_CHARS {
            redacted.replace(value.as_str(), REDACTED)
        } else {
            redact_credential_tokens(&redacted, value)
        };
    }
    redacted
}

const STRONG_SECRET_MIN_CHARS: usize = 8;

fn redact_credential_tokens(text: &str, value: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut cursor = 0;
    for (start, _) in text.match_indices(value) {
        if start < cursor {
            continue;
        }
        let end = start + value.len();
        let before = text[..start].chars().next_back();
        let after = text[end..].chars().next();
        let isolated = !before.is_some_and(is_token_char) && !after.is_some_and(is_token_char);
        let credential_context = matches!(before, Some(':' | '=')) || after == Some('@');
        if isolated && credential_context {
            out.push_str(&text[cursor..start]);
            out.push_str(REDACTED);
            cursor = end;
        }
    }
    out.push_str(&text[cursor..]);
    out
}

fn is_token_char(character: char) -> bool {
    character.is_alphanumeric() || matches!(character, '_' | '-' | '.')
}

fn table_cell(value: &str) -> String {
    value.replace('|', "\\|").replace('\n', " ")
}

fn timestamp(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(SecondsFormat::Secs, true)
}
