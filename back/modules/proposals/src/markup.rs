use serde::Serialize;
use utoipa::ToSchema;

pub(crate) const ITEMS_TABLE: &str = "items_table";
pub(crate) const PAGE_BREAK: &str = "page_break";
pub(crate) const MAX_BODY_CHARS: usize = 100_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Inline {
    Text(String),
    Variable {
        path: String,
        line: usize,
        column: usize,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Run {
    pub(crate) bold: bool,
    pub(crate) inline: Inline,
}

pub(crate) type Line = Vec<Run>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Block {
    Heading { level: u8, line: Line },
    Paragraph(Vec<Line>),
    List(Vec<Line>),
    ItemsTable,
    PageBreak,
    Rule,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
#[schema(as = ProposalTemplateIssue)]
pub(crate) struct Issue {
    pub(crate) line: usize,
    pub(crate) column: usize,
    pub(crate) code: &'static str,
    pub(crate) variable: Option<String>,
}

impl Issue {
    fn new(line: usize, column: usize, code: &'static str, variable: Option<String>) -> Self {
        Self {
            line,
            column,
            code,
            variable,
        }
    }

    pub(crate) fn describe(&self) -> String {
        match &self.variable {
            Some(variable) => format!(
                "line {}, column {}: {} {{{{{variable}}}}}",
                self.line, self.column, self.code
            ),
            None => format!("line {}, column {}: {}", self.line, self.column, self.code),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct Template {
    pub(crate) blocks: Vec<Block>,
    pub(crate) issues: Vec<Issue>,
}

impl Template {
    pub(crate) fn variables(&self) -> Vec<(&str, usize, usize)> {
        let mut found = Vec::new();
        for block in &self.blocks {
            match block {
                Block::Heading { line, .. } => collect_variables(line, &mut found),
                Block::Paragraph(lines) | Block::List(lines) => {
                    for line in lines {
                        collect_variables(line, &mut found);
                    }
                }
                Block::ItemsTable | Block::PageBreak | Block::Rule => {}
            }
        }
        found
    }
}

fn collect_variables<'a>(line: &'a Line, found: &mut Vec<(&'a str, usize, usize)>) {
    for run in line {
        if let Inline::Variable { path, line, column } = &run.inline {
            found.push((path.as_str(), *line, *column));
        }
    }
}

enum Pending {
    None,
    Paragraph(Vec<Line>),
    List(Vec<Line>),
}

impl Pending {
    fn flush(&mut self, blocks: &mut Vec<Block>) {
        match std::mem::replace(self, Pending::None) {
            Pending::None => {}
            Pending::Paragraph(lines) => blocks.push(Block::Paragraph(lines)),
            Pending::List(items) => blocks.push(Block::List(items)),
        }
    }
}

pub(crate) fn parse(body: &str) -> Template {
    let mut blocks = Vec::new();
    let mut issues = Vec::new();
    let mut pending = Pending::None;
    for (index, raw) in body.lines().enumerate() {
        let number = index + 1;
        let trimmed = raw.trim();
        let indent = raw
            .chars()
            .take_while(|character| character.is_whitespace())
            .count();
        if trimmed.is_empty() {
            pending.flush(&mut blocks);
            continue;
        }
        if let Some(directive) = directive(trimmed)
            && (directive == ITEMS_TABLE || directive == PAGE_BREAK)
        {
            pending.flush(&mut blocks);
            blocks.push(if directive == ITEMS_TABLE {
                Block::ItemsTable
            } else {
                Block::PageBreak
            });
            continue;
        }
        if trimmed == "---" {
            pending.flush(&mut blocks);
            blocks.push(Block::Rule);
            continue;
        }
        if let Some((level, rest, offset)) = heading(trimmed) {
            pending.flush(&mut blocks);
            let line = parse_inline(rest, number, indent + offset + 1, &mut issues);
            blocks.push(Block::Heading { level, line });
            continue;
        }
        if let Some(rest) = trimmed
            .strip_prefix("- ")
            .or_else(|| trimmed.strip_prefix("* "))
        {
            let line = parse_inline(rest.trim_start(), number, indent + 3, &mut issues);
            match &mut pending {
                Pending::List(items) => items.push(line),
                _ => {
                    pending.flush(&mut blocks);
                    pending = Pending::List(vec![line]);
                }
            }
            continue;
        }
        let line = parse_inline(trimmed, number, indent + 1, &mut issues);
        match &mut pending {
            Pending::Paragraph(lines) => lines.push(line),
            _ => {
                pending.flush(&mut blocks);
                pending = Pending::Paragraph(vec![line]);
            }
        }
    }
    pending.flush(&mut blocks);
    Template { blocks, issues }
}

fn heading(line: &str) -> Option<(u8, &str, usize)> {
    let hashes = line
        .chars()
        .take_while(|character| *character == '#')
        .count();
    if !(1..=3).contains(&hashes) {
        return None;
    }
    let rest = line[hashes..].strip_prefix(' ')?;
    Some((hashes as u8, rest.trim_start(), hashes + 1))
}

fn directive(line: &str) -> Option<&str> {
    let inner = line.strip_prefix("{{")?.strip_suffix("}}")?.trim();
    (!inner.contains('{') && !inner.contains('}')).then_some(inner)
}

fn is_valid_path(path: &str) -> bool {
    !path.is_empty()
        && path.split('.').all(|segment| {
            !segment.is_empty()
                && segment.starts_with(|character: char| character.is_ascii_lowercase())
                && segment.chars().all(|character| {
                    character.is_ascii_lowercase() || character.is_ascii_digit() || character == '_'
                })
        })
}

fn push_text(line: &mut Line, text: &mut String, bold: bool) {
    if text.is_empty() {
        return;
    }
    line.push(Run {
        bold,
        inline: Inline::Text(std::mem::take(text)),
    });
}

fn parse_inline(
    text: &str,
    line_number: usize,
    first_column: usize,
    issues: &mut Vec<Issue>,
) -> Line {
    let characters: Vec<char> = text.chars().collect();
    let mut line = Vec::new();
    let mut buffer = String::new();
    let mut bold = false;
    let mut index = 0;
    while index < characters.len() {
        let character = characters[index];
        let next = characters.get(index + 1).copied();
        if character == '\\' && next.is_some_and(|next| matches!(next, '*' | '{' | '}' | '\\')) {
            buffer.push(next.unwrap_or_default());
            index += 2;
            continue;
        }
        if character == '*' && next == Some('*') {
            push_text(&mut line, &mut buffer, bold);
            bold = !bold;
            index += 2;
            continue;
        }
        if character == '{' && next == Some('{') {
            let column = first_column + index;
            let close = (index + 2..characters.len().saturating_sub(1))
                .find(|position| characters[*position] == '}' && characters[position + 1] == '}');
            let Some(close) = close else {
                issues.push(Issue::new(line_number, column, "unclosed_variable", None));
                buffer.extend(&characters[index..]);
                break;
            };
            let path: String = characters[index + 2..close]
                .iter()
                .collect::<String>()
                .trim()
                .to_owned();
            push_text(&mut line, &mut buffer, bold);
            if path.is_empty() {
                issues.push(Issue::new(line_number, column, "empty_variable", None));
            } else if !is_valid_path(&path) {
                issues.push(Issue::new(
                    line_number,
                    column,
                    "invalid_variable",
                    Some(path),
                ));
            } else if path == ITEMS_TABLE || path == PAGE_BREAK {
                issues.push(Issue::new(
                    line_number,
                    column,
                    "block_variable_inline",
                    Some(path),
                ));
            } else {
                line.push(Run {
                    bold,
                    inline: Inline::Variable {
                        path,
                        line: line_number,
                        column,
                    },
                });
            }
            index = close + 2;
            continue;
        }
        buffer.push(character);
        index += 1;
    }
    push_text(&mut line, &mut buffer, bold);
    line
}
