use std::str::FromStr;

use rust_decimal::Decimal;

use crate::application::fields::formulas::{
    FormulaError, INVALID_NUMBER, Span, UNEXPECTED_CHARACTER, UNTERMINATED_STRING,
};

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    Number(Decimal),
    Text(String),
    Identifier(String),
    LeftParen,
    RightParen,
    Comma,
    Dot,
    Plus,
    Minus,
    Star,
    Slash,
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    And,
    Or,
    Not,
    True,
    False,
    Null,
    End,
}

impl TokenKind {
    pub fn describe(&self) -> String {
        match self {
            Self::Number(number) => number.to_string(),
            Self::Text(_) => "text".to_owned(),
            Self::Identifier(name) => name.clone(),
            Self::LeftParen => "(".to_owned(),
            Self::RightParen => ")".to_owned(),
            Self::Comma => ",".to_owned(),
            Self::Dot => ".".to_owned(),
            Self::Plus => "+".to_owned(),
            Self::Minus => "-".to_owned(),
            Self::Star => "*".to_owned(),
            Self::Slash => "/".to_owned(),
            Self::Equal => "=".to_owned(),
            Self::NotEqual => "!=".to_owned(),
            Self::Less => "<".to_owned(),
            Self::LessEqual => "<=".to_owned(),
            Self::Greater => ">".to_owned(),
            Self::GreaterEqual => ">=".to_owned(),
            Self::And => "and".to_owned(),
            Self::Or => "or".to_owned(),
            Self::Not => "not".to_owned(),
            Self::True => "true".to_owned(),
            Self::False => "false".to_owned(),
            Self::Null => "null".to_owned(),
            Self::End => "end of expression".to_owned(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

pub fn tokenize(source: &str) -> Result<Vec<Token>, FormulaError> {
    let characters: Vec<char> = source.chars().collect();
    let mut tokens = Vec::new();
    let mut index = 0;
    while index < characters.len() {
        let character = characters[index];
        if character.is_whitespace() {
            index += 1;
            continue;
        }
        let start = index;
        let single = |kind: TokenKind| Token {
            kind,
            span: Span::new(start, start + 1),
        };
        let next = characters.get(index + 1).copied();
        let token = match character {
            '(' => single(TokenKind::LeftParen),
            ')' => single(TokenKind::RightParen),
            ',' => single(TokenKind::Comma),
            '+' => single(TokenKind::Plus),
            '-' => single(TokenKind::Minus),
            '*' => single(TokenKind::Star),
            '/' => single(TokenKind::Slash),
            '.' if !next.is_some_and(|next| next.is_ascii_digit()) => single(TokenKind::Dot),
            '=' => double(start, next == Some('='), TokenKind::Equal, TokenKind::Equal),
            '!' => double(
                start,
                next == Some('='),
                TokenKind::NotEqual,
                TokenKind::Not,
            ),
            '<' if next == Some('>') => double(start, true, TokenKind::NotEqual, TokenKind::Less),
            '<' => double(
                start,
                next == Some('='),
                TokenKind::LessEqual,
                TokenKind::Less,
            ),
            '>' => double(
                start,
                next == Some('='),
                TokenKind::GreaterEqual,
                TokenKind::Greater,
            ),
            '&' if next == Some('&') => double(start, true, TokenKind::And, TokenKind::And),
            '|' if next == Some('|') => double(start, true, TokenKind::Or, TokenKind::Or),
            '"' | '\'' => string_token(&characters, start)?,
            character if character.is_ascii_digit() || character == '.' => {
                number_token(&characters, start)?
            }
            character if character.is_ascii_alphabetic() || character == '_' => {
                word_token(&characters, start)
            }
            other => {
                return Err(FormulaError::new(
                    UNEXPECTED_CHARACTER,
                    format!("unexpected character {other:?}"),
                    Span::new(start, start + 1),
                ));
            }
        };
        index = token.span.end;
        tokens.push(token);
    }
    tokens.push(Token {
        kind: TokenKind::End,
        span: Span::new(characters.len(), characters.len()),
    });
    Ok(tokens)
}

fn double(start: usize, is_double: bool, double: TokenKind, single: TokenKind) -> Token {
    let (kind, length) = if is_double { (double, 2) } else { (single, 1) };
    Token {
        kind,
        span: Span::new(start, start + length),
    }
}

fn string_token(characters: &[char], start: usize) -> Result<Token, FormulaError> {
    let quote = characters[start];
    let mut text = String::new();
    let mut index = start + 1;
    while index < characters.len() {
        match characters[index] {
            '\\' if index + 1 < characters.len() => {
                text.push(characters[index + 1]);
                index += 2;
            }
            character if character == quote => {
                return Ok(Token {
                    kind: TokenKind::Text(text),
                    span: Span::new(start, index + 1),
                });
            }
            character => {
                text.push(character);
                index += 1;
            }
        }
    }
    Err(FormulaError::new(
        UNTERMINATED_STRING,
        "text is not closed",
        Span::new(start, characters.len()),
    ))
}

fn number_token(characters: &[char], start: usize) -> Result<Token, FormulaError> {
    let mut index = start;
    let mut seen_dot = false;
    while index < characters.len() {
        match characters[index] {
            digit if digit.is_ascii_digit() => index += 1,
            '.' if !seen_dot
                && characters
                    .get(index + 1)
                    .is_some_and(|next| next.is_ascii_digit()) =>
            {
                seen_dot = true;
                index += 1;
            }
            _ => break,
        }
    }
    let text: String = characters[start..index].iter().collect();
    let span = Span::new(start, index);
    let number = Decimal::from_str(&text)
        .map_err(|_| FormulaError::new(INVALID_NUMBER, format!("invalid number {text}"), span))?;
    Ok(Token {
        kind: TokenKind::Number(number),
        span,
    })
}

fn word_token(characters: &[char], start: usize) -> Token {
    let mut index = start;
    while index < characters.len()
        && (characters[index].is_ascii_alphanumeric() || characters[index] == '_')
    {
        index += 1;
    }
    let word: String = characters[start..index].iter().collect();
    let kind = match word.to_ascii_lowercase().as_str() {
        "and" => TokenKind::And,
        "or" => TokenKind::Or,
        "not" => TokenKind::Not,
        "true" => TokenKind::True,
        "false" => TokenKind::False,
        "null" => TokenKind::Null,
        _ => TokenKind::Identifier(word),
    };
    Token {
        kind,
        span: Span::new(start, index),
    }
}
