use aphel_sdk::{AppError, documents as doc};
pub(crate) struct Header {
    pub(crate) organization: String,
    pub(crate) title: String,
    pub(crate) subtitle: String,
}

pub(crate) struct TableRow {
    pub(crate) name: String,
    pub(crate) quantity: String,
    pub(crate) unit_price: String,
    pub(crate) discount: String,
    pub(crate) total: String,
}

pub(crate) struct ItemsTable {
    pub(crate) rows: Vec<TableRow>,
    pub(crate) subtotal: String,
    pub(crate) discount_total: String,
    pub(crate) tax_total: String,
    pub(crate) total: String,
}

pub(crate) enum Block {
    Heading(String),
    Field { label: String, value: String },
    Gap,
    Paragraph(String),
    Items(ItemsTable),
}

pub(crate) struct Labels {
    pub(crate) columns: [&'static str; 5],
    pub(crate) subtotal: &'static str,
    pub(crate) discounts: &'static str,
    pub(crate) taxes: &'static str,
    pub(crate) total: &'static str,
    pub(crate) page: &'static str,
    pub(crate) page_of: &'static str,
}

impl Labels {
    pub(crate) fn for_locale(locale: &str) -> Self {
        if locale.starts_with("en") {
            Self {
                columns: ["Item", "Qty.", "Unit price", "Discount", "Total"],
                subtotal: "Subtotal",
                discounts: "Discounts",
                taxes: "Taxes",
                total: "Total",
                page: "Page",
                page_of: "of",
            }
        } else {
            Self {
                columns: ["Item", "Qtd.", "Preço unit.", "Desconto", "Total"],
                subtotal: "Subtotal",
                discounts: "Descontos",
                taxes: "Impostos",
                total: "Total",
                page: "Página",
                page_of: "de",
            }
        }
    }
}

pub(crate) fn document(
    header: Header,
    blocks: &[Block],
    labels: &Labels,
) -> Result<Vec<u8>, AppError> {
    let columns = || {
        labels
            .columns
            .into_iter()
            .zip([203.28, 44.0, 82.0, 68.0, 86.0])
            .enumerate()
            .map(|(index, (label, weight))| doc::TableColumn {
                label: label.into(),
                weight,
                right_aligned: index != 0,
            })
            .collect()
    };
    let blocks = blocks
        .iter()
        .map(|block| match block {
            Block::Heading(text) => doc::Block::Heading {
                level: 3,
                spans: vec![doc::Span {
                    text: text.clone(),
                    bold: true,
                }],
            },
            Block::Field { label, value } => doc::Block::Field {
                label: label.clone(),
                value: value.clone(),
            },
            Block::Gap => doc::Block::Gap,
            Block::Paragraph(text) => doc::Block::Paragraph {
                lines: text
                    .lines()
                    .map(|text| {
                        vec![doc::Span {
                            text: text.into(),
                            bold: false,
                        }]
                    })
                    .collect(),
            },
            Block::Items(table) => doc::Block::Table {
                table: doc::Table {
                    columns: columns(),
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
                        (labels.subtotal, &table.subtotal, false),
                        (labels.discounts, &table.discount_total, false),
                        (labels.taxes, &table.tax_total, false),
                        (labels.total, &table.total, true),
                    ]
                    .into_iter()
                    .map(|(label, value, bold)| doc::SummaryLine {
                        label: label.into(),
                        value: value.clone(),
                        bold,
                    })
                    .collect(),
                    empty_message: None,
                },
            },
        })
        .collect();
    doc::render_pdf(&doc::Document {
        header: doc::DocumentHeader {
            issuer: header.organization,
            title: header.title,
            subtitle: header.subtitle,
            page_label: labels.page.into(),
            page_of_label: labels.page_of.into(),
        },
        blocks,
    })
}
