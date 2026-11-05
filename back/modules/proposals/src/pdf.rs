use crate::{format::Locale, render::RenderedBlock};
use aphel_sdk::{AppError, documents as doc};
pub(crate) struct Header {
    pub(crate) organization: String,
    pub(crate) title: String,
    pub(crate) subtitle: String,
    pub(crate) locale: Locale,
}
pub(crate) fn document(header: Header, blocks: &[RenderedBlock]) -> Result<Vec<u8>, AppError> {
    let text = header.locale.labels();
    let spans = |spans: &[crate::render::Span]| {
        spans
            .iter()
            .map(|span| doc::Span {
                text: span.text.clone(),
                bold: span.bold,
            })
            .collect()
    };
    let blocks = blocks
        .iter()
        .map(|block| match block {
            RenderedBlock::Heading {
                level,
                spans: values,
            } => doc::Block::Heading {
                level: *level,
                spans: spans(values),
            },
            RenderedBlock::Paragraph { lines } => doc::Block::Paragraph {
                lines: lines.iter().map(|line| spans(line)).collect(),
            },
            RenderedBlock::List { items } => doc::Block::List {
                items: items.iter().map(|line| spans(line)).collect(),
            },
            RenderedBlock::PageBreak => doc::Block::PageBreak,
            RenderedBlock::Rule => doc::Block::Rule,
            RenderedBlock::ItemsTable { table } => doc::Block::Table {
                table: doc::Table {
                    columns: [
                        text.item,
                        text.quantity,
                        text.unit_price,
                        text.discount,
                        text.total,
                    ]
                    .into_iter()
                    .zip([203.28, 44.0, 82.0, 68.0, 86.0])
                    .enumerate()
                    .map(|(index, (label, weight))| doc::TableColumn {
                        label: label.into(),
                        weight,
                        right_aligned: index != 0,
                    })
                    .collect(),
                    rows: table
                        .rows
                        .iter()
                        .map(|row| {
                            vec![
                                row.name.clone(),
                                row.quantity.clone(),
                                row.unit_price.clone(),
                                row.discount.clone(),
                                row.total.clone(),
                            ]
                        })
                        .collect(),
                    summary: [
                        (text.subtotal, &table.subtotal, false),
                        (text.discounts, &table.discount_total, false),
                        (text.taxes, &table.tax_total, false),
                        (text.total, &table.total, true),
                    ]
                    .into_iter()
                    .map(|(label, value, bold)| doc::SummaryLine {
                        label: label.into(),
                        value: value.clone(),
                        bold,
                    })
                    .collect(),
                    empty_message: Some(
                        if table.available {
                            text.no_items
                        } else {
                            text.items_unavailable
                        }
                        .into(),
                    ),
                },
            },
        })
        .collect();
    doc::render_pdf(&doc::Document {
        header: doc::DocumentHeader {
            issuer: header.organization,
            title: header.title,
            subtitle: header.subtitle,
            page_label: text.page.into(),
            page_of_label: text.page_of.into(),
        },
        blocks,
    })
}
