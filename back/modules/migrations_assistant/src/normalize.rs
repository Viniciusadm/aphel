use std::collections::{BTreeMap, BTreeSet, HashMap};

use bytes::Bytes;

use crate::{ColumnRule, DateOrder, DecimalStyle, SourceStep, SourceTool, Transform};
use aphel_kernel::errors::AppError;
use imports::{ParsedSource, normalize_label};

const MAX_DISTINCT_VALUES: usize = 200;
const DERIVED_STATUS_HEADER: &str = "Aphel - Status";
const DEAL_RECORD_TYPE: &str = "deal";
const STATUS_TARGET: &str = "status";
const WON_STATUS: &str = "won";
const LOST_STATUS: &str = "lost";
const OPEN_STATUS: &str = "open";
const DONE_STATUS: &str = "done";

const WON_ALIASES: &[&str] = &[
    "won",
    "ganho",
    "ganha",
    "ganhou",
    "vendida",
    "vendido",
    "closed won",
    "closedwon",
    "fechado ganho",
    "fechada ganha",
    "negocio ganho",
];
const LOST_ALIASES: &[&str] = &[
    "lost",
    "perdido",
    "perdida",
    "perdeu",
    "closed lost",
    "closedlost",
    "fechado perdido",
    "fechada perdida",
    "negocio perdido",
];
const OPEN_ALIASES: &[&str] = &[
    "open",
    "aberto",
    "aberta",
    "em aberto",
    "em andamento",
    "andamento",
    "ativo",
    "ativa",
];
const DONE_ALIASES: &[&str] = &[
    "done",
    "completed",
    "complete",
    "concluida",
    "concluido",
    "finalizada",
    "finalizado",
    "feita",
    "feito",
    "sim",
    "yes",
    "true",
    "1",
    "x",
];
const NOT_DONE_ALIASES: &[&str] = &[
    "not done",
    "not started",
    "not_started",
    "in progress",
    "in_progress",
    "waiting",
    "deferred",
    "pendente",
    "nao",
    "no",
    "false",
    "0",
    "aberta",
    "aberto",
    "em andamento",
];

pub(crate) fn deal_status(raw: &str) -> Option<&'static str> {
    let label = normalize_label(raw);
    if WON_ALIASES.contains(&label.as_str()) {
        Some(WON_STATUS)
    } else if LOST_ALIASES.contains(&label.as_str()) {
        Some(LOST_STATUS)
    } else if OPEN_ALIASES.contains(&label.as_str()) {
        Some(OPEN_STATUS)
    } else {
        None
    }
}

pub(crate) fn closed_status(raw: &str) -> Option<&'static str> {
    deal_status(raw).filter(|status| *status != OPEN_STATUS)
}

pub(crate) fn done_flag(raw: &str) -> Option<&'static str> {
    let label = normalize_label(raw);
    if DONE_ALIASES.contains(&label.as_str()) {
        Some(DONE_STATUS)
    } else if NOT_DONE_ALIASES.contains(&label.as_str()) {
        Some(OPEN_STATUS)
    } else {
        None
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DirectoryMember {
    pub(crate) display_name: String,
    pub(crate) email: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct ValueMaps {
    pub(crate) stages: HashMap<String, String>,
    pub(crate) pipelines: HashMap<String, String>,
    pub(crate) owners: HashMap<String, String>,
}

impl ValueMaps {
    pub(crate) fn normalized(self) -> Self {
        let fold = |map: HashMap<String, String>| {
            map.into_iter()
                .map(|(source, target)| (normalize_label(&source), target.trim().to_owned()))
                .filter(|(source, _)| !source.is_empty())
                .collect()
        };
        Self {
            stages: fold(self.stages),
            pipelines: fold(self.pipelines),
            owners: fold(self.owners),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RecognizedColumn {
    pub(crate) index: usize,
    pub(crate) header: String,
    pub(crate) target: String,
    pub(crate) transform: Transform,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct NormalizedFile {
    pub(crate) headers: Vec<String>,
    pub(crate) rows: Vec<Vec<String>>,
    pub(crate) recognized: Vec<RecognizedColumn>,
    pub(crate) unrecognized: Vec<String>,
    pub(crate) changed_cells: u64,
    pub(crate) issues: BTreeMap<String, u64>,
    pub(crate) distinct: BTreeMap<String, Vec<String>>,
}

pub(crate) fn recognize(step: &SourceStep, headers: &[String]) -> Vec<Option<&'static ColumnRule>> {
    let normalized: Vec<String> = headers
        .iter()
        .map(|header| normalize_label(header))
        .collect();
    let mut result: Vec<Option<&'static ColumnRule>> = vec![None; headers.len()];
    let mut used: BTreeSet<&str> = BTreeSet::new();
    for rule in step.columns {
        for alias in rule.headers {
            let alias = normalize_label(alias);
            let Some(index) = normalized
                .iter()
                .enumerate()
                .position(|(index, header)| result[index].is_none() && *header == alias)
            else {
                continue;
            };
            if used.insert(rule.target) {
                result[index] = Some(rule);
            }
            break;
        }
    }
    result
}

pub(crate) fn normalize(
    tool: &SourceTool,
    step: &SourceStep,
    source: &ParsedSource,
    members: &[DirectoryMember],
    maps: &ValueMaps,
) -> NormalizedFile {
    let rules = recognize(step, &source.headers);
    let members: HashMap<String, String> = members
        .iter()
        .map(|member| {
            (
                normalize_label(&member.display_name),
                member.email.to_lowercase(),
            )
        })
        .collect();
    let mut file = NormalizedFile {
        headers: source.headers.clone(),
        ..NormalizedFile::default()
    };
    for (index, rule) in rules.iter().enumerate() {
        match rule {
            Some(rule) => file.recognized.push(RecognizedColumn {
                index,
                header: source.headers[index].clone(),
                target: rule.target.to_owned(),
                transform: rule.transform,
            }),
            None => file.unrecognized.push(source.headers[index].clone()),
        }
    }
    let status_index = rules
        .iter()
        .position(|rule| rule.is_some_and(|rule| rule.transform == Transform::Status));
    let stage_index = rules
        .iter()
        .position(|rule| rule.is_some_and(|rule| rule.transform == Transform::Stage));
    let derive_status =
        step.record_type == DEAL_RECORD_TYPE && status_index.is_none() && stage_index.is_some();
    if derive_status {
        file.recognized.push(RecognizedColumn {
            index: source.headers.len(),
            header: DERIVED_STATUS_HEADER.to_owned(),
            target: STATUS_TARGET.to_owned(),
            transform: Transform::Status,
        });
        file.headers.push(DERIVED_STATUS_HEADER.to_owned());
    }
    let mut distinct: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for row in &source.rows {
        let mut cells = row.cells.clone();
        cells.resize(source.headers.len(), String::new());
        let raw_stage = stage_index.map(|index| cells[index].trim().to_owned());
        for (index, rule) in rules.iter().enumerate() {
            let Some(rule) = rule else { continue };
            let raw = cells[index].trim().to_owned();
            if raw.is_empty() {
                continue;
            }
            if matches!(
                rule.transform,
                Transform::Stage | Transform::Pipeline | Transform::Owner | Transform::Status
            ) {
                let values = distinct.entry(rule.target.to_owned()).or_default();
                if values.len() < MAX_DISTINCT_VALUES {
                    values.insert(raw.clone());
                }
            }
            let converted = convert(tool, rule.transform, &raw, &members, maps);
            let value = match converted {
                Some(value) => value,
                None => {
                    *file.issues.entry(rule.target.to_owned()).or_default() += 1;
                    if rule.transform == Transform::Owner {
                        String::new()
                    } else {
                        raw.clone()
                    }
                }
            };
            if value != cells[index] {
                file.changed_cells += 1;
                cells[index] = value;
            }
        }
        if step.record_type == DEAL_RECORD_TYPE {
            let closed = match status_index {
                Some(index) => closed_status(&cells[index]),
                None => raw_stage.as_deref().and_then(closed_status),
            };
            if derive_status {
                cells.push(closed.unwrap_or_default().to_owned());
            }
            if closed.is_some()
                && let Some(index) = stage_index
                && !cells[index].is_empty()
            {
                cells[index] = String::new();
                file.changed_cells += 1;
            }
        }
        file.rows.push(cells);
    }
    file.distinct = distinct
        .into_iter()
        .map(|(target, values)| (target, values.into_iter().collect()))
        .collect();
    file
}

pub(crate) fn encode(file: &NormalizedFile) -> Result<Bytes, AppError> {
    let mut writer = ::csv::WriterBuilder::new().from_writer(Vec::new());
    let error = |error: ::csv::Error| AppError::Internal(format!("cannot write the CSV: {error}"));
    writer.write_record(&file.headers).map_err(error)?;
    for row in &file.rows {
        writer.write_record(row).map_err(error)?;
    }
    writer
        .into_inner()
        .map(Bytes::from)
        .map_err(|error| AppError::Internal(format!("cannot write the CSV: {error}")))
}

fn convert(
    tool: &SourceTool,
    transform: Transform,
    raw: &str,
    members: &HashMap<String, String>,
    maps: &ValueMaps,
) -> Option<String> {
    match transform {
        Transform::Keep => Some(raw.to_owned()),
        Transform::Email => first_item(raw).map(str::to_lowercase),
        Transform::Phone => first_item(raw).map(normalize_phone),
        Transform::Money => money(raw, tool.decimal),
        Transform::Date => date(raw, tool.date_order).map(|(date, _)| date),
        Transform::DateTime => date(raw, tool.date_order).map(|(date, time)| match time {
            Some(time) => format!("{date} {time}"),
            None => date,
        }),
        Transform::Percent => percent(raw, tool.decimal),
        Transform::Owner => owner(raw, members, maps),
        Transform::EmployeeCount => employee_range(raw),
        Transform::Stage => Some(mapped(raw, &maps.stages)),
        Transform::Pipeline => Some(mapped(raw, &maps.pipelines)),
        Transform::Status => deal_status(raw).map(str::to_owned),
        Transform::Done => done_flag(raw).map(str::to_owned),
    }
}

fn first_item(raw: &str) -> Option<&str> {
    raw.split([',', ';', '|'])
        .map(str::trim)
        .find(|item| !item.is_empty())
}

fn mapped(raw: &str, map: &HashMap<String, String>) -> String {
    map.get(&normalize_label(raw))
        .filter(|target| !target.is_empty())
        .cloned()
        .unwrap_or_else(|| raw.to_owned())
}

pub(crate) fn normalize_phone(raw: &str) -> String {
    let digits: String = raw.chars().filter(char::is_ascii_digit).collect();
    if raw.trim_start().starts_with('+') && digits.len() >= 8 {
        return format!("+{digits}");
    }
    let digits = digits.trim_start_matches('0');
    match digits.len() {
        10 | 11 => format!("+55{digits}"),
        12 | 13 if digits.starts_with("55") => format!("+{digits}"),
        _ => raw.to_owned(),
    }
}

pub(crate) fn money(raw: &str, style: DecimalStyle) -> Option<String> {
    let negative = raw.contains('-') || (raw.contains('(') && raw.contains(')'));
    let cleaned: String = raw
        .chars()
        .filter(|character| character.is_ascii_digit() || matches!(character, '.' | ','))
        .collect();
    if cleaned.is_empty() {
        return None;
    }
    let normalized = match style {
        DecimalStyle::Comma => cleaned.replace('.', "").replace(',', "."),
        DecimalStyle::Dot => cleaned.replace(',', ""),
    };
    let value: f64 = normalized.parse().ok()?;
    let value = if negative { -value } else { value };
    Some(format!("{value:.2}"))
}

fn percent(raw: &str, style: DecimalStyle) -> Option<String> {
    let fraction = !raw.contains('%');
    let number = money(raw, style)?.parse::<f64>().ok()?;
    let value = if fraction && number > 0.0 && number <= 1.0 && raw.contains(['.', ',']) {
        number * 100.0
    } else {
        number
    };
    if !(0.0..=100.0).contains(&value) {
        return None;
    }
    Some(format!("{}", value.round() as i64))
}

pub(crate) fn date(raw: &str, order: DateOrder) -> Option<(String, Option<String>)> {
    let raw = raw.trim();
    let (date_part, time_part) = match raw.split_once(['T', ' ']) {
        Some((date, time)) => (date.trim(), Some(time.trim())),
        None => (raw, None),
    };
    let parts: Vec<&str> = date_part.split(['/', '-', '.']).collect();
    let [first, second, third] = parts.as_slice() else {
        return None;
    };
    let numbers: Vec<u32> = [first, second, third]
        .iter()
        .map(|part| part.parse::<u32>().ok())
        .collect::<Option<_>>()?;
    let (year, month, day) = if first.len() == 4 {
        (numbers[0], numbers[1], numbers[2])
    } else {
        let year = if third.len() == 2 {
            2000 + numbers[2]
        } else {
            numbers[2]
        };
        let (month, day) = match order {
            _ if numbers[0] > 12 => (numbers[1], numbers[0]),
            _ if numbers[1] > 12 => (numbers[0], numbers[1]),
            DateOrder::DayFirst => (numbers[1], numbers[0]),
            DateOrder::MonthFirst => (numbers[0], numbers[1]),
        };
        (year, month, day)
    };
    chrono::NaiveDate::from_ymd_opt(year as i32, month, day)?;
    let time = time_part
        .map(|time| time.trim_end_matches('Z'))
        .and_then(|time| {
            let pieces: Vec<&str> = time.split(':').collect();
            match pieces.as_slice() {
                [hour, minute, ..] => {
                    let hour: u32 = hour.parse().ok()?;
                    let minute: u32 = minute.get(..2).unwrap_or(minute).parse().ok()?;
                    (hour < 24 && minute < 60).then(|| format!("{hour:02}:{minute:02}"))
                }
                _ => None,
            }
        });
    Some((format!("{year:04}-{month:02}-{day:02}"), time))
}

fn owner(raw: &str, members: &HashMap<String, String>, maps: &ValueMaps) -> Option<String> {
    let label = normalize_label(raw);
    if let Some(email) = maps.owners.get(&label).filter(|email| email.contains('@')) {
        return Some(email.to_lowercase());
    }
    if raw.contains('@') {
        return first_item(raw).map(str::to_lowercase);
    }
    members.get(&label).cloned()
}

fn employee_range(raw: &str) -> Option<String> {
    let first: String = raw
        .trim()
        .chars()
        .skip_while(|character| !character.is_ascii_digit())
        .take_while(|character| character.is_ascii_digit() || matches!(character, '.' | ','))
        .filter(char::is_ascii_digit)
        .collect();
    let count: u64 = first.parse().ok()?;
    let range = match count {
        0..=10 => "size_1_10",
        11..=50 => "size_11_50",
        51..=200 => "size_51_200",
        201..=500 => "size_201_500",
        501..=1000 => "size_501_1000",
        _ => "size_1001_plus",
    };
    Some(range.to_owned())
}
