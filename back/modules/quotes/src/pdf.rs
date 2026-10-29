use std::fmt::Write as _;

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

pub(crate) struct Header {
    pub(crate) organization: String,
    pub(crate) title: String,
    pub(crate) subtitle: String,
}

struct Writer {
    header: Header,
    pages: Vec<String>,
    content: String,
    y: f32,
    has_body: bool,
}

impl Writer {
    fn new(header: Header) -> Self {
        let mut writer = Self {
            header,
            pages: Vec::new(),
            content: String::new(),
            y: CONTENT_TOP,
            has_body: false,
        };
        writer.start_page();
        writer
    }

    fn start_page(&mut self) {
        if !self.content.is_empty() {
            self.pages.push(std::mem::take(&mut self.content));
        }
        self.y = CONTENT_TOP;
        self.has_body = false;
        let organization = encode(&self.header.organization);
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
        let _ = writeln!(
            self.content,
            "{gray:.3} G {width:.2} w {x1:.2} {y1:.2} m {x2:.2} {y2:.2} l S"
        );
    }

    fn fill(&mut self, x: f32, y: f32, width: f32, height: f32, gray: f32) {
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

    fn heading(&mut self, text: &str, size: f32) {
        self.gap(size * 0.6);
        self.ensure(size * 1.35 + BODY_LEADING * 2.0);
        self.flow(
            MARGIN,
            CONTENT_WIDTH,
            &[(text.to_owned(), true)],
            size,
            size * 1.35,
        );
        self.y -= 4.0;
    }

    fn paragraph(&mut self, text: &str) {
        for line in text.lines() {
            self.flow(
                MARGIN,
                CONTENT_WIDTH,
                &[(line.to_owned(), false)],
                BODY_SIZE,
                BODY_LEADING,
            );
        }
        self.y -= 8.0;
    }

    fn field(&mut self, label: &str, value: &str) {
        self.flow(
            MARGIN,
            CONTENT_WIDTH,
            &[(format!("{label}: "), true), (value.to_owned(), false)],
            BODY_SIZE,
            BODY_LEADING,
        );
    }

    fn columns() -> [(f32, f32); 5] {
        let fixed = [44.0, 82.0, 68.0, 86.0];
        let name = CONTENT_WIDTH - fixed.iter().sum::<f32>();
        let mut start = MARGIN;
        let mut columns = [(0.0, 0.0); 5];
        for (index, width) in std::iter::once(name).chain(fixed).enumerate() {
            columns[index] = (start, width);
            start += width;
        }
        columns
    }

    fn table_header(&mut self) {
        let height = TABLE_LEADING + 2.0 * CELL_PADDING;
        let top = self.y;
        self.fill(MARGIN, top - height, CONTENT_WIDTH, height, 0.93);
        let labels = ["Item", "Qtd.", "Preço unit.", "Desconto", "Total"];
        let baseline = top - CELL_PADDING - TABLE_SIZE;
        for (index, ((start, width), label)) in Self::columns().iter().zip(labels).enumerate() {
            let bytes = encode(label);
            let x = if index == 0 {
                start + CELL_PADDING
            } else {
                start + width - CELL_PADDING - Font::Bold.measure(&bytes, TABLE_SIZE)
            };
            self.text(x, baseline, Font::Bold, TABLE_SIZE, TEXT, &bytes);
        }
        self.y -= height;
        self.has_body = true;
    }

    fn table(&mut self, table: &ItemsTable) {
        let columns = Self::columns();
        self.y -= 4.0;
        self.ensure(TABLE_LEADING * 4.0 + 4.0 * CELL_PADDING);
        self.table_header();
        if table.rows.is_empty() {
            let message = "Nenhum item nesta cotação.";
            let height = TABLE_LEADING + 2.0 * CELL_PADDING;
            let baseline = self.y - CELL_PADDING - TABLE_SIZE;
            self.text(
                MARGIN + CELL_PADDING,
                baseline,
                Font::Regular,
                TABLE_SIZE,
                MUTED,
                &encode(message),
            );
            self.y -= height;
            let y = self.y;
            self.line(MARGIN, y, PAGE_WIDTH - MARGIN, y, 0.85, 0.5);
        }
        for row in &table.rows {
            let name_width = columns[0].1 - 2.0 * CELL_PADDING;
            let lines = wrap(
                words(&[(row.name.clone(), false)], TABLE_SIZE),
                name_width,
                TABLE_SIZE,
            );
            let height = lines.len() as f32 * TABLE_LEADING + 2.0 * CELL_PADDING;
            if self.ensure(height) {
                self.table_header();
            }
            let top = self.y;
            for (index, line) in lines.iter().enumerate() {
                let baseline = top - CELL_PADDING - TABLE_SIZE - index as f32 * TABLE_LEADING;
                self.draw_words(
                    columns[0].0 + CELL_PADDING,
                    baseline,
                    line,
                    TABLE_SIZE,
                    TEXT,
                );
            }
            let baseline = top - CELL_PADDING - TABLE_SIZE;
            let cells = [&row.quantity, &row.unit_price, &row.discount, &row.total];
            for ((start, width), value) in columns[1..].iter().zip(cells) {
                let bytes = encode(value);
                let x = start + width - CELL_PADDING - Font::Regular.measure(&bytes, TABLE_SIZE);
                self.text(x, baseline, Font::Regular, TABLE_SIZE, TEXT, &bytes);
            }
            self.y -= height;
            let y = self.y;
            self.line(MARGIN, y, PAGE_WIDTH - MARGIN, y, 0.85, 0.5);
        }
        self.y -= 6.0;
        let totals = [
            ("Subtotal", &table.subtotal, false),
            ("Descontos", &table.discount_total, false),
            ("Impostos", &table.tax_total, false),
            ("Total", &table.total, true),
        ];
        self.ensure(4.0 * 16.0);
        let right = PAGE_WIDTH - MARGIN - CELL_PADDING;
        for (label, value, strong) in totals {
            let size = if strong { 11.0 } else { TABLE_SIZE };
            let font = Font::of(strong);
            let baseline = self.y - size;
            let value = encode(value);
            let label = encode(label);
            let value_x = right - font.measure(&value, size);
            self.text(value_x, baseline, font, size, TEXT, &value);
            let label_x = right - 120.0 - font.measure(&label, size);
            self.text(label_x, baseline, font, size, MUTED, &label);
            self.y -= size + 6.0;
        }
        self.y -= 8.0;
        self.has_body = true;
    }

    fn finish(mut self) -> Vec<String> {
        if !self.content.is_empty() {
            self.pages.push(std::mem::take(&mut self.content));
        }
        let total = self.pages.len();
        self.pages
            .into_iter()
            .enumerate()
            .map(|(index, mut page)| {
                let label = encode(&format!("Página {} de {total}", index + 1));
                let x = (PAGE_WIDTH - Font::Regular.measure(&label, 8.5)) / 2.0;
                let _ = writeln!(
                    page,
                    "BT /F1 8.50 Tf {MUTED:.3} g {x:.2} 36.00 Td <{}> Tj ET",
                    hex(&label)
                );
                page
            })
            .collect()
    }
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
            text_string("Aphel CRM"),
            text_string("Aphel CRM")
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

pub(crate) fn document(header: Header, blocks: &[Block]) -> Vec<u8> {
    let title = format!("{} - {}", header.title, header.organization);
    let mut writer = Writer::new(header);
    for block in blocks {
        match block {
            Block::Heading(text) => writer.heading(text, 12.0),
            Block::Field { label, value } => writer.field(label, value),
            Block::Gap => writer.y -= 8.0,
            Block::Paragraph(text) => writer.paragraph(text),
            Block::Items(table) => writer.table(table),
        }
    }
    assemble(writer.finish(), &title)
}
