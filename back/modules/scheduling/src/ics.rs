use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InviteMethod {
    Request,
    Cancel,
}

impl InviteMethod {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Request => "REQUEST",
            Self::Cancel => "CANCEL",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Invite<'a> {
    pub(crate) method: InviteMethod,
    pub(crate) booking_id: Uuid,
    pub(crate) revision: u32,
    pub(crate) summary: &'a str,
    pub(crate) description: &'a str,
    pub(crate) location: Option<&'a str>,
    pub(crate) starts_at: DateTime<Utc>,
    pub(crate) ends_at: DateTime<Utc>,
    pub(crate) stamp: DateTime<Utc>,
    pub(crate) organizer_name: &'a str,
    pub(crate) organizer_email: &'a str,
    pub(crate) attendee_name: &'a str,
    pub(crate) attendee_email: &'a str,
}

fn instant(value: DateTime<Utc>) -> String {
    value.format("%Y%m%dT%H%M%SZ").to_string()
}

fn escape(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '\\' => escaped.push_str("\\\\"),
            ';' => escaped.push_str("\\;"),
            ',' => escaped.push_str("\\,"),
            '\n' => escaped.push_str("\\n"),
            '\r' => {}
            other => escaped.push(other),
        }
    }
    escaped
}

fn parameter(value: &str) -> String {
    value
        .chars()
        .filter(|character| !matches!(character, '"' | '\r' | '\n' | ';' | ':' | ','))
        .collect()
}

fn mailto(value: &str) -> String {
    value
        .chars()
        .filter(|character| !character.is_whitespace() && !matches!(character, '"' | ';'))
        .collect()
}

fn fold(line: &str) -> String {
    let mut folded = String::with_capacity(line.len() + 8);
    let mut width = 0;
    for character in line.chars() {
        let size = character.len_utf8();
        if width + size > 73 {
            folded.push_str("\r\n ");
            width = 1;
        }
        folded.push(character);
        width += size;
    }
    folded
}

pub(crate) fn render(invite: &Invite<'_>) -> String {
    let canceled = invite.method == InviteMethod::Cancel;
    let mut lines = vec![
        "BEGIN:VCALENDAR".to_owned(),
        "VERSION:2.0".to_owned(),
        "PRODID:-//Aphel CRM//Scheduling//PT".to_owned(),
        "CALSCALE:GREGORIAN".to_owned(),
        format!("METHOD:{}", invite.method.as_str()),
        "BEGIN:VEVENT".to_owned(),
        format!("UID:{}@aphel-scheduling", invite.booking_id),
        format!("SEQUENCE:{}", invite.revision),
        format!("DTSTAMP:{}", instant(invite.stamp)),
        format!("DTSTART:{}", instant(invite.starts_at)),
        format!("DTEND:{}", instant(invite.ends_at)),
        format!("SUMMARY:{}", escape(invite.summary)),
        format!("DESCRIPTION:{}", escape(invite.description)),
    ];
    if let Some(location) = invite
        .location
        .filter(|location| !location.trim().is_empty())
    {
        lines.push(format!("LOCATION:{}", escape(location)));
    }
    lines.push(format!(
        "ORGANIZER;CN={}:mailto:{}",
        parameter(invite.organizer_name),
        mailto(invite.organizer_email)
    ));
    lines.push(format!(
        "ATTENDEE;CN={};ROLE=REQ-PARTICIPANT;PARTSTAT={};RSVP=FALSE:mailto:{}",
        parameter(invite.attendee_name),
        if canceled { "DECLINED" } else { "ACCEPTED" },
        mailto(invite.attendee_email)
    ));
    lines.push(format!(
        "STATUS:{}",
        if canceled { "CANCELLED" } else { "CONFIRMED" }
    ));
    lines.push("TRANSP:OPAQUE".to_owned());
    lines.push("END:VEVENT".to_owned());
    lines.push("END:VCALENDAR".to_owned());
    let mut body = lines
        .iter()
        .map(|line| fold(line))
        .collect::<Vec<_>>()
        .join("\r\n");
    body.push_str("\r\n");
    body
}
