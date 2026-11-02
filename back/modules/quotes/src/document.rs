use aphel_contract_crm::CrmPermissions;
use chrono::NaiveDate;
use rust_decimal::Decimal;

use crate::QUOTE;
use crate::domain::{PublicQuote, PublicQuoteState, QuoteItem, QuoteStatus};
use crate::pdf::{self, Block, Header, ItemsTable, TableRow};
use crate::public::public_quote;
use crate::service::{now, timezone, visible};
use aphel_kernel::Ctx;
use aphel_kernel::application::fields::CustomFieldPorts;
use aphel_kernel::application::fields::permissions::{FieldAccess, FieldAccessPolicy};
use aphel_kernel::domain::access::Permission;
use aphel_kernel::errors::AppError;

const HIDDEN: &str = "—";

pub(crate) struct QuotePdf {
    pub(crate) file_name: String,
    pub(crate) content: Vec<u8>,
}

fn group_thousands(digits: &str) -> String {
    let mut grouped = String::new();
    for (index, character) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            grouped.push('.');
        }
        grouped.push(character);
    }
    grouped
}

fn decimal_text(value: Decimal, min_fraction: usize, max_fraction: usize) -> String {
    let raw = format!("{:.*}", max_fraction, value.round_dp(max_fraction as u32));
    let (negative, unsigned) = match raw.strip_prefix('-') {
        Some(rest) => (true, rest.to_owned()),
        None => (false, raw),
    };
    let (integer, fraction) = unsigned.split_once('.').unwrap_or((&unsigned, ""));
    let mut fraction = fraction.to_owned();
    while fraction.len() > min_fraction && fraction.ends_with('0') {
        fraction.pop();
    }
    let sign = if negative { "-" } else { "" };
    let integer = group_thousands(integer);
    if fraction.is_empty() {
        format!("{sign}{integer}")
    } else {
        format!("{sign}{integer},{fraction}")
    }
}

fn currency_symbol(currency: &str) -> String {
    match currency.to_ascii_uppercase().as_str() {
        "BRL" => "R$".to_owned(),
        "USD" => "US$".to_owned(),
        "EUR" => "€".to_owned(),
        "GBP" => "£".to_owned(),
        other => other.to_owned(),
    }
}

fn money_text(value: Decimal, currency: &str) -> String {
    let formatted = decimal_text(value, 2, 2);
    let symbol = currency_symbol(currency);
    match formatted.strip_prefix('-') {
        Some(rest) => format!("-{symbol} {rest}"),
        None => format!("{symbol} {formatted}"),
    }
}

fn date_text(day: NaiveDate) -> String {
    day.format("%d/%m/%Y").to_string()
}

fn status_text(status: QuoteStatus) -> &'static str {
    match status {
        QuoteStatus::Draft => "Rascunho",
        QuoteStatus::PendingApproval => "Em aprovação",
        QuoteStatus::Approved => "Aprovada",
        QuoteStatus::Sent => "Enviada",
        QuoteStatus::Accepted => "Aceita",
        QuoteStatus::Rejected => "Recusada",
        QuoteStatus::Expired => "Expirada",
    }
}

fn slug(text: &str) -> String {
    let mut slug = String::new();
    for character in text.chars() {
        if character.is_ascii_alphanumeric() {
            slug.push(character.to_ascii_lowercase());
        } else if !slug.is_empty() && !slug.ends_with('-') {
            slug.push('-');
        }
    }
    slug.trim_end_matches('-').chars().take(60).collect()
}

fn file_name(number: u32, version: u32, title: &str) -> String {
    let slug = slug(title);
    if slug.is_empty() {
        format!("cotacao-{number}-v{version}.pdf")
    } else {
        format!("cotacao-{number}-v{version}-{slug}.pdf")
    }
}

struct Visibility {
    policy: FieldAccessPolicy,
}

impl Visibility {
    fn shows(&self, key: &str) -> bool {
        self.policy.key_access(key) != FieldAccess::Hidden
    }

    fn money(&self, key: &str, value: Decimal, currency: &str) -> String {
        if self.shows(key) {
            money_text(value, currency)
        } else {
            HIDDEN.to_owned()
        }
    }
}

fn item_row(item: &QuoteItem, currency: &str, visibility: &Visibility) -> TableRow {
    let data = &item.data;
    let mut name = data.name.clone();
    if let Some(unit) = data.unit.as_ref().filter(|unit| !unit.trim().is_empty()) {
        name = format!("{name} ({unit})");
    }
    if let Some(description) = data
        .description
        .as_ref()
        .filter(|description| !description.trim().is_empty())
    {
        name = format!("{name} — {}", description.trim());
    }
    let shows_totals = visibility.shows("total");
    TableRow {
        name,
        quantity: decimal_text(data.quantity, 0, 4),
        unit_price: money_text(data.unit_price, currency),
        discount: if item.amounts.item_discount.is_zero() {
            HIDDEN.to_owned()
        } else {
            money_text(item.amounts.item_discount, currency)
        },
        total: if shows_totals {
            money_text(item.amounts.total, currency)
        } else {
            HIDDEN.to_owned()
        },
    }
}

fn blocks(document: &PublicQuote, visibility: &Visibility) -> Vec<Block> {
    let quote = &document.quote;
    let currency = quote.currency.as_str();
    let mut blocks = vec![Block::Heading(quote.title.clone())];
    let mut field = |label: &str, value: Option<String>| {
        if let Some(value) = value.filter(|value| !value.trim().is_empty()) {
            blocks.push(Block::Field {
                label: label.to_owned(),
                value,
            });
        }
    };
    field("Situação", Some(status_text(quote.status).to_owned()));
    field("Empresa", document.company_name.clone());
    field("Contato", document.contact_name.clone());
    field(
        "Responsável",
        match (&document.owner_name, &document.owner_email) {
            (Some(name), Some(email)) => Some(format!("{name} <{email}>")),
            (Some(name), None) => Some(name.clone()),
            (None, email) => email.clone(),
        },
    );
    field("Válida até", quote.valid_until.map(date_text));
    if let Some(acceptance) = &quote.acceptance {
        let when = quote
            .accepted_at
            .map(|moment| {
                moment
                    .with_timezone(&timezone(&document.organization.timezone))
                    .format("%d/%m/%Y %H:%M")
                    .to_string()
            })
            .unwrap_or_default();
        field(
            "Aceita por",
            Some(format!("{} {when}", acceptance.name).trim().to_owned()),
        );
    }
    blocks.push(Block::Gap);
    blocks.push(Block::Items(ItemsTable {
        rows: document
            .items
            .iter()
            .map(|item| item_row(item, currency, visibility))
            .collect(),
        subtotal: visibility.money("subtotal", quote.subtotal, currency),
        discount_total: visibility.money("discount_total", quote.discount_total, currency),
        tax_total: visibility.money("tax_total", quote.tax_total, currency),
        total: visibility.money("total", quote.total, currency),
    }));
    if let Some(terms) = quote
        .terms
        .as_ref()
        .filter(|terms| !terms.trim().is_empty())
    {
        blocks.push(Block::Heading("Condições".to_owned()));
        blocks.push(Block::Paragraph(terms.clone()));
    }
    if let Some(notes) = quote
        .notes
        .as_ref()
        .filter(|notes| !notes.trim().is_empty())
    {
        blocks.push(Block::Heading("Observações".to_owned()));
        blocks.push(Block::Paragraph(notes.clone()));
    }
    blocks
}

pub(crate) async fn render(ctx: &Ctx<'_>, quote_id: &str) -> Result<QuotePdf, AppError> {
    let quote = visible(ctx, &Permission::QUOTES_READ, quote_id).await?;
    let visibility = Visibility {
        policy: FieldAccessPolicy::load(ctx.fields().definitions(), ctx.scope, &QUOTE).await?,
    };
    let document = public_quote(ctx, quote, PublicQuoteState::Open).await?;
    let quote = &document.quote;
    let today = now(ctx)
        .with_timezone(&timezone(&document.organization.timezone))
        .date_naive();
    let header = Header {
        organization: document.organization.name.clone(),
        title: format!("Cotação nº {} (v{})", quote.number, quote.version),
        subtitle: date_text(today),
    };
    let content = pdf::document(header, &blocks(&document, &visibility));
    Ok(QuotePdf {
        file_name: file_name(quote.number, quote.version, &quote.title),
        content,
    })
}
