use std::collections::BTreeSet;

use crate::application::{
    BinaryOp, Expr, ExprKind, FormulaContext, FormulaError, RELATED_OUTSIDE_AGGREGATE, Span,
    TYPE_MISMATCH, UNKNOWN_FIELD, UNKNOWN_FUNCTION, UNKNOWN_RELATION, UNSUPPORTED_FIELD, UnaryOp,
    ValueKind, WRONG_ARGUMENTS, is_known_function, parse_formula,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum AggregateFunction {
    Count,
    Sum,
    Avg,
    Min,
    Max,
}

impl AggregateFunction {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Count => "count",
            Self::Sum => "sum",
            Self::Avg => "avg",
            Self::Min => "min",
            Self::Max => "max",
        }
    }

    fn parse(name: &str) -> Option<Self> {
        Some(match name {
            "count" => Self::Count,
            "sum" => Self::Sum,
            "avg" => Self::Avg,
            "min" => Self::Min,
            "max" => Self::Max,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct AggregateRef {
    pub function: AggregateFunction,
    pub relation: String,
    pub field: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Dependencies {
    pub fields: BTreeSet<String>,
    pub aggregates: BTreeSet<AggregateRef>,
    pub uses_today: bool,
}

impl Dependencies {
    pub fn relations(&self) -> BTreeSet<&str> {
        self.aggregates
            .iter()
            .map(|aggregate| aggregate.relation.as_str())
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompiledFormula {
    pub expression: Expr,
    pub kind: Option<ValueKind>,
    pub dependencies: Dependencies,
}

pub fn compile_formula(
    source: &str,
    context: &FormulaContext,
) -> Result<CompiledFormula, Vec<FormulaError>> {
    let expression = parse_formula(source).map_err(|error| vec![error])?;
    let mut analyzer = Analyzer {
        context,
        dependencies: Dependencies::default(),
        errors: Vec::new(),
    };
    let kind = analyzer.check(&expression);
    if !analyzer.errors.is_empty() {
        return Err(analyzer.errors);
    }
    Ok(CompiledFormula {
        expression,
        kind: kind.unwrap_or(None),
        dependencies: analyzer.dependencies,
    })
}

type Checked = Option<Option<ValueKind>>;

struct Analyzer<'a> {
    context: &'a FormulaContext,
    dependencies: Dependencies,
    errors: Vec<FormulaError>,
}

fn kind_name(kind: Option<ValueKind>) -> &'static str {
    kind.map_or("null", |kind| kind.as_str())
}

impl Analyzer<'_> {
    fn fail(&mut self, code: &'static str, message: impl Into<String>, span: Span) -> Checked {
        self.errors.push(FormulaError::new(code, message, span));
        None
    }

    fn mismatch(&mut self, message: impl Into<String>, span: Span) -> Checked {
        self.fail(TYPE_MISMATCH, message, span)
    }

    fn expect(&mut self, expression: &Expr, expected: ValueKind) -> Option<()> {
        let kind = self.check(expression)?;
        match kind {
            None => Some(()),
            Some(kind) if kind == expected => Some(()),
            Some(kind) => {
                self.mismatch(
                    format!("expected {} but found {}", expected.as_str(), kind.as_str()),
                    expression.span,
                );
                None
            }
        }
    }

    fn check(&mut self, expression: &Expr) -> Checked {
        let span = expression.span;
        match &expression.kind {
            ExprKind::Number(_) => Some(Some(ValueKind::Number)),
            ExprKind::Text(_) => Some(Some(ValueKind::Text)),
            ExprKind::Bool(_) => Some(Some(ValueKind::Bool)),
            ExprKind::Null => Some(None),
            ExprKind::Field(key) => self.field(key, span),
            ExprKind::Related { relation, .. } => self.fail(
                RELATED_OUTSIDE_AGGREGATE,
                format!("{relation}.… can only be used inside count, sum, avg, min or max"),
                span,
            ),
            ExprKind::Unary(UnaryOp::Negate, operand) => {
                self.expect(operand, ValueKind::Number)?;
                Some(Some(ValueKind::Number))
            }
            ExprKind::Unary(UnaryOp::Not, operand) => {
                self.expect(operand, ValueKind::Bool)?;
                Some(Some(ValueKind::Bool))
            }
            ExprKind::Binary(operator, left, right) => self.binary(*operator, left, right, span),
            ExprKind::Call { name, args } => self.call(name, args, span),
        }
    }

    fn field(&mut self, key: &str, span: Span) -> Checked {
        let Some(field) = self.context.field(key) else {
            return self.fail(UNKNOWN_FIELD, format!("unknown field {key}"), span);
        };
        let Some(kind) = field.kind else {
            let type_name = field.definition().type_name();
            return self.fail(
                UNSUPPORTED_FIELD,
                format!("{type_name} fields such as {key} cannot be used in formulas"),
                span,
            );
        };
        self.dependencies.fields.insert(key.to_owned());
        Some(Some(kind))
    }

    fn binary(&mut self, operator: BinaryOp, left: &Expr, right: &Expr, span: Span) -> Checked {
        let left_kind = self.check(left);
        let right_kind = self.check(right);
        let (left_kind, right_kind) = (left_kind?, right_kind?);
        use ValueKind::*;
        let invalid = |analyzer: &mut Self| {
            analyzer.mismatch(
                format!(
                    "{} cannot combine {} and {}",
                    operator.symbol(),
                    kind_name(left_kind),
                    kind_name(right_kind)
                ),
                span,
            )
        };
        match operator {
            BinaryOp::And | BinaryOp::Or => match (left_kind, right_kind) {
                (None | Some(Bool), None | Some(Bool)) => Some(Some(Bool)),
                _ => invalid(self),
            },
            BinaryOp::Equal | BinaryOp::NotEqual => match (left_kind, right_kind) {
                (None, _) | (_, None) => Some(Some(Bool)),
                (Some(left), Some(right)) if left == right => Some(Some(Bool)),
                _ => invalid(self),
            },
            operator if operator.is_comparison() => match (left_kind, right_kind) {
                (Some(Bool), _) | (_, Some(Bool)) => invalid(self),
                (None, _) | (_, None) => Some(Some(Bool)),
                (Some(left), Some(right)) if left == right => Some(Some(Bool)),
                _ => invalid(self),
            },
            BinaryOp::Add => match (left_kind, right_kind) {
                (Some(Number) | None, Some(Number) | None) => Some(Some(Number)),
                (Some(Date), Some(Number) | None) | (Some(Number) | None, Some(Date)) => {
                    Some(Some(Date))
                }
                (Some(Text), _) | (_, Some(Text)) => {
                    self.mismatch("use concat(...) to join texts", span)
                }
                _ => invalid(self),
            },
            BinaryOp::Subtract => match (left_kind, right_kind) {
                (Some(Number) | None, Some(Number) | None) => Some(Some(Number)),
                (Some(Date), Some(Date)) => Some(Some(Number)),
                (Some(Date), Some(Number) | None) => Some(Some(Date)),
                _ => invalid(self),
            },
            _ => match (left_kind, right_kind) {
                (Some(Number) | None, Some(Number) | None) => Some(Some(Number)),
                _ => invalid(self),
            },
        }
    }

    fn arity(
        &mut self,
        name: &str,
        args: &[Expr],
        range: (usize, usize),
        span: Span,
    ) -> Option<()> {
        let (min, max) = range;
        if args.len() < min || args.len() > max {
            let expected = if min == max {
                format!("{min}")
            } else if max == usize::MAX {
                format!("at least {min}")
            } else {
                format!("{min} to {max}")
            };
            self.fail(
                WRONG_ARGUMENTS,
                format!(
                    "{name} expects {expected} arguments but received {}",
                    args.len()
                ),
                span,
            );
            return None;
        }
        Some(())
    }

    fn call(&mut self, name: &str, args: &[Expr], span: Span) -> Checked {
        use ValueKind::*;
        if !is_known_function(name) {
            return self.fail(UNKNOWN_FUNCTION, format!("unknown function {name}"), span);
        }
        if let Some(function) = AggregateFunction::parse(name)
            && (!matches!(function, AggregateFunction::Min | AggregateFunction::Max)
                || args.len() == 1)
        {
            return self.aggregate(function, args, span);
        }
        match name {
            "today" => {
                self.arity(name, args, (0, 0), span)?;
                self.dependencies.uses_today = true;
                Some(Some(Date))
            }
            "days_between" => {
                self.arity(name, args, (2, 2), span)?;
                let first = self.expect(&args[0], Date);
                let second = self.expect(&args[1], Date);
                first.and(second)?;
                Some(Some(Number))
            }
            "days_until" | "days_since" => {
                self.arity(name, args, (1, 1), span)?;
                self.expect(&args[0], Date)?;
                self.dependencies.uses_today = true;
                Some(Some(Number))
            }
            "add_days" => {
                self.arity(name, args, (2, 2), span)?;
                let first = self.expect(&args[0], Date);
                let second = self.expect(&args[1], Number);
                first.and(second)?;
                Some(Some(Date))
            }
            "if" => {
                self.arity(name, args, (3, 3), span)?;
                let condition = self.expect(&args[0], Bool);
                let then = self.check(&args[1]);
                let otherwise = self.check(&args[2]);
                condition?;
                self.unify(name, &[then?, otherwise?], span)
            }
            "coalesce" => {
                self.arity(name, args, (1, usize::MAX), span)?;
                let kinds: Vec<Checked> = args.iter().map(|arg| self.check(arg)).collect();
                let kinds: Option<Vec<Option<ValueKind>>> = kinds.into_iter().collect();
                self.unify(name, &kinds?, span)
            }
            "min" | "max" => {
                let kinds: Vec<Checked> = args.iter().map(|arg| self.check(arg)).collect();
                let kinds: Option<Vec<Option<ValueKind>>> = kinds.into_iter().collect();
                let kind = self.unify(name, &kinds?, span)?;
                match kind {
                    None | Some(Number) | Some(Date) => Some(kind),
                    Some(other) => self.mismatch(
                        format!("{name} works with numbers or dates, not {}", other.as_str()),
                        span,
                    ),
                }
            }
            "is_empty" => {
                self.arity(name, args, (1, 1), span)?;
                self.check(&args[0])?;
                Some(Some(Bool))
            }
            "concat" => {
                self.arity(name, args, (1, usize::MAX), span)?;
                let checked: Vec<Checked> = args.iter().map(|arg| self.check(arg)).collect();
                checked.into_iter().collect::<Option<Vec<_>>>()?;
                Some(Some(Text))
            }
            "upper" | "lower" => {
                self.arity(name, args, (1, 1), span)?;
                self.expect(&args[0], Text)?;
                Some(Some(Text))
            }
            "len" => {
                self.arity(name, args, (1, 1), span)?;
                self.expect(&args[0], Text)?;
                Some(Some(Number))
            }
            "round" => {
                self.arity(name, args, (1, 2), span)?;
                let checked: Vec<Option<()>> =
                    args.iter().map(|arg| self.expect(arg, Number)).collect();
                checked.into_iter().collect::<Option<Vec<_>>>()?;
                Some(Some(Number))
            }
            _ => {
                self.arity(name, args, (1, 1), span)?;
                self.expect(&args[0], Number)?;
                Some(Some(Number))
            }
        }
    }

    fn unify(&mut self, name: &str, kinds: &[Option<ValueKind>], span: Span) -> Checked {
        let mut result: Option<ValueKind> = None;
        for kind in kinds.iter().flatten() {
            match result {
                None => result = Some(*kind),
                Some(current) if current == *kind => {}
                Some(current) => {
                    return self.mismatch(
                        format!("{name} mixes {} and {}", current.as_str(), kind.as_str()),
                        span,
                    );
                }
            }
        }
        Some(result)
    }

    fn aggregate(&mut self, function: AggregateFunction, args: &[Expr], span: Span) -> Checked {
        let name = function.as_str();
        self.arity(name, args, (1, 1), span)?;
        let argument = &args[0];
        let (relation_key, field_key) = match &argument.kind {
            ExprKind::Field(key) if function == AggregateFunction::Count => (key.as_str(), None),
            ExprKind::Related { relation, field } => (relation.as_str(), Some(field.as_str())),
            _ => {
                let expected = if function == AggregateFunction::Count {
                    "a relation such as count(items)"
                } else {
                    "a related field such as sum(items.total)"
                };
                return self.fail(
                    WRONG_ARGUMENTS,
                    format!("{name} expects {expected}"),
                    argument.span,
                );
            }
        };
        let Some(relation) = self.context.relation(relation_key) else {
            return self.fail(
                UNKNOWN_RELATION,
                format!("unknown relation {relation_key}"),
                argument.span,
            );
        };
        if let Some(field_key) = field_key
            && relation.field(field_key).is_none()
        {
            return self.fail(
                UNKNOWN_FIELD,
                format!("{relation_key} has no numeric field {field_key}"),
                argument.span,
            );
        }
        if field_key.is_none() && function != AggregateFunction::Count {
            return self.fail(
                WRONG_ARGUMENTS,
                format!("{name} expects a related field"),
                argument.span,
            );
        }
        self.dependencies.aggregates.insert(AggregateRef {
            function,
            relation: relation_key.to_owned(),
            field: if function == AggregateFunction::Count {
                None
            } else {
                field_key.map(str::to_owned)
            },
        });
        Some(Some(ValueKind::Number))
    }
}
