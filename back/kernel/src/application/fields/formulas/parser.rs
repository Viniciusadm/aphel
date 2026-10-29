use crate::application::fields::formulas::{
    BinaryOp, EMPTY_EXPRESSION, EXPRESSION_TOO_COMPLEX, EXPRESSION_TOO_LONG, Expr, ExprKind,
    FormulaError, Span, Token, TokenKind, UNEXPECTED_END, UNEXPECTED_TOKEN, UnaryOp, tokenize,
};
use crate::domain::fields::MAX_FORMULA_LENGTH;

pub const MAX_DEPTH: usize = 32;
pub const MAX_NODES: usize = 400;
pub const MAX_ARGUMENTS: usize = 20;

const OR_POWER: u8 = 1;
const AND_POWER: u8 = 2;
const NOT_POWER: u8 = 3;
const COMPARISON_POWER: u8 = 4;
const ADDITIVE_POWER: u8 = 5;
const MULTIPLICATIVE_POWER: u8 = 6;
const NEGATE_POWER: u8 = 7;

pub fn parse_formula(source: &str) -> Result<Expr, FormulaError> {
    let length = source.chars().count();
    if length > MAX_FORMULA_LENGTH {
        return Err(FormulaError::new(
            EXPRESSION_TOO_LONG,
            format!("the expression must have at most {MAX_FORMULA_LENGTH} characters"),
            Span::new(MAX_FORMULA_LENGTH, length),
        ));
    }
    if source.trim().is_empty() {
        return Err(FormulaError::new(
            EMPTY_EXPRESSION,
            "the expression is empty",
            Span::default(),
        ));
    }
    let tokens = tokenize(source)?;
    let mut parser = Parser {
        tokens,
        position: 0,
        nodes: 0,
    };
    let expression = parser.expression(0, 0)?;
    let token = parser.peek();
    if token.kind != TokenKind::End {
        return Err(unexpected(token));
    }
    Ok(expression)
}

struct Parser {
    tokens: Vec<Token>,
    position: usize,
    nodes: usize,
}

fn unexpected(token: &Token) -> FormulaError {
    if token.kind == TokenKind::End {
        return FormulaError::new(
            UNEXPECTED_END,
            "the expression ended unexpectedly",
            token.span,
        );
    }
    FormulaError::new(
        UNEXPECTED_TOKEN,
        format!("unexpected {}", token.kind.describe()),
        token.span,
    )
}

fn infix(kind: &TokenKind) -> Option<(BinaryOp, u8)> {
    Some(match kind {
        TokenKind::Or => (BinaryOp::Or, OR_POWER),
        TokenKind::And => (BinaryOp::And, AND_POWER),
        TokenKind::Equal => (BinaryOp::Equal, COMPARISON_POWER),
        TokenKind::NotEqual => (BinaryOp::NotEqual, COMPARISON_POWER),
        TokenKind::Less => (BinaryOp::Less, COMPARISON_POWER),
        TokenKind::LessEqual => (BinaryOp::LessEqual, COMPARISON_POWER),
        TokenKind::Greater => (BinaryOp::Greater, COMPARISON_POWER),
        TokenKind::GreaterEqual => (BinaryOp::GreaterEqual, COMPARISON_POWER),
        TokenKind::Plus => (BinaryOp::Add, ADDITIVE_POWER),
        TokenKind::Minus => (BinaryOp::Subtract, ADDITIVE_POWER),
        TokenKind::Star => (BinaryOp::Multiply, MULTIPLICATIVE_POWER),
        TokenKind::Slash => (BinaryOp::Divide, MULTIPLICATIVE_POWER),
        _ => return None,
    })
}

impl Parser {
    fn peek(&self) -> &Token {
        &self.tokens[self.position.min(self.tokens.len() - 1)]
    }

    fn advance(&mut self) -> Token {
        let token = self.peek().clone();
        if self.position < self.tokens.len() - 1 {
            self.position += 1;
        }
        token
    }

    fn expect(&mut self, kind: TokenKind) -> Result<Token, FormulaError> {
        let token = self.peek();
        if token.kind == kind {
            return Ok(self.advance());
        }
        Err(unexpected(token))
    }

    fn node(&mut self, kind: ExprKind, span: Span) -> Result<Expr, FormulaError> {
        self.nodes += 1;
        if self.nodes > MAX_NODES {
            return Err(FormulaError::new(
                EXPRESSION_TOO_COMPLEX,
                format!("the expression must have at most {MAX_NODES} elements"),
                span,
            ));
        }
        Ok(Expr::new(kind, span))
    }

    fn expression(&mut self, min_power: u8, depth: usize) -> Result<Expr, FormulaError> {
        if depth > MAX_DEPTH {
            return Err(FormulaError::new(
                EXPRESSION_TOO_COMPLEX,
                format!("the expression must nest at most {MAX_DEPTH} levels"),
                self.peek().span,
            ));
        }
        let mut left = self.prefix(depth)?;
        while let Some((operator, power)) = infix(&self.peek().kind) {
            if power <= min_power {
                break;
            }
            self.advance();
            let right = self.expression(power, depth + 1)?;
            let span = left.span.join(right.span);
            left = self.node(
                ExprKind::Binary(operator, Box::new(left), Box::new(right)),
                span,
            )?;
        }
        Ok(left)
    }

    fn prefix(&mut self, depth: usize) -> Result<Expr, FormulaError> {
        let token = self.advance();
        let span = token.span;
        match token.kind {
            TokenKind::Number(number) => self.node(ExprKind::Number(number), span),
            TokenKind::Text(text) => self.node(ExprKind::Text(text), span),
            TokenKind::True => self.node(ExprKind::Bool(true), span),
            TokenKind::False => self.node(ExprKind::Bool(false), span),
            TokenKind::Null => self.node(ExprKind::Null, span),
            TokenKind::Minus => {
                let operand = self.expression(NEGATE_POWER, depth + 1)?;
                let span = span.join(operand.span);
                self.node(ExprKind::Unary(UnaryOp::Negate, Box::new(operand)), span)
            }
            TokenKind::Not => {
                let operand = self.expression(NOT_POWER, depth + 1)?;
                let span = span.join(operand.span);
                self.node(ExprKind::Unary(UnaryOp::Not, Box::new(operand)), span)
            }
            TokenKind::LeftParen => {
                let inner = self.expression(0, depth + 1)?;
                let close = self.expect(TokenKind::RightParen)?;
                Ok(Expr::new(inner.kind, span.join(close.span)))
            }
            TokenKind::Identifier(name) => self.identifier(name, span, depth),
            _ => Err(unexpected(&token)),
        }
    }

    fn identifier(&mut self, name: String, span: Span, depth: usize) -> Result<Expr, FormulaError> {
        match self.peek().kind {
            TokenKind::LeftParen => {
                self.advance();
                let mut args = Vec::new();
                if self.peek().kind != TokenKind::RightParen {
                    loop {
                        if args.len() >= MAX_ARGUMENTS {
                            return Err(FormulaError::new(
                                EXPRESSION_TOO_COMPLEX,
                                format!("a function accepts at most {MAX_ARGUMENTS} arguments"),
                                self.peek().span,
                            ));
                        }
                        args.push(self.expression(0, depth + 1)?);
                        if self.peek().kind == TokenKind::Comma {
                            self.advance();
                            continue;
                        }
                        break;
                    }
                }
                let close = self.expect(TokenKind::RightParen)?;
                self.node(
                    ExprKind::Call {
                        name: name.to_ascii_lowercase(),
                        args,
                    },
                    span.join(close.span),
                )
            }
            TokenKind::Dot => {
                self.advance();
                let token = self.advance();
                let TokenKind::Identifier(field) = token.kind else {
                    return Err(unexpected(&token));
                };
                self.node(
                    ExprKind::Related {
                        relation: name,
                        field,
                    },
                    span.join(token.span),
                )
            }
            _ => self.node(ExprKind::Field(name), span),
        }
    }
}
