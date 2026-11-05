use chrono::{DateTime, NaiveDate, Utc};
use chrono_tz::Tz;
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum Locale {
    #[default]
    PtBr,
    En,
}

pub(crate) struct Labels {
    pub(crate) yes: &'static str,
    pub(crate) no: &'static str,
    pub(crate) proposal_title: &'static str,
    pub(crate) item: &'static str,
    pub(crate) quantity: &'static str,
    pub(crate) unit_price: &'static str,
    pub(crate) discount: &'static str,
    pub(crate) total: &'static str,
    pub(crate) subtotal: &'static str,
    pub(crate) discounts: &'static str,
    pub(crate) taxes: &'static str,
    pub(crate) no_items: &'static str,
    pub(crate) items_unavailable: &'static str,
    pub(crate) page: &'static str,
    pub(crate) page_of: &'static str,
    pub(crate) deal_open: &'static str,
    pub(crate) deal_won: &'static str,
    pub(crate) deal_lost: &'static str,
}

const PT_BR_LABELS: Labels = Labels {
    yes: "Sim",
    no: "Não",
    proposal_title: "Proposta nº",
    item: "Item",
    quantity: "Qtd.",
    unit_price: "Preço unit.",
    discount: "Desconto",
    total: "Total",
    subtotal: "Subtotal",
    discounts: "Descontos",
    taxes: "Impostos",
    no_items: "Nenhum item nesta oportunidade.",
    items_unavailable: "Itens indisponíveis.",
    page: "Página",
    page_of: "de",
    deal_open: "Em aberto",
    deal_won: "Ganha",
    deal_lost: "Perdida",
};

const EN_LABELS: Labels = Labels {
    yes: "Yes",
    no: "No",
    proposal_title: "Proposal no.",
    item: "Item",
    quantity: "Qty.",
    unit_price: "Unit price",
    discount: "Discount",
    total: "Total",
    subtotal: "Subtotal",
    discounts: "Discounts",
    taxes: "Taxes",
    no_items: "No items in this deal.",
    items_unavailable: "Items unavailable.",
    page: "Page",
    page_of: "of",
    deal_open: "Open",
    deal_won: "Won",
    deal_lost: "Lost",
};

impl Locale {
    pub(crate) fn parse(tag: &str) -> Self {
        let tag = tag.trim().to_ascii_lowercase();
        if tag == "en" || tag.starts_with("en-") || tag.starts_with("en_") {
            Self::En
        } else {
            Self::PtBr
        }
    }

    pub(crate) fn labels(self) -> &'static Labels {
        match self {
            Self::PtBr => &PT_BR_LABELS,
            Self::En => &EN_LABELS,
        }
    }

    fn thousands_separator(self) -> char {
        match self {
            Self::PtBr => '.',
            Self::En => ',',
        }
    }

    fn decimal_separator(self) -> char {
        match self {
            Self::PtBr => ',',
            Self::En => '.',
        }
    }

    fn date_pattern(self) -> &'static str {
        match self {
            Self::PtBr => "%d/%m/%Y",
            Self::En => "%m/%d/%Y",
        }
    }

    fn datetime_pattern(self) -> &'static str {
        match self {
            Self::PtBr => "%d/%m/%Y %H:%M",
            Self::En => "%m/%d/%Y %I:%M %p",
        }
    }
}

pub(crate) fn group_thousands(digits: &str, locale: Locale) -> String {
    let mut grouped = String::new();
    for (index, character) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            grouped.push(locale.thousands_separator());
        }
        grouped.push(character);
    }
    grouped
}

pub(crate) fn decimal(
    raw: &str,
    min_fraction: usize,
    max_fraction: usize,
    locale: Locale,
) -> String {
    let raw = raw.trim();
    let (negative, unsigned) = match raw.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, raw),
    };
    let (integer, fraction) = unsigned.split_once('.').unwrap_or((unsigned, ""));
    if (integer.is_empty() && fraction.is_empty())
        || !integer.chars().all(|character| character.is_ascii_digit())
        || !fraction.chars().all(|character| character.is_ascii_digit())
    {
        return raw.to_owned();
    }
    let integer = integer.trim_start_matches('0');
    let integer = if integer.is_empty() { "0" } else { integer };
    let mut fraction: String = fraction.chars().take(max_fraction).collect();
    while fraction.len() > min_fraction && fraction.ends_with('0') {
        fraction.pop();
    }
    while fraction.len() < min_fraction {
        fraction.push('0');
    }
    let sign = if negative { "-" } else { "" };
    let integer = group_thousands(integer, locale);
    if fraction.is_empty() {
        format!("{sign}{integer}")
    } else {
        format!("{sign}{integer}{}{fraction}", locale.decimal_separator())
    }
}

pub(crate) fn currency_symbol(currency: &str, locale: Locale) -> String {
    match (currency.to_ascii_uppercase().as_str(), locale) {
        ("BRL", _) => "R$".to_owned(),
        ("USD", Locale::En) => "$".to_owned(),
        ("USD", _) => "US$".to_owned(),
        ("EUR", _) => "€".to_owned(),
        ("GBP", _) => "£".to_owned(),
        (other, _) => other.to_owned(),
    }
}

pub(crate) fn money(raw: &str, currency: &str, locale: Locale) -> String {
    let formatted = decimal(raw, 2, 2, locale);
    let symbol = currency_symbol(currency, locale);
    let separator = match (locale, symbol.chars().count()) {
        (Locale::En, 1) => "",
        _ => " ",
    };
    match formatted.strip_prefix('-') {
        Some(rest) => format!("-{symbol}{separator}{rest}"),
        None => format!("{symbol}{separator}{formatted}"),
    }
}

pub(crate) fn number(raw: &str, locale: Locale) -> String {
    decimal(raw, 0, 4, locale)
}

pub(crate) fn value_text(value: &Value) -> Option<String> {
    match value {
        Value::Null => None,
        Value::String(text) => Some(text.clone()),
        Value::Number(number) => Some(number.to_string()),
        Value::Bool(flag) => Some(flag.to_string()),
        Value::Array(values) => {
            let parts: Vec<String> = values.iter().filter_map(value_text).collect();
            (!parts.is_empty()).then(|| parts.join(", "))
        }
        Value::Object(map) => ["name", "display_name", "label", "title"]
            .iter()
            .find_map(|key| map.get(*key).and_then(value_text)),
    }
}

pub(crate) fn display_text(value: &Value, locale: Locale) -> Option<String> {
    match value {
        Value::Bool(flag) => Some(yes_no(*flag, locale)),
        Value::Array(values) => {
            let parts: Vec<String> = values
                .iter()
                .filter_map(|value| display_text(value, locale))
                .collect();
            (!parts.is_empty()).then(|| parts.join(", "))
        }
        other => value_text(other),
    }
}

pub(crate) fn yes_no(flag: bool, locale: Locale) -> String {
    let labels = locale.labels();
    if flag { labels.yes } else { labels.no }.to_owned()
}

pub(crate) fn date(day: NaiveDate, locale: Locale) -> String {
    day.format(locale.date_pattern()).to_string()
}

pub(crate) fn date_text(raw: &str, timezone: Tz, locale: Locale) -> String {
    if let Ok(day) = NaiveDate::parse_from_str(raw, "%Y-%m-%d") {
        return date(day, locale);
    }
    match DateTime::parse_from_rfc3339(raw) {
        Ok(moment) => date(moment.with_timezone(&timezone).date_naive(), locale),
        Err(_) => raw.to_owned(),
    }
}

pub(crate) fn datetime_text(raw: &str, timezone: Tz, locale: Locale) -> String {
    match DateTime::parse_from_rfc3339(raw) {
        Ok(moment) => moment
            .with_timezone(&timezone)
            .format(locale.datetime_pattern())
            .to_string(),
        Err(_) => date_text(raw, timezone, locale),
    }
}

pub(crate) fn today(timezone: Tz) -> NaiveDate {
    Utc::now().with_timezone(&timezone).date_naive()
}

pub(crate) fn proposal_number(number: u32) -> String {
    format!("{number:04}")
}

pub(crate) fn slug(text: &str) -> String {
    let mut slug = String::new();
    for character in text.chars() {
        let folded = fold_accent(character);
        if folded.is_ascii_alphanumeric() {
            slug.push(folded.to_ascii_lowercase());
        } else if !slug.ends_with('-') && !slug.is_empty() {
            slug.push('-');
        }
    }
    let trimmed = slug.trim_end_matches('-');
    trimmed
        .chars()
        .take(60)
        .collect::<String>()
        .trim_end_matches('-')
        .to_owned()
}

fn fold_accent(character: char) -> char {
    match character {
        'á' | 'à' | 'â' | 'ã' | 'ä' | 'Á' | 'À' | 'Â' | 'Ã' | 'Ä' => 'a',
        'é' | 'è' | 'ê' | 'ë' | 'É' | 'È' | 'Ê' | 'Ë' => 'e',
        'í' | 'ì' | 'î' | 'ï' | 'Í' | 'Ì' | 'Î' | 'Ï' => 'i',
        'ó' | 'ò' | 'ô' | 'õ' | 'ö' | 'Ó' | 'Ò' | 'Ô' | 'Õ' | 'Ö' => 'o',
        'ú' | 'ù' | 'û' | 'ü' | 'Ú' | 'Ù' | 'Û' | 'Ü' => 'u',
        'ç' | 'Ç' => 'c',
        'ñ' | 'Ñ' => 'n',
        other => other,
    }
}
