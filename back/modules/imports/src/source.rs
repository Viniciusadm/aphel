use std::borrow::Cow;
use std::io::Cursor;

use calamine::{Data, Reader, open_workbook_auto_from_rs};
use chrono::NaiveTime;

use encoding_rs::WINDOWS_1252;

use aphel_kernel::errors::AppError;
use aphel_sdk::exports::restore_neutralized;

pub(crate) const MAX_IMPORT_ROWS: usize = 50_000;
pub(crate) const MAX_IMPORT_COLUMNS: usize = 200;
pub(crate) const MAX_CELL_LENGTH: usize = 10_000;
pub(crate) const PREVIEW_ROWS: usize = 20;
pub(crate) const DELIMITER_CANDIDATES: [u8; 4] = *b",;\t|";

pub(crate) const EMPTY_IMPORT: &str = "empty_import";
pub(crate) const TOO_MANY_ROWS: &str = "too_many_rows";
pub(crate) const TOO_MANY_COLUMNS: &str = "too_many_columns";
pub(crate) const INVALID_CSV: &str = "invalid_csv";
pub(crate) const INVALID_SPREADSHEET: &str = "invalid_spreadsheet";
pub(crate) const XLS_MIME_TYPE: &str = "application/vnd.ms-excel";
pub(crate) const XLSX_MIME_TYPE: &str =
    "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet";

const UTF8_BOM: &[u8] = &[0xEF, 0xBB, 0xBF];
const ZIP_SIGNATURE: &[u8] = &[0x50, 0x4B, 0x03, 0x04];
const OLE_SIGNATURE: &[u8] = &[0xD0, 0xCF, 0x11, 0xE0];
const DETECTION_SAMPLE_LINES: usize = 50;

pub use aphel_sdk::imports::{ParsedSource, SourceRow, TextEncoding};

pub(crate) fn decode(bytes: &[u8]) -> (Cow<'_, str>, TextEncoding) {
    let content = bytes.strip_prefix(UTF8_BOM).unwrap_or(bytes);
    match std::str::from_utf8(content) {
        Ok(text) => (Cow::Borrowed(text), TextEncoding::Utf8),
        Err(_) => {
            let (text, _) = WINDOWS_1252.decode_without_bom_handling(content);
            (text, TextEncoding::Windows1252)
        }
    }
}

pub(crate) fn detect_delimiter(text: &str) -> u8 {
    let sample: String = text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .take(DETECTION_SAMPLE_LINES)
        .collect::<Vec<_>>()
        .join("\n");
    let mut best = (b',', 0usize);
    for candidate in DELIMITER_CANDIDATES {
        let score = delimiter_score(&sample, candidate);
        if score > best.1 {
            best = (candidate, score);
        }
    }
    best.0
}

fn delimiter_score(sample: &str, delimiter: u8) -> usize {
    let mut reader = ::csv::ReaderBuilder::new()
        .delimiter(delimiter)
        .has_headers(false)
        .flexible(true)
        .from_reader(sample.as_bytes());
    let counts: Vec<usize> = reader
        .records()
        .map_while(Result::ok)
        .map(|record| record.len())
        .collect();
    let Some(&first) = counts.first() else {
        return 0;
    };
    if first < 2 {
        return 0;
    }
    let consistent = counts.iter().filter(|count| **count == first).count();
    consistent * 1000 / counts.len() * first
}

pub(crate) fn is_spreadsheet(bytes: &[u8]) -> bool {
    bytes.starts_with(ZIP_SIGNATURE) || bytes.starts_with(OLE_SIGNATURE)
}

pub(crate) fn source_mime_type(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(ZIP_SIGNATURE) {
        Some(XLSX_MIME_TYPE)
    } else if bytes.starts_with(OLE_SIGNATURE) {
        Some(XLS_MIME_TYPE)
    } else {
        None
    }
}

pub fn parse_source(bytes: &[u8]) -> Result<ParsedSource, AppError> {
    if is_spreadsheet(bytes) {
        return parse_spreadsheet(bytes);
    }
    let (text, encoding) = decode(bytes);
    let delimiter = detect_delimiter(&text);
    let mut reader = ::csv::ReaderBuilder::new()
        .delimiter(delimiter)
        .has_headers(false)
        .flexible(true)
        .from_reader(text.as_bytes());

    let mut collector = RowCollector::default();
    for record in reader.records() {
        let record = record.map_err(|error| {
            AppError::validation_code(INVALID_CSV, format!("the file is not a valid CSV: {error}"))
        })?;
        let line = record
            .position()
            .map(|position| position.line() as u32)
            .unwrap_or_default();
        collector.push(line, record.iter().map(clean_cell).collect())?;
    }
    collector.finish(encoding, delimiter)
}

fn parse_spreadsheet(bytes: &[u8]) -> Result<ParsedSource, AppError> {
    let invalid = |error: String| {
        AppError::validation_code(
            INVALID_SPREADSHEET,
            format!("the file is not a valid spreadsheet: {error}"),
        )
    };
    let mut workbook = open_workbook_auto_from_rs(Cursor::new(bytes))
        .map_err(|error| invalid(error.to_string()))?;
    let range = workbook
        .worksheet_range_at(0)
        .ok_or_else(|| invalid("the workbook has no sheets".to_owned()))?
        .map_err(|error| invalid(error.to_string()))?;
    let first_row = range.start().map_or(0, |(row, _)| row);
    let mut collector = RowCollector::default();
    for (offset, cells) in range.rows().enumerate() {
        let line = first_row + offset as u32 + 1;
        collector.push(
            line,
            cells
                .iter()
                .map(|cell| clean_cell(&spreadsheet_text(cell)))
                .collect(),
        )?;
    }
    collector.finish(TextEncoding::Utf8, b',')
}

fn spreadsheet_text(cell: &Data) -> String {
    match cell {
        Data::Empty | Data::Error(_) => String::new(),
        Data::String(text) | Data::DateTimeIso(text) | Data::DurationIso(text) => text.clone(),
        Data::Int(value) => value.to_string(),
        Data::Float(value) => {
            if value.fract() == 0.0 && value.abs() < 1e15 {
                format!("{value:.0}")
            } else {
                value.to_string()
            }
        }
        Data::Bool(value) => value.to_string(),
        Data::DateTime(value) => match value.as_datetime() {
            Some(moment) if moment.time() == NaiveTime::MIN => {
                moment.date().format("%Y-%m-%d").to_string()
            }
            Some(moment) => moment.format("%Y-%m-%d %H:%M:%S").to_string(),
            None => value.as_f64().to_string(),
        },
    }
}

#[derive(Default)]
struct RowCollector {
    headers: Option<Vec<String>>,
    rows: Vec<SourceRow>,
}

impl RowCollector {
    fn push(&mut self, row_index: u32, cells: Vec<String>) -> Result<(), AppError> {
        if cells.iter().all(|cell| cell.is_empty()) {
            return Ok(());
        }
        if self.headers.is_none() {
            if cells.len() > MAX_IMPORT_COLUMNS {
                return Err(AppError::validation_code(
                    TOO_MANY_COLUMNS,
                    format!("the file has more than {MAX_IMPORT_COLUMNS} columns"),
                ));
            }
            self.headers = Some(header_names(trim_trailing_empty(cells)));
            return Ok(());
        }
        if self.rows.len() == MAX_IMPORT_ROWS {
            return Err(AppError::validation_code(
                TOO_MANY_ROWS,
                format!("the file has more than {MAX_IMPORT_ROWS} rows"),
            ));
        }
        self.rows.push(SourceRow { row_index, cells });
        Ok(())
    }

    fn finish(self, encoding: TextEncoding, delimiter: u8) -> Result<ParsedSource, AppError> {
        let headers = self.headers.unwrap_or_default();
        if headers.is_empty() || self.rows.is_empty() {
            return Err(AppError::validation_code(
                EMPTY_IMPORT,
                "the file must have a header line and at least one data row",
            ));
        }
        Ok(ParsedSource {
            encoding,
            delimiter,
            headers,
            rows: self.rows,
        })
    }
}

fn trim_trailing_empty(mut cells: Vec<String>) -> Vec<String> {
    while cells.last().is_some_and(String::is_empty) {
        cells.pop();
    }
    cells
}

fn clean_cell(cell: &str) -> String {
    let trimmed = restore_neutralized(cell.trim());
    if trimmed.chars().count() > MAX_CELL_LENGTH {
        return trimmed.chars().take(MAX_CELL_LENGTH).collect();
    }
    trimmed.to_owned()
}

fn header_names(cells: Vec<String>) -> Vec<String> {
    cells
        .into_iter()
        .enumerate()
        .map(|(index, cell)| {
            if cell.is_empty() {
                format!("Coluna {}", index + 1)
            } else {
                cell
            }
        })
        .collect()
}

pub(crate) fn delimiter_label(delimiter: u8) -> String {
    match delimiter {
        b'\t' => "\\t".to_owned(),
        other => (other as char).to_string(),
    }
}

pub(crate) fn delimiter_from_label(label: &str) -> Option<u8> {
    match label {
        "\\t" => Some(b'\t'),
        "," => Some(b','),
        ";" => Some(b';'),
        "|" => Some(b'|'),
        _ => None,
    }
}
