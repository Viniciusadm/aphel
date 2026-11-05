use serde_json::Value;

pub(crate) const MAX_PATH_LENGTH: usize = 200;
const TEMPLATE_OPEN: &str = "{{";
const TEMPLATE_CLOSE: &str = "}}";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PathSegment {
    Key(String),
    Index(usize),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct JsonPath {
    segments: Vec<PathSegment>,
}

impl JsonPath {
    pub(crate) fn parse(raw: &str) -> Result<Self, String> {
        let raw = raw.trim();
        if raw.len() > MAX_PATH_LENGTH {
            return Err(format!("must have at most {MAX_PATH_LENGTH} characters"));
        }
        let Some(mut rest) = raw.strip_prefix('$') else {
            return Err("must start with $".to_owned());
        };
        let mut segments = Vec::new();
        while !rest.is_empty() {
            if let Some(after) = rest.strip_prefix('.') {
                let end = after.find(['.', '[']).unwrap_or(after.len());
                let key = &after[..end];
                if key.is_empty() || key.contains(']') {
                    return Err(format!("invalid segment in {raw}"));
                }
                segments.push(PathSegment::Key(key.to_owned()));
                rest = &after[end..];
            } else if let Some(after) = rest.strip_prefix('[') {
                let end = after
                    .find(']')
                    .ok_or_else(|| format!("unclosed [ in {raw}"))?;
                let inner = after[..end].trim();
                let quoted = inner
                    .strip_prefix('"')
                    .and_then(|value| value.strip_suffix('"'))
                    .or_else(|| {
                        inner
                            .strip_prefix('\'')
                            .and_then(|value| value.strip_suffix('\''))
                    });
                match quoted {
                    Some(key) if !key.is_empty() => {
                        segments.push(PathSegment::Key(key.to_owned()));
                    }
                    Some(_) => return Err(format!("empty key in {raw}")),
                    None => {
                        let index = inner
                            .parse::<usize>()
                            .map_err(|_| format!("invalid index [{inner}] in {raw}"))?;
                        segments.push(PathSegment::Index(index));
                    }
                }
                rest = &after[end + 1..];
            } else {
                return Err(format!("unexpected character in {raw}"));
            }
        }
        Ok(Self { segments })
    }

    pub(crate) fn root(&self) -> Option<&str> {
        match self.segments.first() {
            Some(PathSegment::Key(key)) => Some(key),
            _ => None,
        }
    }

    pub(crate) fn action_index(&self) -> Option<usize> {
        match (self.segments.first(), self.segments.get(1)) {
            (Some(PathSegment::Key(root)), Some(PathSegment::Index(index)))
                if root == "actions" =>
            {
                Some(*index)
            }
            _ => None,
        }
    }

    pub(crate) fn lookup<'a>(&self, root: &'a Value) -> Option<&'a Value> {
        self.segments
            .iter()
            .try_fold(root, |current, segment| match segment {
                PathSegment::Key(key) => current.as_object()?.get(key),
                PathSegment::Index(index) => current.as_array()?.get(*index),
            })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum MappingExpression {
    Path(JsonPath),
    Template(Vec<TemplatePart>),
    Constant(Value),
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum TemplatePart {
    Text(String),
    Path(JsonPath),
}

impl MappingExpression {
    pub(crate) fn parse(value: &Value) -> Result<Self, String> {
        let Value::String(text) = value else {
            return match value {
                Value::Array(_) | Value::Object(_) => {
                    Err("must be a $.path, a text template or a scalar constant".to_owned())
                }
                other => Ok(Self::Constant(other.clone())),
            };
        };
        let trimmed = text.trim();
        if trimmed.starts_with('$') && !trimmed.contains(char::is_whitespace) {
            return JsonPath::parse(trimmed).map(Self::Path);
        }
        if text.contains(TEMPLATE_OPEN) {
            return parse_template(text).map(Self::Template);
        }
        Ok(Self::Constant(value.clone()))
    }

    pub(crate) fn paths(&self) -> Vec<&JsonPath> {
        match self {
            Self::Path(path) => vec![path],
            Self::Template(parts) => parts
                .iter()
                .filter_map(|part| match part {
                    TemplatePart::Path(path) => Some(path),
                    TemplatePart::Text(_) => None,
                })
                .collect(),
            Self::Constant(_) => Vec::new(),
        }
    }

    pub(crate) fn evaluate(&self, payload: &Value) -> Option<Value> {
        match self {
            Self::Path(path) => path.lookup(payload).cloned(),
            Self::Constant(value) => Some(value.clone()),
            Self::Template(parts) => {
                let rendered: String = parts
                    .iter()
                    .map(|part| match part {
                        TemplatePart::Text(text) => text.clone(),
                        TemplatePart::Path(path) => path
                            .lookup(payload)
                            .and_then(scalar_text)
                            .unwrap_or_default(),
                    })
                    .collect();
                Some(Value::String(rendered.trim().to_owned()))
            }
        }
    }
}

fn parse_template(text: &str) -> Result<Vec<TemplatePart>, String> {
    let mut parts = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find(TEMPLATE_OPEN) {
        if start > 0 {
            parts.push(TemplatePart::Text(rest[..start].to_owned()));
        }
        let after = &rest[start + TEMPLATE_OPEN.len()..];
        let end = after
            .find(TEMPLATE_CLOSE)
            .ok_or_else(|| "unclosed {{ in template".to_owned())?;
        parts.push(TemplatePart::Path(JsonPath::parse(&after[..end])?));
        rest = &after[end + TEMPLATE_CLOSE.len()..];
    }
    if !rest.is_empty() {
        parts.push(TemplatePart::Text(rest.to_owned()));
    }
    Ok(parts)
}

pub(crate) fn scalar_text(value: &Value) -> Option<String> {
    match value {
        Value::Null => None,
        Value::String(text) => Some(text.clone()),
        Value::Bool(flag) => Some(flag.to_string()),
        Value::Number(number) => Some(number.to_string()),
        Value::Array(items) => {
            let texts: Vec<String> = items.iter().filter_map(scalar_text).collect();
            (!texts.is_empty()).then(|| texts.join(", "))
        }
        Value::Object(_) => None,
    }
}
