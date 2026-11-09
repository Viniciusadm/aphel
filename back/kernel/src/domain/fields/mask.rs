use std::str::FromStr;

use rust_decimal::Decimal;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextMask {
    Cpf,
    Cnpj,
    CpfCnpj,
    Cep,
    Uf,
    IbgeCity,
    IbgeState,
    Ncm,
    Cest,
    Cfop,
    Gtin,
    Time,
    Digits,
    Percent,
}

const BRAZILIAN_STATES: [&str; 27] = [
    "AC", "AL", "AP", "AM", "BA", "CE", "DF", "ES", "GO", "MA", "MT", "MS", "MG", "PA", "PB", "PR",
    "PE", "PI", "RJ", "RN", "RS", "RO", "RR", "SC", "SP", "SE", "TO",
];

const CNPJ_FIRST_WEIGHTS: [u32; 12] = [5, 4, 3, 2, 9, 8, 7, 6, 5, 4, 3, 2];
const CNPJ_SECOND_WEIGHTS: [u32; 13] = [6, 5, 4, 3, 2, 9, 8, 7, 6, 5, 4, 3, 2];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidMaskedValue(pub TextMask);

impl TextMask {
    pub const ALL: [Self; 14] = [
        Self::Cpf,
        Self::Cnpj,
        Self::CpfCnpj,
        Self::Cep,
        Self::Uf,
        Self::IbgeCity,
        Self::IbgeState,
        Self::Ncm,
        Self::Cest,
        Self::Cfop,
        Self::Gtin,
        Self::Time,
        Self::Digits,
        Self::Percent,
    ];

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Cpf => "cpf",
            Self::Cnpj => "cnpj",
            Self::CpfCnpj => "cpf_cnpj",
            Self::Cep => "cep",
            Self::Uf => "uf",
            Self::IbgeCity => "ibge_city",
            Self::IbgeState => "ibge_state",
            Self::Ncm => "ncm",
            Self::Cest => "cest",
            Self::Cfop => "cfop",
            Self::Gtin => "gtin",
            Self::Time => "time",
            Self::Digits => "digits",
            Self::Percent => "percent",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|mask| mask.as_str() == value)
    }

    pub fn normalize(&self, text: &str) -> Result<String, InvalidMaskedValue> {
        let invalid = InvalidMaskedValue(*self);
        let normalized = match self {
            Self::Cpf => valid_cpf(&digits(text)),
            Self::Cnpj => valid_cnpj(&alphanumeric(text)),
            Self::CpfCnpj => {
                let cleaned = alphanumeric(text);
                if cleaned.len() == 11 {
                    valid_cpf(&cleaned)
                } else {
                    valid_cnpj(&cleaned)
                }
            }
            Self::Cep => fixed_digits(text, 8),
            Self::Uf => {
                let state = text.trim().to_ascii_uppercase();
                BRAZILIAN_STATES.contains(&state.as_str()).then_some(state)
            }
            Self::IbgeCity => fixed_digits(text, 7),
            Self::IbgeState => fixed_digits(text, 2),
            Self::Ncm => fixed_digits(text, 8),
            Self::Cest => fixed_digits(text, 7),
            Self::Cfop => {
                fixed_digits(text, 4).filter(|code| matches!(code.as_bytes()[0], b'1'..=b'7'))
            }
            Self::Gtin => {
                let code = digits(text);
                matches!(code.len(), 8 | 12 | 13 | 14).then_some(code)
            }
            Self::Time => valid_time(text),
            Self::Digits => {
                let code = digits(text);
                (!code.is_empty() && has_only_separators(text, |c| c.is_ascii_digit()))
                    .then_some(code)
            }
            Self::Percent => valid_percent(text),
        };
        normalized.ok_or(invalid)
    }
}

fn digits(text: &str) -> String {
    text.chars().filter(char::is_ascii_digit).collect()
}

fn alphanumeric(text: &str) -> String {
    text.chars()
        .filter(char::is_ascii_alphanumeric)
        .map(|c| c.to_ascii_uppercase())
        .collect()
}

fn has_only_separators(text: &str, keep: impl Fn(char) -> bool) -> bool {
    text.trim()
        .chars()
        .all(|c| keep(c) || matches!(c, '.' | '-' | '/' | ' '))
}

fn fixed_digits(text: &str, length: usize) -> Option<String> {
    if !has_only_separators(text, |c| c.is_ascii_digit()) {
        return None;
    }
    let code = digits(text);
    (code.len() == length).then_some(code)
}

fn repeated(code: &str) -> bool {
    code.chars().all(|c| code.starts_with(c))
}

fn valid_cpf(code: &str) -> Option<String> {
    if code.len() != 11 || !code.chars().all(|c| c.is_ascii_digit()) || repeated(code) {
        return None;
    }
    let values: Vec<u32> = code.chars().filter_map(|c| c.to_digit(10)).collect();
    let check = |length: usize| {
        let sum: u32 = values[..length]
            .iter()
            .enumerate()
            .map(|(index, value)| value * (length as u32 + 1 - index as u32))
            .sum();
        let remainder = (sum * 10) % 11;
        if remainder == 10 { 0 } else { remainder }
    };
    (check(9) == values[9] && check(10) == values[10]).then(|| code.to_owned())
}

fn cnpj_check_digit(values: &[u32], weights: &[u32]) -> u32 {
    let sum: u32 = values
        .iter()
        .zip(weights)
        .map(|(value, weight)| value * weight)
        .sum();
    let remainder = sum % 11;
    if remainder < 2 { 0 } else { 11 - remainder }
}

fn valid_cnpj(code: &str) -> Option<String> {
    let bytes = code.as_bytes();
    if bytes.len() != 14
        || !bytes[..12].iter().all(u8::is_ascii_alphanumeric)
        || !bytes[12..].iter().all(u8::is_ascii_digit)
        || repeated(code)
    {
        return None;
    }
    let values: Vec<u32> = bytes.iter().map(|byte| u32::from(*byte) - 48).collect();
    let first = cnpj_check_digit(&values[..12], &CNPJ_FIRST_WEIGHTS);
    let second = cnpj_check_digit(&values[..13], &CNPJ_SECOND_WEIGHTS);
    (values[12] == first && values[13] == second).then(|| code.to_owned())
}

fn valid_time(text: &str) -> Option<String> {
    if !has_only_separators(text, |c| c.is_ascii_digit() || c == ':') {
        return None;
    }
    let code = digits(text);
    if code.len() != 4 {
        return None;
    }
    let hours: u32 = code[..2].parse().ok()?;
    let minutes: u32 = code[2..].parse().ok()?;
    let valid = (hours < 24 && minutes < 60) || (hours == 24 && minutes == 0);
    valid.then(|| format!("{}:{}", &code[..2], &code[2..]))
}

fn valid_percent(text: &str) -> Option<String> {
    let number = Decimal::from_str(&text.trim().replace(',', ".")).ok()?;
    let valid = number >= Decimal::ZERO && number <= Decimal::from(100) && number.scale() <= 2;
    valid.then(|| number.normalize().to_string())
}
