use std::fmt::Write as _;

use aphel_sdk::{
    AppError,
    documents::{Block, Document, DocumentHeader, Span, Table},
};

const HELVETICA: [u16; 224] = [
    278, 278, 355, 556, 556, 889, 667, 191, 333, 333, 389, 584, 278, 333, 278, 278, 556, 556, 556,
    556, 556, 556, 556, 556, 556, 556, 278, 278, 584, 584, 584, 556, 1015, 667, 667, 722, 722, 667,
    611, 778, 722, 278, 500, 667, 556, 833, 722, 778, 667, 778, 722, 667, 611, 722, 667, 944, 667,
    667, 611, 278, 278, 278, 469, 556, 333, 556, 556, 500, 556, 556, 278, 556, 556, 222, 222, 500,
    222, 833, 556, 556, 556, 556, 333, 500, 278, 556, 500, 722, 500, 500, 500, 334, 260, 334, 584,
    0, 556, 0, 222, 556, 333, 1000, 556, 556, 333, 1000, 667, 333, 1000, 0, 611, 0, 0, 222, 222,
    333, 333, 350, 556, 1000, 333, 1000, 500, 333, 944, 0, 500, 667, 278, 333, 556, 556, 556, 556,
    260, 556, 333, 737, 370, 556, 584, 333, 737, 333, 400, 584, 333, 333, 333, 556, 537, 278, 333,
    333, 365, 556, 834, 834, 834, 611, 667, 667, 667, 667, 667, 667, 1000, 722, 667, 667, 667, 667,
    278, 278, 278, 278, 722, 722, 778, 778, 778, 778, 778, 584, 778, 722, 722, 722, 722, 667, 667,
    611, 556, 556, 556, 556, 556, 556, 889, 500, 556, 556, 556, 556, 278, 278, 278, 278, 556, 556,
    556, 556, 556, 556, 556, 584, 611, 556, 556, 556, 556, 500, 556, 500,
];

const HELVETICA_BOLD: [u16; 224] = [
    278, 333, 474, 556, 556, 889, 722, 238, 333, 333, 389, 584, 278, 333, 278, 278, 556, 556, 556,
    556, 556, 556, 556, 556, 556, 556, 333, 333, 584, 584, 584, 611, 975, 722, 722, 722, 722, 667,
    611, 778, 722, 278, 556, 722, 611, 833, 722, 778, 667, 778, 722, 667, 611, 722, 667, 944, 667,
    667, 611, 333, 278, 333, 584, 556, 333, 556, 611, 556, 611, 556, 333, 611, 611, 278, 278, 556,
    278, 889, 611, 611, 611, 611, 389, 556, 333, 611, 556, 778, 556, 556, 500, 389, 280, 389, 584,
    0, 556, 0, 278, 556, 500, 1000, 556, 556, 333, 1000, 667, 333, 1000, 0, 611, 0, 0, 278, 278,
    500, 500, 350, 556, 1000, 333, 1000, 556, 333, 944, 0, 500, 667, 278, 333, 556, 556, 556, 556,
    280, 556, 333, 737, 370, 556, 584, 333, 737, 333, 400, 584, 333, 333, 333, 611, 556, 278, 333,
    333, 365, 556, 834, 834, 834, 611, 722, 722, 722, 722, 722, 722, 1000, 722, 667, 667, 667, 667,
    278, 278, 278, 278, 722, 722, 778, 778, 778, 778, 778, 584, 778, 722, 722, 722, 722, 667, 667,
    611, 556, 556, 556, 556, 556, 556, 889, 556, 556, 556, 556, 556, 278, 278, 278, 278, 611, 611,
    611, 611, 611, 611, 611, 584, 611, 611, 611, 611, 611, 556, 611, 556,
];

const PAGE_WIDTH: f32 = 595.28;
const PAGE_HEIGHT: f32 = 841.89;
const MARGIN: f32 = 56.0;
const CONTENT_TOP: f32 = PAGE_HEIGHT - 100.0;
const CONTENT_BOTTOM: f32 = 64.0;
const CONTENT_WIDTH: f32 = PAGE_WIDTH - 2.0 * MARGIN;
const BODY_SIZE: f32 = 10.5;
const BODY_LEADING: f32 = 15.0;
const TABLE_SIZE: f32 = 9.5;
const TABLE_LEADING: f32 = 13.0;
const CELL_PADDING: f32 = 5.0;
const TEXT: f32 = 0.13;
const MUTED: f32 = 0.45;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Font {
    Regular,
    Bold,
}

impl Font {
    fn of(bold: bool) -> Self {
        if bold { Self::Bold } else { Self::Regular }
    }

    fn resource(self) -> &'static str {
        match self {
            Self::Regular => "F1",
            Self::Bold => "F2",
        }
    }

    fn advance(self, byte: u8, size: f32) -> f32 {
        let table = match self {
            Self::Regular => &HELVETICA,
            Self::Bold => &HELVETICA_BOLD,
        };
        let width = byte
            .checked_sub(32)
            .and_then(|index| table.get(usize::from(index)))
            .copied()
            .filter(|width| *width > 0)
            .unwrap_or(556);
        f32::from(width) * size / 1000.0
    }

    fn measure(self, bytes: &[u8], size: f32) -> f32 {
        bytes.iter().map(|byte| self.advance(*byte, size)).sum()
    }
}

fn encode_char(character: char) -> u8 {
    match character {
        ' '..='~' => character as u8,
        '\u{a0}'..='\u{ff}' => character as u32 as u8,
        '€' => 0x80,
        '‚' => 0x82,
        'ƒ' => 0x83,
        '„' => 0x84,
        '…' => 0x85,
        '†' => 0x86,
        '‡' => 0x87,
        'ˆ' => 0x88,
        '‰' => 0x89,
        'Š' => 0x8a,
        '‹' => 0x8b,
        'Œ' => 0x8c,
        'Ž' => 0x8e,
        '‘' => 0x91,
        '’' => 0x92,
        '“' => 0x93,
        '”' => 0x94,
        '•' => 0x95,
        '–' => 0x96,
        '—' => 0x97,
        '˜' => 0x98,
        '™' => 0x99,
        'š' => 0x9a,
        '›' => 0x9b,
        'œ' => 0x9c,
        'ž' => 0x9e,
        'Ÿ' => 0x9f,
        '\t' => b' ',
        _ => b'?',
    }
}

fn encode(text: &str) -> Vec<u8> {
    text.chars()
        .filter(|character| !character.is_control() || *character == '\t')
        .map(encode_char)
        .collect()
}

#[derive(Debug, Clone)]
struct Piece {
    bytes: Vec<u8>,
    font: Font,
}

#[derive(Debug, Clone, Default)]
struct Word {
    pieces: Vec<Piece>,
    width: f32,
    space: f32,
}

impl Word {
    fn push(&mut self, byte: u8, font: Font, size: f32) {
        match self.pieces.last_mut() {
            Some(piece) if piece.font == font => piece.bytes.push(byte),
            _ => self.pieces.push(Piece {
                bytes: vec![byte],
                font,
            }),
        }
        self.width += font.advance(byte, size);
    }

    fn split(self, max_width: f32, size: f32) -> Vec<Word> {
        let mut parts = Vec::new();
        let mut current = Word::default();
        for piece in &self.pieces {
            for byte in &piece.bytes {
                let advance = piece.font.advance(*byte, size);
                if current.width + advance > max_width && !current.pieces.is_empty() {
                    parts.push(std::mem::take(&mut current));
                }
                current.push(*byte, piece.font, size);
            }
        }
        current.space = self.space;
        parts.push(current);
        parts
    }
}

fn words(spans: &[(String, bool)], size: f32) -> Vec<Word> {
    let mut words: Vec<Word> = Vec::new();
    let mut current = Word::default();
    for (text, bold) in spans {
        let font = Font::of(*bold);
        for character in text.chars() {
            if character.is_whitespace() {
                let space = font.advance(b' ', size);
                if !current.pieces.is_empty() {
                    current.space = space;
                    words.push(std::mem::take(&mut current));
                } else if let Some(last) = words.last_mut()
                    && last.space == 0.0
                {
                    last.space = space;
                }
                continue;
            }
            if character.is_control() {
                continue;
            }
            current.push(encode_char(character), font, size);
        }
    }
    if !current.pieces.is_empty() {
        words.push(current);
    }
    words
}

fn wrap(words: Vec<Word>, max_width: f32, size: f32) -> Vec<Vec<Word>> {
    let mut lines: Vec<Vec<Word>> = Vec::new();
    let mut line: Vec<Word> = Vec::new();
    let mut width = 0.0;
    for word in words {
        let parts = if word.width > max_width {
            word.split(max_width, size)
        } else {
            vec![word]
        };
        for part in parts {
            let gap = line.last().map_or(0.0, |previous| previous.space);
            if !line.is_empty() && width + gap + part.width > max_width {
                lines.push(std::mem::take(&mut line));
                width = 0.0;
            } else {
                width += gap;
            }
            width += part.width;
            line.push(part);
        }
    }
    if !line.is_empty() || lines.is_empty() {
        lines.push(line);
    }
    lines
}

fn hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .fold(String::with_capacity(bytes.len() * 2), |mut out, byte| {
            let _ = write!(out, "{byte:02X}");
            out
        })
}

struct Writer {
    header: DocumentHeader,
    pages: Vec<String>,
    content: String,
    y: f32,
    has_body: bool,
    failed: bool,
}

impl Writer {
    fn new(header: DocumentHeader) -> Self {
        let mut writer = Self {
            header,
            pages: Vec::new(),
            content: String::new(),
            y: CONTENT_TOP,
            has_body: false,
            failed: false,
        };
        writer.start_page();
        writer
    }

    fn start_page(&mut self) {
        if !self.content.is_empty() {
            self.pages.push(std::mem::take(&mut self.content));
        }
        if self.pages.len() >= 1000 {
            self.failed = true;
            self.y = CONTENT_TOP;
            self.has_body = false;
            return;
        }
        self.y = CONTENT_TOP;
        self.has_body = false;
        let organization = encode(&self.header.issuer);
        let title = encode(&self.header.title);
        let subtitle = encode(&self.header.subtitle);
        self.text(
            MARGIN,
            PAGE_HEIGHT - 58.0,
            Font::Bold,
            13.0,
            TEXT,
            &organization,
        );
        let right = PAGE_WIDTH - MARGIN;
        self.text(
            right - Font::Bold.measure(&title, 10.0),
            PAGE_HEIGHT - 54.0,
            Font::Bold,
            10.0,
            TEXT,
            &title,
        );
        self.text(
            right - Font::Regular.measure(&subtitle, 8.5),
            PAGE_HEIGHT - 66.0,
            Font::Regular,
            8.5,
            MUTED,
            &subtitle,
        );
        self.line(
            MARGIN,
            PAGE_HEIGHT - 76.0,
            right,
            PAGE_HEIGHT - 76.0,
            0.75,
            0.8,
        );
    }

    fn ensure(&mut self, height: f32) -> bool {
        if self.y - height < CONTENT_BOTTOM && self.has_body {
            self.start_page();
            return true;
        }
        false
    }

    fn text(&mut self, x: f32, y: f32, font: Font, size: f32, gray: f32, bytes: &[u8]) {
        if self.failed {
            return;
        }
        if bytes.is_empty() {
            return;
        }
        let _ = writeln!(
            self.content,
            "BT /{} {size:.2} Tf {gray:.3} g {x:.2} {y:.2} Td <{}> Tj ET",
            font.resource(),
            hex(bytes)
        );
    }

    fn line(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, gray: f32, width: f32) {
        if self.failed {
            return;
        }
        let _ = writeln!(
            self.content,
            "{gray:.3} G {width:.2} w {x1:.2} {y1:.2} m {x2:.2} {y2:.2} l S"
        );
    }

    fn fill(&mut self, x: f32, y: f32, width: f32, height: f32, gray: f32) {
        if self.failed {
            return;
        }
        let _ = writeln!(
            self.content,
            "{gray:.3} g {x:.2} {y:.2} {width:.2} {height:.2} re f"
        );
    }

    fn draw_words(&mut self, x: f32, baseline: f32, line: &[Word], size: f32, gray: f32) {
        let mut cursor = x;
        for word in line {
            for piece in &word.pieces {
                self.text(cursor, baseline, piece.font, size, gray, &piece.bytes);
                cursor += piece.font.measure(&piece.bytes, size);
            }
            cursor += word.space;
        }
    }

    fn flow(&mut self, x: f32, width: f32, spans: &[(String, bool)], size: f32, leading: f32) {
        if self.failed {
            return;
        }
        for line in wrap(words(spans, size), width, size) {
            self.ensure(leading);
            let baseline = self.y - size;
            self.draw_words(x, baseline, &line, size, TEXT);
            self.y -= leading;
            self.has_body = true;
        }
    }

    fn gap(&mut self, height: f32) {
        if self.has_body {
            self.y -= height;
        }
    }

    fn block(&mut self, block: &Block) {
        match block {
            Block::Heading { level, spans } => {
                let size = match level {
                    1 => 18.0,
                    2 => 14.0,
                    _ => 12.0,
                };
                self.gap(size * 0.6);
                self.ensure(size * 1.35 + BODY_LEADING * 2.0);
                let spans: Vec<(String, bool)> =
                    spans.iter().map(|span| (span.text.clone(), true)).collect();
                self.flow(MARGIN, CONTENT_WIDTH, &spans, size, size * 1.35);
                self.y -= 4.0;
            }
            Block::Paragraph { lines } => {
                for line in lines {
                    self.flow(MARGIN, CONTENT_WIDTH, &pairs(line), BODY_SIZE, BODY_LEADING);
                }
                self.y -= 8.0;
            }
            Block::List { items } => {
                for item in items {
                    self.ensure(BODY_LEADING);
                    let bullet = [0x95];
                    let baseline = self.y - BODY_SIZE;
                    self.text(
                        MARGIN + 4.0,
                        baseline,
                        Font::Regular,
                        BODY_SIZE,
                        TEXT,
                        &bullet,
                    );
                    self.flow(
                        MARGIN + 16.0,
                        CONTENT_WIDTH - 16.0,
                        &pairs(item),
                        BODY_SIZE,
                        BODY_LEADING,
                    );
                    self.y -= 2.0;
                }
                self.y -= 6.0;
            }
            Block::Table { table } => self.table(table),
            Block::Field { label, value } => self.flow(
                MARGIN,
                CONTENT_WIDTH,
                &[(format!("{label}: "), true), (value.clone(), false)],
                BODY_SIZE,
                BODY_LEADING,
            ),
            Block::Gap => self.y -= 8.0,
            Block::PageBreak => {
                if self.has_body {
                    self.start_page();
                }
            }
            Block::Rule => {
                self.ensure(16.0);
                self.y -= 6.0;
                let y = self.y;
                self.line(MARGIN, y, PAGE_WIDTH - MARGIN, y, 0.8, 0.6);
                self.y -= 10.0;
                self.has_body = true;
            }
        }
    }

    fn columns(table: &Table) -> Vec<(f32, f32)> {
        let total: f32 = table.columns.iter().map(|column| column.weight).sum();
        let mut start = MARGIN;
        table
            .columns
            .iter()
            .map(|column| {
                let width = CONTENT_WIDTH * (column.weight / total);
                let value = (start, width);
                start += width;
                value
            })
            .collect()
    }
    fn table_header(&mut self, table: &Table) {
        if self.failed {
            return;
        }
        let height = TABLE_LEADING + 2.0 * CELL_PADDING;
        let top = self.y;
        self.fill(MARGIN, top - height, CONTENT_WIDTH, height, 0.93);
        let baseline = top - CELL_PADDING - TABLE_SIZE;
        for ((start, width), column) in Self::columns(table).iter().zip(&table.columns) {
            let bytes = encode(&column.label);
            let x = if column.right_aligned {
                start + width - CELL_PADDING - Font::Bold.measure(&bytes, TABLE_SIZE)
            } else {
                start + CELL_PADDING
            };
            self.text(x, baseline, Font::Bold, TABLE_SIZE, TEXT, &bytes);
        }
        self.y -= height;
        self.has_body = true;
    }
    fn table(&mut self, table: &Table) {
        if self.failed {
            return;
        }
        let columns = Self::columns(table);
        self.y -= 4.0;
        self.ensure(TABLE_LEADING * 4.0 + 4.0 * CELL_PADDING);
        self.table_header(table);
        if table.rows.is_empty()
            && let Some(message) = &table.empty_message
        {
            self.flow(
                MARGIN + CELL_PADDING,
                CONTENT_WIDTH - 2.0 * CELL_PADDING,
                &[(message.clone(), false)],
                TABLE_SIZE,
                TABLE_LEADING,
            );
        }
        for row in &table.rows {
            if self.failed {
                break;
            }
            let cells: Vec<_> = row
                .iter()
                .zip(&columns)
                .map(|(text, (_, width))| {
                    wrap(
                        words(&[(text.clone(), false)], TABLE_SIZE),
                        width - 2.0 * CELL_PADDING,
                        TABLE_SIZE,
                    )
                })
                .collect();
            let line_count = cells.iter().map(Vec::len).max().unwrap_or(1).max(1);
            let height = line_count as f32 * TABLE_LEADING + 2.0 * CELL_PADDING;
            if self.ensure(
                height.min(CONTENT_TOP - CONTENT_BOTTOM - TABLE_LEADING - 2.0 * CELL_PADDING),
            ) {
                self.table_header(table);
            }
            let mut offset = 0;
            while offset < line_count && !self.failed {
                let available = ((self.y - CONTENT_BOTTOM - 2.0 * CELL_PADDING) / TABLE_LEADING)
                    .floor()
                    .max(0.0) as usize;
                if available == 0 {
                    self.start_page();
                    self.table_header(table);
                    continue;
                }
                let count = (line_count - offset).min(available);
                let top = self.y;
                for (index, ((start, width), lines)) in columns.iter().zip(&cells).enumerate() {
                    for (line_index, line) in lines.iter().skip(offset).take(count).enumerate() {
                        let baseline =
                            top - CELL_PADDING - TABLE_SIZE - line_index as f32 * TABLE_LEADING;
                        let measured = line.iter().map(|word| word.width).sum::<f32>()
                            + Font::Regular.advance(b' ', TABLE_SIZE)
                                * line.len().saturating_sub(1) as f32;
                        let x = if table.columns[index].right_aligned {
                            start + width - CELL_PADDING - measured
                        } else {
                            start + CELL_PADDING
                        };
                        self.draw_words(x, baseline, line, TABLE_SIZE, TEXT);
                    }
                }
                self.y -= count as f32 * TABLE_LEADING + 2.0 * CELL_PADDING;
                let y = self.y;
                self.line(MARGIN, y, PAGE_WIDTH - MARGIN, y, 0.85, 0.5);
                offset += count;
                if offset < line_count {
                    self.start_page();
                    self.table_header(table);
                }
            }
        }
        self.y -= 6.0;
        for summary in &table.summary {
            self.ensure(18.0);
            let size = if summary.bold { 11.0 } else { TABLE_SIZE };
            let font = Font::of(summary.bold);
            let baseline = self.y - size;
            let right = PAGE_WIDTH - MARGIN - CELL_PADDING;
            let value = encode(&summary.value);
            let label = encode(&summary.label);
            self.text(
                right - font.measure(&value, size),
                baseline,
                font,
                size,
                TEXT,
                &value,
            );
            self.text(
                right - 120.0 - font.measure(&label, size),
                baseline,
                font,
                size,
                MUTED,
                &label,
            );
            self.y -= size + 6.0;
        }
        self.y -= 8.0;
        self.has_body = true;
    }

    fn finish(mut self) -> Result<Vec<String>, AppError> {
        if self.failed {
            return Err(AppError::Validation(
                "document exceeds the page limit".into(),
            ));
        }
        if !self.content.is_empty() {
            self.pages.push(std::mem::take(&mut self.content));
        }
        let total = self.pages.len();
        let page_label = self.header.page_label.clone();
        let page_of_label = self.header.page_of_label.clone();
        Ok(self
            .pages
            .into_iter()
            .enumerate()
            .map(|(index, mut page)| {
                let label = encode(&format!(
                    "{} {} {} {total}",
                    page_label,
                    index + 1,
                    page_of_label
                ));
                let x = (PAGE_WIDTH - Font::Regular.measure(&label, 8.5)) / 2.0;
                let _ = writeln!(
                    page,
                    "BT /F1 8.50 Tf {MUTED:.3} g {x:.2} 36.00 Td <{}> Tj ET",
                    hex(&label)
                );
                page
            })
            .collect())
    }
}

fn pairs(spans: &[Span]) -> Vec<(String, bool)> {
    spans
        .iter()
        .map(|span| (span.text.clone(), span.bold))
        .collect()
}

fn text_string(text: &str) -> String {
    let mut out = String::from("<FEFF");
    for unit in text.encode_utf16() {
        let _ = write!(out, "{unit:04X}");
    }
    out.push('>');
    out
}

fn assemble(pages: Vec<String>, title: &str) -> Vec<u8> {
    let mut objects: Vec<Vec<u8>> = Vec::new();
    let page_ids: Vec<usize> = (0..pages.len()).map(|index| 6 + index * 2).collect();
    let kids = page_ids
        .iter()
        .map(|id| format!("{id} 0 R"))
        .collect::<Vec<_>>()
        .join(" ");
    objects.push(b"<< /Type /Catalog /Pages 2 0 R >>".to_vec());
    objects.push(format!("<< /Type /Pages /Kids [{kids}] /Count {} >>", pages.len()).into_bytes());
    objects.push(
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>"
            .to_vec(),
    );
    objects.push(
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica-Bold /Encoding /WinAnsiEncoding >>"
            .to_vec(),
    );
    objects.push(
        format!(
            "<< /Title {} /Producer {} /Creator {} >>",
            text_string(title),
            text_string("Aphel"),
            text_string("Aphel")
        )
        .into_bytes(),
    );
    for (index, content) in pages.iter().enumerate() {
        let content_id = 7 + index * 2;
        objects.push(
            format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {PAGE_WIDTH:.2} {PAGE_HEIGHT:.2}] /Resources << /Font << /F1 3 0 R /F2 4 0 R >> >> /Contents {content_id} 0 R >>"
            )
            .into_bytes(),
        );
        let mut stream = format!("<< /Length {} >>\nstream\n", content.len()).into_bytes();
        stream.extend_from_slice(content.as_bytes());
        stream.extend_from_slice(b"\nendstream");
        objects.push(stream);
    }
    let mut output: Vec<u8> = b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n".to_vec();
    let mut offsets = Vec::with_capacity(objects.len());
    for (index, object) in objects.iter().enumerate() {
        offsets.push(output.len());
        output.extend_from_slice(format!("{} 0 obj\n", index + 1).as_bytes());
        output.extend_from_slice(object);
        output.extend_from_slice(b"\nendobj\n");
    }
    let xref = output.len();
    let mut table = format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1);
    for offset in offsets {
        let _ = writeln!(table, "{offset:010} 00000 n ");
    }
    let _ = write!(
        table,
        "trailer\n<< /Size {} /Root 1 0 R /Info 5 0 R >>\nstartxref\n{xref}\n%%EOF\n",
        objects.len() + 1
    );
    output.extend_from_slice(table.as_bytes());
    output
}

pub(crate) fn render(document: &Document) -> Result<Vec<u8>, AppError> {
    validate(document)?;
    let title = format!("{} - {}", document.header.title, document.header.issuer);
    let mut writer = Writer::new(document.header.clone());
    for block in &document.blocks {
        if writer.failed {
            break;
        }
        writer.block(block);
    }
    Ok(assemble(writer.finish()?, &title))
}

fn validate(document: &Document) -> Result<(), AppError> {
    let invalid = || AppError::Validation("invalid or oversized document layout".into());
    let header = &document.header;
    if document.blocks.len() > 10_000
        || [
            &header.issuer,
            &header.title,
            &header.subtitle,
            &header.page_label,
            &header.page_of_label,
        ]
        .iter()
        .any(|value| value.len() > 4096)
    {
        return Err(invalid());
    }
    let mut chars = header.issuer.len()
        + header.title.len()
        + header.subtitle.len()
        + header.page_label.len()
        + header.page_of_label.len();
    let mut nodes = document.blocks.len();
    for block in &document.blocks {
        match block {
            Block::Heading { spans, .. } => {
                nodes += spans.len();
                for span in spans {
                    if span.text.len() > 100_000 {
                        return Err(invalid());
                    }
                    chars += span.text.len();
                }
            }
            Block::Paragraph { lines } | Block::List { items: lines } => {
                nodes += lines.len();
                for spans in lines {
                    nodes += spans.len();
                    for span in spans {
                        if span.text.len() > 100_000 {
                            return Err(invalid());
                        }
                        chars += span.text.len();
                    }
                }
            }
            Block::Field { label, value } => {
                if label.len() > 4096 || value.len() > 100_000 {
                    return Err(invalid());
                }
                chars += label.len() + value.len();
            }
            Block::Table { table } => {
                let weight: f32 = table.columns.iter().map(|column| column.weight).sum();
                if table.columns.is_empty()
                    || table.columns.len() > 12
                    || !weight.is_finite()
                    || table.columns.iter().any(|column| {
                        !column.weight.is_finite()
                            || column.weight <= 0.0
                            || CONTENT_WIDTH * (column.weight / weight) < 24.0
                            || column.label.len() > 128
                    })
                    || table
                        .rows
                        .iter()
                        .any(|row| row.len() != table.columns.len())
                {
                    return Err(invalid());
                }
                if let Some(message) = &table.empty_message {
                    if message.len() > 100_000 {
                        return Err(invalid());
                    }
                    chars += message.len();
                }
                for column in &table.columns {
                    let width = CONTENT_WIDTH * (column.weight / weight);
                    if Font::Bold.measure(&encode(&column.label), TABLE_SIZE)
                        > width - 2.0 * CELL_PADDING
                    {
                        return Err(invalid());
                    }
                }
                nodes += table.columns.len() + table.rows.len() + table.summary.len();
                for column in &table.columns {
                    chars += column.label.len();
                }
                for row in &table.rows {
                    nodes += row.len();
                    for value in row {
                        if value.len() > 16_000 {
                            return Err(invalid());
                        }
                        chars += value.len();
                    }
                }
                for line in &table.summary {
                    if line.label.len() > 4096 || line.value.len() > 4096 {
                        return Err(invalid());
                    }
                    chars += line.label.len() + line.value.len();
                }
            }
            _ => {}
        }
        if chars > 1_000_000 || nodes > 100_000 {
            return Err(invalid());
        }
    }
    Ok(())
}
