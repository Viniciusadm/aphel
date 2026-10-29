use chrono::{DateTime, Datelike, Utc};
use sha2::{Digest, Sha256};
use uuid::Uuid;

pub const MAX_FILE_NAME_CHARS: usize = 200;
pub const DEFAULT_FILE_NAME: &str = "file";
pub const TEXT_PLAIN: &str = "text/plain";
pub const TEXT_CSV: &str = "text/csv";

const MAX_EXTENSION_CHARS: usize = 16;
const MARKUP_SNIFF_BYTES: usize = 1024;

pub const ALLOWED_MIME_TYPES: &[&str] = &[
    "image/jpeg",
    "image/png",
    "image/gif",
    "image/webp",
    "image/bmp",
    "image/tiff",
    "image/heif",
    "image/avif",
    "application/pdf",
    TEXT_PLAIN,
    TEXT_CSV,
    "application/rtf",
    "application/msword",
    "application/vnd.ms-excel",
    "application/vnd.ms-powerpoint",
    "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
    "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
    "application/vnd.openxmlformats-officedocument.presentationml.presentation",
    "application/vnd.oasis.opendocument.text",
    "application/vnd.oasis.opendocument.spreadsheet",
    "application/vnd.oasis.opendocument.presentation",
    "application/zip",
    "audio/mpeg",
    "video/mp4",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DetectedContent {
    Allowed(String),
    Rejected(String),
    Unknown,
}

pub fn detect_mime_type(content: &[u8], file_name: &str) -> DetectedContent {
    if let Some(kind) = infer::get(content) {
        let mime = kind.mime_type();
        return if is_allowed_mime_type(mime) {
            DetectedContent::Allowed(mime.to_owned())
        } else {
            DetectedContent::Rejected(mime.to_owned())
        };
    }
    if !looks_like_text(content) {
        return DetectedContent::Unknown;
    }
    if starts_with_markup(content) {
        return DetectedContent::Rejected("text/markup".to_owned());
    }
    if extension_of(file_name).is_some_and(|extension| extension.eq_ignore_ascii_case("csv")) {
        DetectedContent::Allowed(TEXT_CSV.to_owned())
    } else {
        DetectedContent::Allowed(TEXT_PLAIN.to_owned())
    }
}

pub fn is_allowed_mime_type(mime: &str) -> bool {
    ALLOWED_MIME_TYPES.contains(&mime)
}

fn looks_like_text(content: &[u8]) -> bool {
    !content.contains(&0) && std::str::from_utf8(content).is_ok()
}

fn starts_with_markup(content: &[u8]) -> bool {
    let head = &content[..content.len().min(MARKUP_SNIFF_BYTES)];
    let text = String::from_utf8_lossy(head);
    let trimmed = text.trim_start_matches('\u{feff}').trim_start();
    trimmed.starts_with('<')
}

pub fn sanitize_file_name(raw: &str) -> String {
    let base = raw.rsplit(['/', '\\']).next().unwrap_or_default();
    let cleaned: String = base
        .chars()
        .filter(|character| !character.is_control() && !is_bidi_control(*character))
        .collect();
    let trimmed = cleaned
        .trim()
        .trim_start_matches('.')
        .trim_end_matches(['.', ' '])
        .trim();
    if trimmed.is_empty() {
        return DEFAULT_FILE_NAME.to_owned();
    }
    truncate_preserving_extension(trimmed)
}

fn is_bidi_control(character: char) -> bool {
    matches!(
        character,
        '\u{200e}' | '\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}'
    )
}

fn truncate_preserving_extension(name: &str) -> String {
    if name.chars().count() <= MAX_FILE_NAME_CHARS {
        return name.to_owned();
    }
    match extension_of(name) {
        Some(extension) if extension.chars().count() <= MAX_EXTENSION_CHARS => {
            let stem_length = MAX_FILE_NAME_CHARS - extension.chars().count() - 1;
            let stem: String = name.chars().take(stem_length).collect();
            format!("{}.{extension}", stem.trim_end())
        }
        _ => name.chars().take(MAX_FILE_NAME_CHARS).collect(),
    }
}

fn extension_of(name: &str) -> Option<&str> {
    name.rsplit_once('.')
        .map(|(_, extension)| extension)
        .filter(|extension| !extension.is_empty())
}

pub fn sha256_hex(content: &[u8]) -> String {
    Sha256::digest(content)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

pub fn storage_key(file_id: Uuid, at: DateTime<Utc>) -> String {
    format!("{:04}/{:02}/{file_id}", at.year(), at.month())
}
