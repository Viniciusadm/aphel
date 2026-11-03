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

pub(crate) fn document(header: Header, blocks: &[Block]) -> Result<Vec<u8>, AppError> {
    let columns = || {
        ["Item", "Qtd.", "Preço unit.", "Desconto", "Total"]
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
                        ("Subtotal", &table.subtotal, false),
                        ("Descontos", &table.discount_total, false),
                        ("Impostos", &table.tax_total, false),
                        ("Total", &table.total, true),
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
            page_label: "Página".into(),
            page_of_label: "de".into(),
        },
        blocks,
    })
}
