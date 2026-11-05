use chrono::{Datelike, NaiveDate};
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;

use aphel_sdk::exports::ExportCell;
use aphel_sdk::platform::application::query::{
    AggregateFunction, AggregateGroupColumn, AggregateMetricColumn, AggregateResult, GroupValue,
    TimeGranularity,
};
use aphel_sdk::platform::domain::fields::FieldType;

pub(crate) const EMPTY_GROUP_LABEL: &str = "(vazio)";
pub(crate) const TOTAL_LABEL: &str = "Total";

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ReportCell {
    Text(String),
    Number { value: Decimal, currency: bool },
    Ratio(Decimal),
    Empty,
}

impl ReportCell {
    pub(crate) fn display(&self) -> String {
        match self {
            Self::Text(text) => text.clone(),
            Self::Number { value, currency } => format_decimal(*value, *currency),
            Self::Ratio(ratio) => {
                let percent = (*ratio * Decimal::ONE_HUNDRED).round_dp(1);
                let sign = if percent > Decimal::ZERO { "+" } else { "" };
                format!("{sign}{}%", format_plain(percent, 1))
            }
            Self::Empty => String::new(),
        }
    }

    pub(crate) fn is_numeric(&self) -> bool {
        matches!(self, Self::Number { .. } | Self::Ratio(_))
    }

    pub(crate) fn to_export(&self) -> ExportCell {
        match self {
            Self::Text(text) => ExportCell::Text(text.clone()),
            Self::Number { value, .. } => {
                value.to_f64().map_or(ExportCell::Empty, ExportCell::Number)
            }
            Self::Ratio(ratio) => ratio
                .round_dp(4)
                .to_f64()
                .map_or(ExportCell::Empty, ExportCell::Number),
            Self::Empty => ExportCell::Empty,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ReportTable {
    pub(crate) headers: Vec<String>,
    pub(crate) rows: Vec<Vec<ReportCell>>,
    pub(crate) totals: Vec<ReportCell>,
}

fn format_plain(value: Decimal, decimals: u32) -> String {
    let rounded = value.round_dp(decimals);
    let negative = rounded.is_sign_negative() && !rounded.is_zero();
    let text = rounded.abs().to_string();
    let (integer, fraction) = match text.split_once('.') {
        Some((integer, fraction)) => (integer.to_owned(), Some(fraction.to_owned())),
        None => (text, None),
    };
    let mut grouped = String::new();
    for (index, digit) in integer.chars().enumerate() {
        if index > 0 && (integer.len() - index) % 3 == 0 {
            grouped.push('.');
        }
        grouped.push(digit);
    }
    let sign = if negative { "-" } else { "" };
    match fraction.filter(|fraction| !fraction.is_empty()) {
        Some(fraction) => format!("{sign}{grouped},{fraction}"),
        None => format!("{sign}{grouped}"),
    }
}

pub(crate) fn format_decimal(value: Decimal, currency: bool) -> String {
    if currency {
        let text = format_plain(value.round_dp(2), 2);
        match text.split_once(',') {
            Some((integer, fraction)) if fraction.len() == 1 => format!("{integer},{fraction}0"),
            Some(_) => text,
            None => format!("{text},00"),
        }
    } else {
        format_plain(value.normalize(), 4)
    }
}

pub(crate) fn function_label(function: AggregateFunction) -> &'static str {
    match function {
        AggregateFunction::Count => "Quantidade",
        AggregateFunction::Sum => "Soma",
        AggregateFunction::Avg => "Média",
        AggregateFunction::Min => "Mínimo",
        AggregateFunction::Max => "Máximo",
    }
}

pub(crate) fn metric_label(metric: &AggregateMetricColumn) -> String {
    let function = function_label(metric.function);
    match (&metric.field_label, &metric.field) {
        (Some(label), _) => format!("{function} de {label}"),
        (None, Some(field)) => format!("{function} de {field}"),
        (None, None) => function.to_owned(),
    }
}

fn quarter_of(day: NaiveDate) -> u32 {
    (day.month() - 1) / 3 + 1
}

fn bucket_label(granularity: Option<TimeGranularity>, value: &str) -> String {
    let Ok(day) = NaiveDate::parse_from_str(value, "%Y-%m-%d") else {
        return value.to_owned();
    };
    match granularity.unwrap_or(TimeGranularity::Day) {
        TimeGranularity::Day => day.format("%d/%m/%Y").to_string(),
        TimeGranularity::Week => format!("Semana de {}", day.format("%d/%m/%Y")),
        TimeGranularity::Month => day.format("%m/%Y").to_string(),
        TimeGranularity::Quarter => format!("{}º tri/{}", quarter_of(day), day.year()),
        TimeGranularity::Year => day.year().to_string(),
    }
}

pub(crate) fn group_label(column: &AggregateGroupColumn, group: &GroupValue) -> String {
    if let Some(label) = group.label.as_ref().filter(|label| !label.is_empty()) {
        return label.clone();
    }
    let Some(value) = group.value.as_ref() else {
        return EMPTY_GROUP_LABEL.to_owned();
    };
    match column.field_type.base_type() {
        FieldType::Date | FieldType::DateTime => bucket_label(column.granularity, value),
        FieldType::Checkbox => match value.as_str() {
            "true" => "Sim".to_owned(),
            _ => "Não".to_owned(),
        },
        _ => value.clone(),
    }
}

fn metric_cell(metric: &AggregateMetricColumn, value: Option<Decimal>) -> ReportCell {
    match value {
        Some(value) => ReportCell::Number {
            value,
            currency: metric.field_type == Some(FieldType::Currency)
                && metric.function != AggregateFunction::Count,
        },
        None => ReportCell::Empty,
    }
}

fn variation(current: Option<Decimal>, previous: Option<Decimal>) -> ReportCell {
    match (current, previous) {
        (Some(current), Some(previous)) if !previous.is_zero() => {
            ReportCell::Ratio((current - previous) / previous.abs())
        }
        _ => ReportCell::Empty,
    }
}

fn value_cells(
    metrics: &[AggregateMetricColumn],
    values: &[Option<Decimal>],
    previous: Option<&Vec<Option<Decimal>>>,
    compare: bool,
) -> Vec<ReportCell> {
    let mut cells = Vec::new();
    for (index, metric) in metrics.iter().enumerate() {
        let current = values.get(index).copied().flatten();
        cells.push(metric_cell(metric, current));
        if compare {
            let before = previous.and_then(|values| values.get(index).copied().flatten());
            cells.push(metric_cell(metric, before));
            cells.push(variation(current, before));
        }
    }
    cells
}

pub(crate) fn report_table(result: &AggregateResult) -> ReportTable {
    let compare = result.previous_period.is_some();
    let mut headers: Vec<String> = result
        .groups
        .iter()
        .map(|group| group.label.clone())
        .collect();
    for metric in &result.metrics {
        let label = metric_label(metric);
        if compare {
            headers.push(format!("{label} (anterior)"));
            headers.push(format!("{label} (variação)"));
            let position = headers.len() - 2;
            headers.insert(position, label);
        } else {
            headers.push(label);
        }
    }
    let rows = result
        .rows
        .iter()
        .map(|row| {
            let mut cells: Vec<ReportCell> = result
                .groups
                .iter()
                .zip(&row.groups)
                .map(|(column, group)| ReportCell::Text(group_label(column, group)))
                .collect();
            cells.extend(value_cells(
                &result.metrics,
                &row.values,
                row.previous.as_ref(),
                compare,
            ));
            cells
        })
        .collect();
    let mut totals: Vec<ReportCell> = (0..result.groups.len())
        .map(|index| {
            if index == 0 {
                ReportCell::Text(TOTAL_LABEL.to_owned())
            } else {
                ReportCell::Empty
            }
        })
        .collect();
    totals.extend(value_cells(
        &result.metrics,
        &result.totals.values,
        result.totals.previous.as_ref(),
        compare,
    ));
    ReportTable {
        headers,
        rows,
        totals,
    }
}
