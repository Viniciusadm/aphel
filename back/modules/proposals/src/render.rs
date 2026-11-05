use serde::Serialize;
use utoipa::ToSchema;

use crate::format;
use crate::markup::{Block, Inline, Line, Template};
use crate::variables::{Catalog, RenderData};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
#[schema(as = ProposalSpan)]
pub(crate) struct Span {
    pub(crate) text: String,
    pub(crate) bold: bool,
    pub(crate) variable: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
#[schema(as = ProposalTableRow)]
pub(crate) struct TableRow {
    pub(crate) name: String,
    pub(crate) quantity: String,
    pub(crate) unit_price: String,
    pub(crate) discount: String,
    pub(crate) total: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
#[schema(as = ProposalItemsTable)]
pub(crate) struct ItemsTable {
    pub(crate) currency: String,
    pub(crate) available: bool,
    pub(crate) rows: Vec<TableRow>,
    pub(crate) subtotal: String,
    pub(crate) discount_total: String,
    pub(crate) tax_total: String,
    pub(crate) total: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
#[schema(as = ProposalBlock)]
pub(crate) enum RenderedBlock {
    Heading { level: u8, spans: Vec<Span> },
    Paragraph { lines: Vec<Vec<Span>> },
    List { items: Vec<Vec<Span>> },
    ItemsTable { table: ItemsTable },
    PageBreak,
    Rule,
}

fn spans(line: &Line, data: &RenderData, catalog: &Catalog) -> Vec<Span> {
    line.iter()
        .map(|run| match &run.inline {
            Inline::Text(text) => Span {
                text: text.clone(),
                bold: run.bold,
                variable: None,
            },
            Inline::Variable { path, .. } => Span {
                text: data.resolve(path, catalog),
                bold: run.bold,
                variable: Some(path.clone()),
            },
        })
        .collect()
}

fn tax_total(items: &crate::variables::Items) -> String {
    use rust_decimal::Decimal;
    use std::str::FromStr;
    let amount = |raw: &str| Decimal::from_str(raw).unwrap_or_default();
    let taxes = amount(&items.total) - (amount(&items.subtotal) - amount(&items.discount_total));
    taxes.max(Decimal::ZERO).to_string()
}

fn items_table(data: &RenderData) -> ItemsTable {
    let currency = data.currency();
    let locale = data.locale;
    let money = |raw: &str| format::money(raw, &currency, locale);
    match &data.items {
        Some(items) => ItemsTable {
            currency: currency.clone(),
            available: true,
            rows: items
                .rows
                .iter()
                .map(|row| TableRow {
                    name: match &row.unit {
                        Some(unit) if !unit.is_empty() => format!("{} ({unit})", row.name),
                        _ => row.name.clone(),
                    },
                    quantity: format::number(&row.quantity, locale),
                    unit_price: money(&row.unit_price),
                    discount: match row.discount_type.as_str() {
                        "percent" => {
                            format!("{}%", format::decimal(&row.discount_value, 0, 2, locale))
                        }
                        "amount" => money(&row.discount_amount),
                        _ => "—".to_owned(),
                    },
                    total: money(&row.total),
                })
                .collect(),
            subtotal: money(&items.subtotal),
            discount_total: money(&items.discount_total),
            tax_total: money(&tax_total(items)),
            total: money(&items.total),
        },
        None => ItemsTable {
            currency: currency.clone(),
            available: false,
            rows: Vec::new(),
            subtotal: money("0"),
            discount_total: money("0"),
            tax_total: money("0"),
            total: money("0"),
        },
    }
}

pub(crate) fn render(
    template: &Template,
    data: &RenderData,
    catalog: &Catalog,
) -> Vec<RenderedBlock> {
    template
        .blocks
        .iter()
        .map(|block| match block {
            Block::Heading { level, line } => RenderedBlock::Heading {
                level: *level,
                spans: spans(line, data, catalog),
            },
            Block::Paragraph(lines) => RenderedBlock::Paragraph {
                lines: lines
                    .iter()
                    .map(|line| spans(line, data, catalog))
                    .collect(),
            },
            Block::List(items) => RenderedBlock::List {
                items: items
                    .iter()
                    .map(|line| spans(line, data, catalog))
                    .collect(),
            },
            Block::ItemsTable => RenderedBlock::ItemsTable {
                table: items_table(data),
            },
            Block::PageBreak => RenderedBlock::PageBreak,
            Block::Rule => RenderedBlock::Rule,
        })
        .collect()
}
