use std::collections::BTreeSet;

use chrono::{DateTime, Duration, SecondsFormat, Utc};
use chrono_tz::Tz;
use serde_json::{Map, Value};

use crate::application::context::lookup;
use aphel_kernel::application::shared::REDACTED;

pub const RECORD_ROOT: &str = "record";
pub const TRIGGER_ROOT: &str = "trigger";
pub const AUTOMATION_ROOT: &str = "automation";
pub const RUN_ROOT: &str = "run";
pub const NOW_ROOT: &str = "now";
pub const TODAY_ROOT: &str = "today";
pub const CUSTOM_ALIAS: &str = "custom";
pub const CUSTOM_FIELDS_KEY: &str = "custom_fields";
pub const MAX_TEMPLATE_LENGTH: usize = 10_000;
pub const MAX_OFFSET_DAYS: i64 = 3650;

pub const TEMPLATE_ROOTS: [&str; 6] = [
    RECORD_ROOT,
    TRIGGER_ROOT,
    AUTOMATION_ROOT,
    RUN_ROOT,
    NOW_ROOT,
    TODAY_ROOT,
];

pub const TRIGGER_PATHS: [&str; 8] = [
    "type",
    "event_id",
    "event_type",
    "occurred_at",
    "actor",
    "changed_fields",
    "previous",
    "payload",
];

pub const RECORD_EXTRA_PATHS: [&str; 7] = [
    "id",
    "title",
    "name",
    CUSTOM_ALIAS,
    CUSTOM_FIELDS_KEY,
    "created_at",
    "updated_at",
];

#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    Path(Vec<String>),
    Now(Duration),
    Today(i64),
}

#[derive(Debug, Clone, PartialEq)]
enum Segment {
    Text(String),
    Expression(Expression),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Template {
    segments: Vec<Segment>,
}

impl Template {
    pub fn parse(source: &str) -> Result<Self, String> {
        if source.len() > MAX_TEMPLATE_LENGTH {
            return Err(format!(
                "templates can have at most {MAX_TEMPLATE_LENGTH} characters"
            ));
        }
        let mut segments = Vec::new();
        let mut rest = source;
        while let Some(start) = rest.find("{{") {
            if start > 0 {
                segments.push(Segment::Text(rest[..start].to_owned()));
            }
            let after = &rest[start + 2..];
            let Some(end) = after.find("}}") else {
                return Err("has a '{{' without the closing '}}'".to_owned());
            };
            segments.push(Segment::Expression(parse_expression(&after[..end])?));
            rest = &after[end + 2..];
        }
        if rest.contains("}}") {
            return Err("has a '}}' without the opening '{{'".to_owned());
        }
        if !rest.is_empty() {
            segments.push(Segment::Text(rest.to_owned()));
        }
        Ok(Self { segments })
    }

    pub fn is_plain(&self) -> bool {
        self.segments
            .iter()
            .all(|segment| matches!(segment, Segment::Text(_)))
    }

    pub fn expressions(&self) -> impl Iterator<Item = &Expression> {
        self.segments.iter().filter_map(|segment| match segment {
            Segment::Expression(expression) => Some(expression),
            Segment::Text(_) => None,
        })
    }

    pub fn render(&self, scope: &TemplateScope) -> Rendered {
        let mut sensitive = false;
        if let [Segment::Expression(expression)] = self.segments.as_slice() {
            let (value, is_sensitive) = scope.evaluate(expression);
            return Rendered {
                value,
                sensitive: is_sensitive,
            };
        }
        let mut text = String::new();
        for segment in &self.segments {
            match segment {
                Segment::Text(part) => text.push_str(part),
                Segment::Expression(expression) => {
                    let (value, is_sensitive) = scope.evaluate(expression);
                    sensitive |= is_sensitive;
                    text.push_str(&display(&value));
                }
            }
        }
        Rendered {
            value: Value::String(text),
            sensitive,
        }
    }
}

fn parse_expression(source: &str) -> Result<Expression, String> {
    let source = source.trim();
    if source.is_empty() {
        return Err("has an empty '{{ }}'".to_owned());
    }
    let (head, offset) = match source.find(['+', '-']) {
        Some(index) => (source[..index].trim(), Some(source[index..].trim())),
        None => (source, None),
    };
    match head {
        NOW_ROOT => {
            let offset = offset.map(parse_offset).transpose()?;
            Ok(Expression::Now(offset.unwrap_or_else(Duration::zero)))
        }
        TODAY_ROOT => {
            let offset = offset.map(parse_offset).transpose()?;
            let duration = offset.unwrap_or_else(Duration::zero);
            if duration.num_seconds() % 86_400 != 0 {
                return Err("today accepts only offsets in days or weeks".to_owned());
            }
            Ok(Expression::Today(duration.num_days()))
        }
        _ if offset.is_some() => Err(format!(
            "{source} is not valid; only now and today accept offsets"
        )),
        path => {
            let segments: Vec<String> = path.split('.').map(str::to_owned).collect();
            let valid = segments.iter().all(|segment| {
                !segment.is_empty()
                    && segment
                        .chars()
                        .all(|character| character.is_ascii_alphanumeric() || character == '_')
            });
            if !valid {
                return Err(format!("{path} is not a valid path"));
            }
            Ok(Expression::Path(segments))
        }
    }
}

pub fn parse_offset(source: &str) -> Result<Duration, String> {
    let (sign, rest) = match source.chars().next() {
        Some('+') => (1, &source[1..]),
        Some('-') => (-1, &source[1..]),
        _ => (1, source),
    };
    let rest = rest.trim();
    let Some(unit) = rest.chars().last() else {
        return Err("has an empty offset".to_owned());
    };
    let amount: i64 = rest[..rest.len() - unit.len_utf8()]
        .trim()
        .parse()
        .map_err(|_| format!("{source} is not a valid offset (examples: 30m, 2h, 2d, 1w)"))?;
    let duration = match unit {
        'm' => Duration::try_minutes(amount),
        'h' => Duration::try_hours(amount),
        'd' => Duration::try_days(amount),
        'w' => Duration::try_weeks(amount),
        _ => None,
    }
    .ok_or_else(|| format!("{source} is not a valid offset (examples: 30m, 2h, 2d, 1w)"))?;
    if duration.num_days().abs() > MAX_OFFSET_DAYS {
        return Err(format!("offsets can be at most {MAX_OFFSET_DAYS} days"));
    }
    Ok(duration * sign)
}

fn display(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::String(text) => text.clone(),
        Value::Bool(flag) => flag.to_string(),
        Value::Number(number) => number.to_string(),
        other => other.to_string(),
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Rendered {
    pub value: Value,
    pub sensitive: bool,
}

#[derive(Debug, Clone)]
pub struct TemplateScope {
    pub record: Value,
    pub trigger: Value,
    pub automation: Value,
    pub run: Value,
    pub now: DateTime<Utc>,
    pub timezone: Tz,
    pub sensitive_fields: BTreeSet<String>,
}

impl TemplateScope {
    fn evaluate(&self, expression: &Expression) -> (Value, bool) {
        match expression {
            Expression::Now(offset) => (
                Value::String((self.now + *offset).to_rfc3339_opts(SecondsFormat::Secs, true)),
                false,
            ),
            Expression::Today(days) => {
                let today = self.now.with_timezone(&self.timezone).date_naive();
                let day = today + Duration::days(*days);
                (Value::String(day.format("%Y-%m-%d").to_string()), false)
            }
            Expression::Path(segments) => self.path(segments),
        }
    }

    fn path(&self, segments: &[String]) -> (Value, bool) {
        let Some((root, rest)) = segments.split_first() else {
            return (Value::Null, false);
        };
        let rest: Vec<&str> = rest.iter().map(String::as_str).collect();
        match root.as_str() {
            RECORD_ROOT => {
                let sensitive = self.is_sensitive_record_path(&rest);
                (record_value(&self.record, &rest), sensitive)
            }
            TRIGGER_ROOT => {
                let sensitive = matches!(rest.as_slice(), ["previous", key, ..] if self.sensitive_fields.contains(*key))
                    || matches!(rest.first(), Some(&"payload"));
                (
                    lookup(&self.trigger, &rest).cloned().unwrap_or(Value::Null),
                    sensitive,
                )
            }
            AUTOMATION_ROOT => (
                lookup(&self.automation, &rest)
                    .cloned()
                    .unwrap_or(Value::Null),
                false,
            ),
            RUN_ROOT => (
                lookup(&self.run, &rest).cloned().unwrap_or(Value::Null),
                false,
            ),
            _ => (Value::Null, false),
        }
    }

    fn is_sensitive_record_path(&self, rest: &[&str]) -> bool {
        match rest {
            [CUSTOM_ALIAS | CUSTOM_FIELDS_KEY, key, ..] => self.sensitive_fields.contains(*key),
            [CUSTOM_ALIAS | CUSTOM_FIELDS_KEY] => !self.sensitive_fields.is_empty(),
            [key, ..] => self.sensitive_fields.contains(*key),
            [] => !self.sensitive_fields.is_empty(),
        }
    }

    pub fn render(&self, value: &Value) -> Result<RenderedConfig, String> {
        let mut sensitive = false;
        let (value, masked) = self.render_node(value, &mut sensitive)?;
        Ok(RenderedConfig {
            value,
            masked,
            sensitive,
        })
    }

    fn render_node(&self, value: &Value, sensitive: &mut bool) -> Result<(Value, Value), String> {
        Ok(match value {
            Value::String(text) => {
                let template = Template::parse(text)?;
                if template.is_plain() {
                    (value.clone(), value.clone())
                } else {
                    let rendered = template.render(self);
                    *sensitive |= rendered.sensitive;
                    let masked = if rendered.sensitive {
                        Value::String(REDACTED.to_owned())
                    } else {
                        rendered.value.clone()
                    };
                    (rendered.value, masked)
                }
            }
            Value::Array(items) => {
                let mut values = Vec::with_capacity(items.len());
                let mut masks = Vec::with_capacity(items.len());
                for item in items {
                    let (value, masked) = self.render_node(item, sensitive)?;
                    values.push(value);
                    masks.push(masked);
                }
                (Value::Array(values), Value::Array(masks))
            }
            Value::Object(map) => {
                let mut values = Map::new();
                let mut masks = Map::new();
                for (key, item) in map {
                    let (value, masked) = self.render_node(item, sensitive)?;
                    values.insert(key.clone(), value);
                    masks.insert(key.clone(), masked);
                }
                (Value::Object(values), Value::Object(masks))
            }
            other => (other.clone(), other.clone()),
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RenderedConfig {
    pub value: Value,
    pub masked: Value,
    pub sensitive: bool,
}

pub fn record_value(record: &Value, rest: &[&str]) -> Value {
    match rest {
        [CUSTOM_ALIAS, tail @ ..] => {
            let mut path = vec![CUSTOM_FIELDS_KEY];
            path.extend_from_slice(tail);
            lookup(record, &path).cloned().unwrap_or(Value::Null)
        }
        ["title"] if record.get("title").is_none() => {
            record.get("name").cloned().unwrap_or(Value::Null)
        }
        _ => lookup(record, rest).cloned().unwrap_or(Value::Null),
    }
}

#[derive(Debug, Clone, Default)]
pub struct TemplateRules {
    pub record_fields: BTreeSet<String>,
    pub custom_fields: BTreeSet<String>,
}

impl TemplateRules {
    pub fn check_expression(&self, expression: &Expression) -> Result<(), String> {
        let Expression::Path(segments) = expression else {
            return Ok(());
        };
        let parts: Vec<&str> = segments.iter().map(String::as_str).collect();
        match parts.as_slice() {
            [RECORD_ROOT] | [TRIGGER_ROOT] => Ok(()),
            [RECORD_ROOT, CUSTOM_ALIAS | CUSTOM_FIELDS_KEY, key, ..] => {
                if self.custom_fields.contains(*key) {
                    Ok(())
                } else {
                    Err(format!(
                        "record.custom.{key} is not a custom field of this record type"
                    ))
                }
            }
            [RECORD_ROOT, key, ..] => {
                if self.record_fields.contains(*key)
                    || RECORD_EXTRA_PATHS.contains(key)
                    || self.record_fields.contains(&format!("{key}_id"))
                {
                    Ok(())
                } else {
                    Err(format!("record.{key} is not a field of this record type"))
                }
            }
            [TRIGGER_ROOT, key, ..] => {
                if TRIGGER_PATHS.contains(key) {
                    Ok(())
                } else {
                    Err(format!(
                        "trigger.{key} is not available (use {})",
                        TRIGGER_PATHS.join(", ")
                    ))
                }
            }
            [AUTOMATION_ROOT, "id" | "name" | "key"] | [RUN_ROOT, "id" | "depth"] => Ok(()),
            [root, ..] => Err(format!(
                "{} is not available (roots: {})",
                segments.join("."),
                if TEMPLATE_ROOTS.contains(root) {
                    "record, trigger, automation.id|name|key, run.id|depth, now, today"
                } else {
                    "record, trigger, automation, run, now, today"
                }
            )),
            [] => Err("has an empty path".to_owned()),
        }
    }

    pub fn violations(&self, value: &Value, path: &str) -> Vec<(String, String)> {
        let mut violations = Vec::new();
        self.visit(value, path, &mut violations);
        violations
    }

    fn visit(&self, value: &Value, path: &str, violations: &mut Vec<(String, String)>) {
        match value {
            Value::String(text) => match Template::parse(text) {
                Ok(template) => {
                    for expression in template.expressions() {
                        if let Err(message) = self.check_expression(expression) {
                            violations.push((path.to_owned(), message));
                        }
                    }
                }
                Err(message) => violations.push((path.to_owned(), message)),
            },
            Value::Array(items) => {
                for (index, item) in items.iter().enumerate() {
                    self.visit(item, &format!("{path}.{index}"), violations);
                }
            }
            Value::Object(map) => {
                for (key, item) in map {
                    self.visit(item, &format!("{path}.{key}"), violations);
                }
            }
            _ => {}
        }
    }
}
