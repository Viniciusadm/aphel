use serde_json::Value;

use crate::application::mail::{EmailMessage, MailLocale};
use crate::application::notifications::{NotificationView, Recipient};
use crate::domain::notifications::NotificationKind;

const AUTOMATION_TYPE: &str = "automation";

pub fn record_type_label(record_type: &str, locale: MailLocale) -> String {
    if record_type == AUTOMATION_TYPE {
        return locale.pick("Automações", "Automations").to_owned();
    }
    crate::modules::app()
        .record_types()
        .capabilities(record_type)
        .and_then(|capabilities| capabilities.plural_labels_value())
        .map_or_else(
            || record_type.to_owned(),
            |labels| labels.get(locale.html_lang()).to_owned(),
        )
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LocalizedContent {
    RecordType(String),
    DefaultReminder,
    DailyDigest {
        count: usize,
        lines: Vec<String>,
        more: usize,
    },
}

impl LocalizedContent {
    pub fn render(&self, locale: MailLocale) -> (String, Option<String>) {
        match self {
            Self::RecordType(record_type) => (record_type_label(record_type, locale), None),
            Self::DefaultReminder => match locale {
                MailLocale::PtBr => ("Lembrete".to_owned(), None),
                MailLocale::En => ("Reminder".to_owned(), None),
            },
            Self::DailyDigest { count, lines, more } => {
                let title = match (locale, *count) {
                    (MailLocale::PtBr, 1) => "1 tarefa para hoje".to_owned(),
                    (MailLocale::PtBr, count) => format!("{count} tarefas para hoje"),
                    (MailLocale::En, 1) => "1 task for today".to_owned(),
                    (MailLocale::En, count) => format!("{count} tasks for today"),
                };
                let mut body = lines.clone();
                if *more > 0 {
                    body.push(match locale {
                        MailLocale::PtBr => format!("e mais {more}"),
                        MailLocale::En => format!("and {more} more"),
                    });
                }
                (title, Some(body.join("\n")))
            }
        }
    }
}

pub trait NotificationTexts: Send + Sync {
    fn headline(&self, view: &NotificationView, actor: &str, locale: MailLocale) -> Option<String>;

    fn notice(&self, _view: &NotificationView, _locale: MailLocale) -> Option<String> {
        None
    }

    fn action_label(&self, _view: &NotificationView, _locale: MailLocale) -> Option<&'static str> {
        None
    }

    fn default_path(&self, _view: &NotificationView) -> Option<&'static str> {
        None
    }
}

fn contributed<T>(pick: impl Fn(&dyn NotificationTexts) -> Option<T>) -> Option<T> {
    crate::modules::try_app()?
        .all_contributions::<dyn NotificationTexts>()
        .into_iter()
        .find_map(|(_, texts)| pick(texts.as_ref()))
}

pub fn data_text<'a>(view: &'a NotificationView, key: &str) -> Option<&'a str> {
    view.data.get(key).and_then(Value::as_str)
}

pub fn headline(view: &NotificationView, locale: MailLocale) -> String {
    let actor = view
        .actor
        .as_ref()
        .map(|actor| actor.display_name.as_str())
        .filter(|name| !name.trim().is_empty())
        .unwrap_or(locale.pick("Alguém", "Someone"));
    if let Some(headline) = contributed(|texts| texts.headline(view, actor, locale)) {
        return headline;
    }
    match view.kind {
        NotificationKind::DAILY_DIGEST => locale
            .pick(
                "Resumo das suas tarefas de hoje",
                "Summary of your tasks for today",
            )
            .to_owned(),
        NotificationKind::AUTOMATION_MESSAGE | NotificationKind::AUTOMATION_EMAIL => {
            view.title.clone()
        }
        NotificationKind::AUTOMATION_SKIPPED => locale
            .pick(
                "Uma automação foi interrompida pela proteção contra ciclos",
                "An automation was stopped by the loop protection",
            )
            .to_owned(),
        _ => data_text(view, "title")
            .map(str::to_owned)
            .unwrap_or_else(|| {
                locale
                    .pick("Nova notificação", "New notification")
                    .to_owned()
            }),
    }
}

fn detail_lines(view: &NotificationView, locale: MailLocale) -> Vec<String> {
    let mut lines = Vec::new();
    if let Some(notice) = contributed(|texts| texts.notice(view, locale)) {
        lines.push(notice);
    } else if !view.title.trim().is_empty()
        && !matches!(
            view.kind,
            NotificationKind::AUTOMATION_MESSAGE | NotificationKind::AUTOMATION_EMAIL
        )
    {
        lines.push(view.title.clone());
    }
    if let Some(body) = &view.body {
        lines.extend(
            body.lines()
                .map(str::trim)
                .filter(|line| !line.is_empty())
                .map(str::to_owned),
        );
    }
    lines
}

fn action_label(view: &NotificationView, locale: MailLocale) -> &'static str {
    if let Some(label) = contributed(|texts| texts.action_label(view, locale)) {
        return label;
    }
    match view.kind {
        NotificationKind::DAILY_DIGEST => locale.pick("Ver minhas tarefas", "See my tasks"),
        _ => locale.pick("Abrir no Aphel", "Open in Aphel"),
    }
}

fn default_path(view: &NotificationView) -> &'static str {
    if let Some(path) = contributed(|texts| texts.default_path(view)) {
        return path;
    }
    match view.kind {
        NotificationKind::DAILY_DIGEST => "/tasks",
        NotificationKind::AUTOMATION_SKIPPED => "/settings/automations",
        _ => "/",
    }
}

pub fn notification_email(
    recipient: &Recipient,
    view: &NotificationView,
    app_base_url: &str,
) -> EmailMessage {
    let locale = MailLocale::resolve(recipient.locale.as_deref(), None);
    let base = app_base_url.trim_end_matches('/');
    let path = view.link.as_deref().unwrap_or_else(|| default_path(view));
    let link = format!("{base}{path}");
    let settings_link = format!("{base}/settings/notifications");
    let subject = headline(view, locale);
    let greeting = locale.greeting(&recipient.display_name);
    let lines = detail_lines(view, locale);
    let action = action_label(view, locale);
    let footer = locale.pick(
        "Você recebe este e-mail pelas suas preferências de notificação.",
        "You receive this email because of your notification preferences.",
    );
    let adjust = locale.pick("Ajuste em:", "Change them at:");
    let adjust_link = locale.pick("Ajustar preferências", "Change preferences");
    let signature = locale.signature();

    let mut text = format!("{greeting}\n\n{subject}.\n\n");
    for line in &lines {
        text.push_str(line);
        text.push('\n');
    }
    text.push_str(&format!(
        "\n{action}: {link}\n\n{footer} {adjust} {settings_link}\n\n{signature}\n"
    ));

    let html_lines: String = lines
        .iter()
        .map(|line| format!("<p style=\"margin:0 0 8px\">{}</p>", escape_html(line)))
        .collect();
    let html = format!(
        "<!doctype html><html lang=\"{lang}\"><body style=\"margin:0;padding:24px;background:#f4f4f5;font-family:Arial,Helvetica,sans-serif;color:#18181b\">\
<div style=\"max-width:520px;margin:0 auto;background:#ffffff;border-radius:8px;padding:32px\">\
<p style=\"margin:0 0 16px\">{greeting}</p>\
<p style=\"margin:0 0 16px;font-weight:bold\">{subject}</p>{html_lines}\
<p style=\"margin:24px 0\"><a href=\"{link}\" style=\"background:#4f46e5;color:#ffffff;padding:12px 20px;border-radius:6px;text-decoration:none;display:inline-block\">{action}</a></p>\
<p style=\"margin:0;font-size:13px;color:#52525b\">{footer} <a href=\"{settings_link}\" style=\"color:#4f46e5\">{adjust_link}</a></p>\
</div></body></html>",
        lang = locale.html_lang(),
        greeting = escape_html(&greeting),
        subject = escape_html(&subject),
        link = escape_html(&link),
        action = escape_html(action),
        footer = escape_html(footer),
        settings_link = escape_html(&settings_link),
        adjust_link = escape_html(adjust_link),
    );

    EmailMessage {
        calendar: None,
        to: recipient.email.clone(),
        subject,
        html,
        text,
    }
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
