use crate::domain::custom::{ReportRecipient, SavedReport};
use crate::table::{ReportCell, report_table};
use aphel_kernel::application::mail::{EmailMessage, MailLocale};
use aphel_kernel::application::query::{AggregateRange, AggregateResult};

pub(crate) const MAX_EMAIL_ROWS: usize = 50;
pub(crate) const MAX_ATTACHMENT_BYTES: usize = 5 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AttachmentNote {
    Attached,
    TooLarge,
    Unavailable,
}

fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn range_text(range: &AggregateRange, locale: MailLocale) -> String {
    format!(
        "{} {} {}",
        range.from.format(locale.date_format()),
        locale.pick("a", "to"),
        range.to.format(locale.date_format())
    )
}

fn cell_style(cell: &ReportCell, bold: bool) -> String {
    let align = if cell.is_numeric() { "right" } else { "left" };
    let weight = if bold { "font-weight:bold;" } else { "" };
    format!(
        "padding:6px 8px;border-bottom:1px solid #e4e4e7;text-align:{align};{weight}white-space:nowrap"
    )
}

pub(crate) fn report_link(app_base_url: &str, report: &SavedReport) -> String {
    format!(
        "{}/reports/saved/{}",
        app_base_url.trim_end_matches('/'),
        report.id
    )
}

pub(crate) fn report_email(
    recipient: &ReportRecipient,
    report: &SavedReport,
    result: &AggregateResult,
    link: &str,
    attachment: AttachmentNote,
) -> EmailMessage {
    let table = report_table(result);
    let total_rows = table.rows.len();
    let shown = total_rows.min(MAX_EMAIL_ROWS);
    let with_totals = !result.groups.is_empty();
    let locale = MailLocale::resolve(recipient.locale.as_deref(), None);
    let greeting = locale.greeting(&recipient.display_name);
    let more = if result.truncated { "+" } else { "" };
    let mut lines = vec![match locale {
        MailLocale::PtBr => format!(
            "Segue o relatório \"{}\" calculado com as suas permissões no Aphel.",
            report.name
        ),
        MailLocale::En => format!(
            "Here is the report \"{}\", calculated with your permissions in Aphel.",
            report.name
        ),
    }];
    if let Some(period) = &result.period {
        lines.push(format!(
            "{} {}.",
            locale.pick("Período:", "Period:"),
            range_text(period, locale)
        ));
    }
    if let Some(previous) = &result.previous_period {
        lines.push(format!(
            "{} {}.",
            locale.pick("Comparado com:", "Compared with:"),
            range_text(previous, locale)
        ));
    }
    if result.truncated || total_rows > shown {
        lines.push(match locale {
            MailLocale::PtBr => format!(
                "Mostrando {shown} de {total_rows}{more} linhas. Veja o relatório completo no Aphel."
            ),
            MailLocale::En => format!(
                "Showing {shown} of {total_rows}{more} rows. See the full report in Aphel."
            ),
        });
    }
    lines.push(
        match attachment {
            AttachmentNote::Attached => locale.pick(
                "A planilha completa vai em anexo.",
                "The full spreadsheet is attached.",
            ),
            AttachmentNote::TooLarge => locale.pick(
                "A planilha ficou grande demais para ir em anexo; baixe-a pelo link abaixo.",
                "The spreadsheet is too large to attach; download it from the link below.",
            ),
            AttachmentNote::Unavailable => locale.pick(
                "Não foi possível gerar a planilha em anexo; baixe-a pelo link abaixo.",
                "The spreadsheet could not be attached; download it from the link below.",
            ),
        }
        .to_owned(),
    );
    let empty = locale.pick("Nenhum registro encontrado.", "No records found.");
    let open_label = locale.pick("Abrir no Aphel", "Open in Aphel");
    let footer = locale.pick(
        "Você recebe este e-mail porque foi incluído no envio programado deste relatório.",
        "You receive this email because you were added to the scheduled delivery of this report.",
    );
    let signature = locale.signature();

    let mut text = format!("{greeting}\n\n");
    for line in &lines {
        text.push_str(line);
        text.push('\n');
    }
    text.push('\n');
    text.push_str(&table.headers.join("\t"));
    text.push('\n');
    let visible_rows = table.rows.iter().take(shown);
    for row in visible_rows.clone() {
        let cells: Vec<String> = row.iter().map(ReportCell::display).collect();
        text.push_str(&cells.join("\t"));
        text.push('\n');
    }
    if with_totals {
        let cells: Vec<String> = table.totals.iter().map(ReportCell::display).collect();
        text.push_str(&cells.join("\t"));
        text.push('\n');
    }
    if total_rows == 0 {
        text.push_str(empty);
        text.push('\n');
    }
    text.push_str(&format!(
        "\n{open_label}: {link}\n\n{footer}\n\n{signature}\n"
    ));

    let header_html: String = table
        .headers
        .iter()
        .map(|header| {
            format!(
                "<th style=\"padding:6px 8px;border-bottom:2px solid #d4d4d8;text-align:left;white-space:nowrap\">{}</th>",
                escape(header)
            )
        })
        .collect();
    let row_html = |row: &Vec<ReportCell>, bold: bool| -> String {
        let cells: String = row
            .iter()
            .map(|cell| {
                format!(
                    "<td style=\"{}\">{}</td>",
                    cell_style(cell, bold),
                    escape(&cell.display())
                )
            })
            .collect();
        format!("<tr>{cells}</tr>")
    };
    let mut body_html: String = visible_rows.map(|row| row_html(row, false)).collect();
    if with_totals {
        body_html.push_str(&row_html(&table.totals, true));
    }
    if total_rows == 0 {
        body_html.push_str(&format!(
            "<tr><td colspan=\"{}\" style=\"padding:12px 8px;color:#52525b\">{}</td></tr>",
            table.headers.len().max(1),
            escape(empty)
        ));
    }
    let paragraphs: String = lines
        .iter()
        .map(|line| format!("<p style=\"margin:0 0 12px\">{}</p>", escape(line)))
        .collect();
    let html = format!(
        "<!doctype html><html lang=\"{lang}\"><body style=\"margin:0;padding:24px;background:#f4f4f5;font-family:Arial,Helvetica,sans-serif;color:#18181b\">\
<div style=\"max-width:720px;margin:0 auto;background:#ffffff;border-radius:8px;padding:32px\">\
<p style=\"margin:0 0 16px\">{greeting}</p>{paragraphs}\
<div style=\"overflow-x:auto;margin:16px 0\"><table style=\"border-collapse:collapse;width:100%;font-size:13px\"><thead><tr>{header_html}</tr></thead><tbody>{body_html}</tbody></table></div>\
<p style=\"margin:24px 0\"><a href=\"{link}\" style=\"background:#4f46e5;color:#ffffff;padding:12px 20px;border-radius:6px;text-decoration:none;display:inline-block\">{open_report}</a></p>\
<p style=\"margin:0;font-size:13px;color:#52525b\">{footer}</p>\
</div></body></html>",
        lang = locale.html_lang(),
        greeting = escape(&greeting),
        link = escape(link),
        open_report = escape(locale.pick("Abrir relatório", "Open report")),
        footer = escape(footer),
    );

    EmailMessage {
        calendar: None,
        to: recipient.email.clone(),
        subject: format!("{} {}", locale.pick("Relatório:", "Report:"), report.name),
        html,
        text,
    }
}
