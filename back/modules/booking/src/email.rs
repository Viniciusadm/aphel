use chrono::{DateTime, Utc};
use chrono_tz::Tz;

use crate::ics::{Invite, InviteMethod, render};
use crate::validation::parse_timezone;
use crate::{Booking, HostProfile, LocationKind, MeetingType};
use aphel_kernel::application::mail::MailLocale;
use aphel_kernel::application::mail::templates::escape_html;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BookingEmailKind {
    Booked,
    Rescheduled,
    Canceled,
}

impl BookingEmailKind {
    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "booked" => Some(Self::Booked),
            "rescheduled" => Some(Self::Rescheduled),
            "canceled" => Some(Self::Canceled),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Recipient {
    Guest,
    Host,
}

impl Recipient {
    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "guest" => Some(Self::Guest),
            "host" => Some(Self::Host),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BookingEmail {
    pub(crate) to: String,
    pub(crate) subject: String,
    pub(crate) html: String,
    pub(crate) text: String,
    pub(crate) calendar: String,
    pub(crate) method: InviteMethod,
}

pub(crate) struct BookingEmailContext<'a> {
    pub(crate) kind: BookingEmailKind,
    pub(crate) recipient: Recipient,
    pub(crate) booking: &'a Booking,
    pub(crate) meeting_type: &'a MeetingType,
    pub(crate) host: &'a HostProfile,
    pub(crate) organization_name: &'a str,
    pub(crate) host_timezone: Tz,
    pub(crate) manage_url: &'a str,
    pub(crate) booking_url: &'a str,
    pub(crate) now: DateTime<Utc>,
}

fn location_line(meeting_type: &MeetingType, locale: MailLocale) -> Option<String> {
    let value = meeting_type.location_value.as_deref()?;
    let label = match meeting_type.location_kind {
        LocationKind::Video => locale.pick("Link da reunião", "Meeting link"),
        LocationKind::Address => locale.pick("Endereço", "Address"),
        LocationKind::Phone => locale.pick("Telefone", "Phone"),
        LocationKind::None => locale.pick("Local", "Location"),
    };
    Some(format!("{label}: {value}"))
}

fn when(booking: &Booking, timezone: Tz, locale: MailLocale) -> String {
    let start = booking.starts_at.with_timezone(&timezone);
    let end = booking.ends_at.with_timezone(&timezone);
    let date = start.format(locale.pick("%d/%m/%Y", "%b %-d, %Y"));
    format!(
        "{date}, {}–{} ({})",
        start.format("%H:%M"),
        end.format("%H:%M"),
        timezone.name()
    )
}

pub(crate) fn render_booking_email(context: &BookingEmailContext<'_>) -> BookingEmail {
    let booking = context.booking;
    let (to, display_name, locale, timezone) = match context.recipient {
        Recipient::Guest => (
            booking.guest_email.clone(),
            booking.guest_name.clone(),
            MailLocale::resolve(booking.guest_locale.as_deref(), None),
            booking
                .guest_timezone
                .as_deref()
                .and_then(parse_timezone)
                .unwrap_or(context.host_timezone),
        ),
        Recipient::Host => (
            context.host.email.clone(),
            context.host.name.clone(),
            MailLocale::resolve(context.host.locale.as_deref(), None),
            context.host_timezone,
        ),
    };
    let meeting = &context.meeting_type.name;
    let other = match context.recipient {
        Recipient::Guest => context.host.name.clone(),
        Recipient::Host => format!("{} <{}>", booking.guest_name, booking.guest_email),
    };
    let subject = match (context.kind, locale) {
        (BookingEmailKind::Booked, MailLocale::PtBr) => format!("Reunião confirmada: {meeting}"),
        (BookingEmailKind::Booked, MailLocale::En) => format!("Meeting confirmed: {meeting}"),
        (BookingEmailKind::Rescheduled, MailLocale::PtBr) => {
            format!("Reunião remarcada: {meeting}")
        }
        (BookingEmailKind::Rescheduled, MailLocale::En) => {
            format!("Meeting rescheduled: {meeting}")
        }
        (BookingEmailKind::Canceled, MailLocale::PtBr) => format!("Reunião cancelada: {meeting}"),
        (BookingEmailKind::Canceled, MailLocale::En) => format!("Meeting canceled: {meeting}"),
    };
    let headline = match context.kind {
        BookingEmailKind::Booked => {
            locale.pick("A reunião está confirmada.", "The meeting is confirmed.")
        }
        BookingEmailKind::Rescheduled => {
            locale.pick("A reunião foi remarcada.", "The meeting was rescheduled.")
        }
        BookingEmailKind::Canceled => {
            locale.pick("A reunião foi cancelada.", "The meeting was canceled.")
        }
    };
    let mut lines = vec![
        headline.to_owned(),
        format!("{}: {meeting}", locale.pick("Reunião", "Meeting")),
        format!(
            "{}: {}",
            locale.pick("Quando", "When"),
            when(booking, timezone, locale)
        ),
        format!("{}: {other}", locale.pick("Com", "With")),
    ];
    if context.kind != BookingEmailKind::Canceled
        && let Some(location) = location_line(context.meeting_type, locale)
    {
        lines.push(location);
    }
    if let Some(reason) = booking
        .cancel_reason
        .as_deref()
        .filter(|_| context.kind == BookingEmailKind::Canceled)
    {
        lines.push(format!("{}: {reason}", locale.pick("Motivo", "Reason")));
    }
    if context.recipient == Recipient::Host {
        for question in &context.meeting_type.questions {
            if let Some(serde_json::Value::String(answer)) = booking.answers.get(&question.key) {
                lines.push(format!("{}: {answer}", question.label));
            }
        }
    }
    let (action_label, link) = match (context.recipient, context.kind) {
        (Recipient::Guest, BookingEmailKind::Canceled) => (
            locale.pick("Agendar outro horário", "Book another time"),
            context.booking_url,
        ),
        (Recipient::Guest, _) => (
            locale.pick("Remarcar ou cancelar", "Reschedule or cancel"),
            context.manage_url,
        ),
        (Recipient::Host, _) => ("", ""),
    };
    let greeting = locale.greeting(&display_name);
    let signature = context.organization_name.to_owned();
    let mut text = format!("{greeting}\n\n");
    for line in &lines {
        text.push_str(line);
        text.push('\n');
    }
    if !link.is_empty() {
        text.push_str(&format!("\n{action_label}: {link}\n"));
    }
    text.push_str(&format!("\n{signature}\n"));
    let html_lines: String = lines
        .iter()
        .map(|line| format!("<p style=\"margin:0 0 12px\">{}</p>", escape_html(line)))
        .collect();
    let action = if link.is_empty() {
        String::new()
    } else {
        format!(
            "<p style=\"margin:24px 0\"><a href=\"{link}\" style=\"background:#4f46e5;color:#ffffff;padding:12px 20px;border-radius:6px;text-decoration:none;display:inline-block\">{label}</a></p>",
            link = escape_html(link),
            label = escape_html(action_label),
        )
    };
    let html = format!(
        "<!doctype html><html lang=\"{lang}\"><body style=\"margin:0;padding:24px;background:#f4f4f5;font-family:Arial,Helvetica,sans-serif;color:#18181b\">\
<div style=\"max-width:520px;margin:0 auto;background:#ffffff;border-radius:8px;padding:32px\">\
<p style=\"margin:0 0 16px\">{greeting}</p>{html_lines}{action}\
<p style=\"margin:16px 0 0;font-size:13px;color:#52525b\">{signature}</p>\
</div></body></html>",
        lang = locale.html_lang(),
        greeting = escape_html(&greeting),
        signature = escape_html(&signature),
    );
    let method = if context.kind == BookingEmailKind::Canceled {
        InviteMethod::Cancel
    } else {
        InviteMethod::Request
    };
    let description = lines.join("\n");
    let summary = match context.recipient {
        Recipient::Guest => format!("{meeting} — {}", context.host.name),
        Recipient::Host => format!("{meeting} — {}", booking.guest_name),
    };
    let calendar = render(&Invite {
        method,
        booking_id: booking.id,
        revision: booking.revision,
        summary: &summary,
        description: &description,
        location: context.meeting_type.location_value.as_deref(),
        starts_at: booking.starts_at,
        ends_at: booking.ends_at,
        stamp: context.now,
        organizer_name: &context.host.name,
        organizer_email: &context.host.email,
        attendee_name: &booking.guest_name,
        attendee_email: &booking.guest_email,
    });
    BookingEmail {
        to,
        subject,
        html,
        text,
        calendar,
        method,
    }
}
