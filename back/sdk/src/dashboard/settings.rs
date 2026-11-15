use serde_json::{Map, Value};

use aphel_kernel::errors::{AppError, FieldViolation};

pub struct SettingsReader<'a> {
    settings: Option<&'a Map<String, Value>>,
    known: Vec<&'static str>,
    normalized: Map<String, Value>,
    violations: Vec<FieldViolation>,
}

impl<'a> SettingsReader<'a> {
    pub fn new(settings: &'a Value) -> Result<Self, Vec<FieldViolation>> {
        let settings = match settings {
            Value::Null => None,
            Value::Object(map) => Some(map),
            _ => return Err(vec![FieldViolation::new("settings", "must be an object")]),
        };
        Ok(Self {
            settings,
            known: Vec::new(),
            normalized: Map::new(),
            violations: Vec::new(),
        })
    }

    fn get(&self, key: &str) -> Option<&'a Value> {
        self.settings
            .and_then(|settings| settings.get(key))
            .filter(|value| !value.is_null())
    }

    pub fn integer(&mut self, key: &'static str, default: i64, min: i64, max: i64) -> i64 {
        self.known.push(key);
        let value = match self.get(key) {
            None => default,
            Some(value) => match value.as_i64() {
                Some(number) if (min..=max).contains(&number) => number,
                _ => {
                    self.violations.push(FieldViolation::new(
                        format!("settings.{key}"),
                        format!("must be an integer between {min} and {max}"),
                    ));
                    default
                }
            },
        };
        self.normalized.insert(key.to_owned(), Value::from(value));
        value
    }

    pub fn boolean(&mut self, key: &'static str, default: bool) -> bool {
        self.known.push(key);
        let value = match self.get(key) {
            None => default,
            Some(Value::Bool(value)) => *value,
            Some(_) => {
                self.violations.push(FieldViolation::new(
                    format!("settings.{key}"),
                    "must be a boolean",
                ));
                default
            }
        };
        self.normalized.insert(key.to_owned(), Value::Bool(value));
        value
    }

    pub fn finish(mut self) -> Result<Value, Vec<FieldViolation>> {
        if let Some(settings) = self.settings {
            for key in settings.keys() {
                if !self.known.contains(&key.as_str()) {
                    self.violations.push(FieldViolation::new(
                        format!("settings.{key}"),
                        "is not a setting of this widget",
                    ));
                }
            }
        }
        if self.violations.is_empty() {
            Ok(Value::Object(self.normalized))
        } else {
            Err(self.violations)
        }
    }
}

pub fn invalid_settings(violations: Vec<FieldViolation>) -> AppError {
    AppError::invalid_fields("invalid widget settings", violations)
}
