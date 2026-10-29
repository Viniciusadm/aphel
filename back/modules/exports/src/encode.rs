use std::borrow::Cow;

use bytes::Bytes;
use rust_xlsxwriter::{Format, Workbook};

use aphel_kernel::errors::AppError;

pub const CSV_MIME_TYPE: &str = "text/csv";
pub const XLSX_MIME_TYPE: &str =
    "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet";

const UTF8_BOM: &[u8] = &[0xEF, 0xBB, 0xBF];
const FORMULA_PREFIXES: [char; 6] = ['=', '+', '-', '@', '\t', '\r'];
const MAX_XLSX_TEXT_LENGTH: usize = 32_767;
const MAX_XLSX_ROWS: usize = 1_048_575;

#[derive(Debug, Clone, PartialEq)]
pub enum ExportCell {
    Empty,
    Text(String),
    Number(f64),
}

impl ExportCell {
    pub(crate) fn as_text(&self) -> Cow<'_, str> {
        match self {
            Self::Empty => Cow::Borrowed(""),
            Self::Text(text) => neutralize_formula(text),
            Self::Number(number) => Cow::Owned(format_number(*number)),
        }
    }
}

fn format_number(number: f64) -> String {
    if number.fract() == 0.0 && number.abs() < 1e15 {
        format!("{}", number as i64)
    } else {
        number.to_string()
    }
}

fn is_plain_number(text: &str) -> bool {
    text.chars().any(|character| character.is_ascii_digit())
        && text.chars().all(|character| {
            character.is_ascii_digit()
                || matches!(character, ' ' | '(' | ')' | '-' | '.' | ',' | '+')
        })
}

pub(crate) fn neutralize_formula(text: &str) -> Cow<'_, str> {
    match text.chars().next() {
        Some(first) if FORMULA_PREFIXES.contains(&first) && !is_plain_number(text) => {
            Cow::Owned(format!("'{text}"))
        }
        _ => Cow::Borrowed(text),
    }
}

pub fn restore_neutralized(text: &str) -> &str {
    let mut characters = text.chars();
    match (characters.next(), characters.next()) {
        (Some('\''), Some(second)) if FORMULA_PREFIXES.contains(&second) => &text[1..],
        _ => text,
    }
}

pub fn encode_csv(
    headers: &[String],
    rows: &[Vec<ExportCell>],
    delimiter: u8,
) -> Result<Bytes, AppError> {
    let mut writer = ::csv::WriterBuilder::new()
        .delimiter(delimiter)
        .from_writer(UTF8_BOM.to_vec());
    let failure = |error: ::csv::Error| AppError::Internal(format!("failed to write CSV: {error}"));
    writer
        .write_record(
            headers
                .iter()
                .map(|header| neutralize_formula(header).into_owned()),
        )
        .map_err(failure)?;
    for row in rows {
        writer
            .write_record(row.iter().map(|cell| cell.as_text().into_owned()))
            .map_err(failure)?;
    }
    let content = writer
        .into_inner()
        .map_err(|error| AppError::Internal(format!("failed to write CSV: {error}")))?;
    Ok(Bytes::from(content))
}

pub fn encode_xlsx(headers: &[String], rows: &[Vec<ExportCell>]) -> Result<Bytes, AppError> {
    if rows.len() > MAX_XLSX_ROWS {
        return Err(AppError::Validation(format!(
            "XLSX files hold at most {MAX_XLSX_ROWS} rows"
        )));
    }
    let failure = |error: rust_xlsxwriter::XlsxError| {
        AppError::Internal(format!("failed to write XLSX: {error}"))
    };
    let mut workbook = Workbook::new();
    let bold = Format::new().set_bold();
    let sheet = workbook.add_worksheet();
    for (column, header) in headers.iter().enumerate() {
        sheet
            .write_string_with_format(0, column as u16, xlsx_text(header), &bold)
            .map_err(failure)?;
    }
    sheet.set_freeze_panes(1, 0).map_err(failure)?;
    for (index, row) in rows.iter().enumerate() {
        let row_number = index as u32 + 1;
        for (column, cell) in row.iter().enumerate() {
            match cell {
                ExportCell::Empty => {}
                ExportCell::Text(text) => {
                    sheet
                        .write_string(row_number, column as u16, xlsx_text(text))
                        .map_err(failure)?;
                }
                ExportCell::Number(number) => {
                    sheet
                        .write_number(row_number, column as u16, *number)
                        .map_err(failure)?;
                }
            }
        }
    }
    Ok(Bytes::from(workbook.save_to_buffer().map_err(failure)?))
}

fn xlsx_text(text: &str) -> String {
    let neutralized = neutralize_formula(text);
    if neutralized.chars().count() > MAX_XLSX_TEXT_LENGTH {
        neutralized.chars().take(MAX_XLSX_TEXT_LENGTH).collect()
    } else {
        neutralized.into_owned()
    }
}
