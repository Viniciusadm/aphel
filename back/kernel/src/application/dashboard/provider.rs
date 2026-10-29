use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, SecondsFormat, Utc};
use chrono_tz::Tz;
use serde_json::{Map, Value, json};
use uuid::Uuid;

use crate::application::dashboard::DashboardPorts;
use crate::application::dashboard::DashboardRepository;
use crate::application::dashboard::WidgetSubject;
use crate::application::fields::{FieldDefinitionRepository, FieldSet};
use crate::application::organization_admin::ListWindow;
use crate::application::query::{
    AggregateEngine, AggregatePeriod, AggregateQuery, AggregateResult, ArchivedFilter, Filter,
    MetricSpec, QueryPlan, RecordQuery, RecordQueryPorts, ResolveContext, SortSpec, resolve_query,
};
use crate::application::shared::RecordType;
use crate::domain::access::{AccessScope, Permission, RecordScopeFilter};
use crate::domain::dashboard::ResolvedPeriod;
use crate::errors::{AppError, FieldViolation};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WidgetSize {
    Small,
    Medium,
    Large,
}

impl WidgetSize {
    pub const ALL: [Self; 3] = [Self::Small, Self::Medium, Self::Large];

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Small => "small",
            Self::Medium => "medium",
            Self::Large => "large",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|size| size.as_str() == value)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct WidgetDescriptor {
    pub key: String,
    pub title: String,
    pub category: String,
    pub required_permissions: Vec<Permission>,
    pub default_size: WidgetSize,
    pub uses_period: bool,
    pub default_settings: Value,
}

pub struct Planner {
    fields: FieldSet,
    context: ResolveContext,
    visibility: RecordScopeFilter,
    subject: Option<Filter>,
}

impl Planner {
    pub fn visibility(&self) -> &RecordScopeFilter {
        &self.visibility
    }

    pub fn plan(
        &self,
        mut conditions: Vec<Filter>,
        sort: Vec<SortSpec>,
        limit: u64,
    ) -> Result<QueryPlan, AppError> {
        conditions.extend(self.subject.clone());
        resolve_query(
            &self.fields,
            &self.context,
            self.visibility.clone(),
            RecordQuery {
                filter: if conditions.is_empty() {
                    None
                } else {
                    Some(Filter::and(conditions))
                },
                sort,
                search: None,
                archived: ArchivedFilter::Exclude,
                window: ListWindow { offset: 0, limit },
            },
        )
    }
}

pub struct WidgetContext<'a, P> {
    pub scope: &'a AccessScope,
    pub ports: &'a P,
    pub now: DateTime<Utc>,
    pub timezone: Tz,
    pub period: ResolvedPeriod,
    pub subject: Option<WidgetSubject>,
    pub compare_to_previous: bool,
    pub previous_period: Option<ResolvedPeriod>,
}

impl<P> WidgetContext<'_, P>
where
    P: DashboardPorts,
{
    pub async fn planner(&self, record_type: &RecordType) -> Result<Option<Planner>, AppError> {
        let query = self.ports.query();
        let Some(definition) = query.registry().get(record_type) else {
            return Ok(None);
        };
        let Some(visibility) = self.scope.filter_for(&definition.read_permission) else {
            return Ok(None);
        };
        let fields = FieldSet::new(
            definition,
            query.definitions().list(&definition.record_type).await?,
        );
        Ok(Some(Planner {
            fields,
            context: ResolveContext {
                now: self.now,
                timezone: self.timezone,
                user_id: self.scope.user_id().clone(),
            },
            visibility,
            subject: definition.owner_column.as_ref().and(self.subject_filter()),
        }))
    }

    pub fn subject_filter(&self) -> Option<Filter> {
        self.subject.as_ref().map(WidgetSubject::owner_filter)
    }

    pub fn with_subject(&self, mut conditions: Vec<Filter>) -> Vec<Filter> {
        conditions.extend(self.subject_filter());
        conditions
    }

    pub fn aggregate_period(&self, field: &str) -> AggregatePeriod {
        AggregatePeriod::between(
            field,
            &self.period.from.format("%Y-%m-%d").to_string(),
            &self.period.to.format("%Y-%m-%d").to_string(),
        )
    }

    pub async fn currency(&self) -> Result<String, AppError> {
        self.ports.dashboard().currency().await
    }

    pub fn can_read(&self, permission: &Permission) -> bool {
        self.scope.filter_for(permission).is_some()
    }
}

pub fn instant_value(instant: DateTime<Utc>) -> Value {
    Value::String(instant.to_rfc3339_opts(SecondsFormat::Micros, true))
}

pub async fn period_totals<P: DashboardPorts>(
    context: &WidgetContext<'_, P>,
    record_type: &str,
    conditions: Vec<Filter>,
    metrics: Vec<MetricSpec>,
    period_field: &str,
) -> Result<AggregateResult, AppError> {
    let query = AggregateQuery::new(record_type)
        .filter(Filter::and(context.with_subject(conditions)))
        .metrics(metrics)
        .period(context.aggregate_period(period_field))
        .compare(context.compare_to_previous);
    AggregateEngine::new(context.ports.query())
        .run(context.scope, query)
        .await
}

#[async_trait]
pub trait WidgetProvider<P>: Send + Sync
where
    P: DashboardPorts,
{
    fn key(&self) -> &str;
    fn title(&self) -> &str;
    fn category(&self) -> &str;
    fn required_permissions(&self) -> Vec<Permission>;

    fn is_available(&self, scope: &AccessScope) -> bool {
        self.required_permissions()
            .iter()
            .all(|permission| scope.scope_for(permission).is_some())
    }

    fn default_size(&self) -> WidgetSize {
        WidgetSize::Medium
    }

    fn uses_period(&self) -> bool {
        true
    }

    fn default_settings(&self) -> Value {
        json!({})
    }

    fn normalize_settings(&self, settings: &Value) -> Result<Value, Vec<FieldViolation>> {
        SettingsReader::new(settings)?.finish(Map::new())
    }

    async fn data(
        &self,
        context: &WidgetContext<'_, P>,
        settings: &Value,
    ) -> Result<Value, AppError>;

    fn descriptor(&self) -> WidgetDescriptor {
        WidgetDescriptor {
            key: self.key().to_owned(),
            title: self.title().to_owned(),
            category: self.category().to_owned(),
            required_permissions: self.required_permissions(),
            default_size: self.default_size(),
            uses_period: self.uses_period(),
            default_settings: self.default_settings(),
        }
    }
}

pub struct SettingsReader<'a> {
    settings: &'a Map<String, Value>,
    known: Vec<&'static str>,
    violations: Vec<FieldViolation>,
}

impl<'a> SettingsReader<'a> {
    pub fn new(settings: &'a Value) -> Result<Self, Vec<FieldViolation>> {
        static EMPTY: std::sync::OnceLock<Map<String, Value>> = std::sync::OnceLock::new();
        let settings = match settings {
            Value::Null => EMPTY.get_or_init(Map::new),
            Value::Object(map) => map,
            _ => {
                return Err(vec![FieldViolation::new("settings", "must be an object")]);
            }
        };
        Ok(Self {
            settings,
            known: Vec::new(),
            violations: Vec::new(),
        })
    }

    pub fn integer(&mut self, key: &'static str, default: i64, min: i64, max: i64) -> i64 {
        self.known.push(key);
        match self.settings.get(key) {
            None | Some(Value::Null) => default,
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
        }
    }

    pub fn boolean(&mut self, key: &'static str, default: bool) -> bool {
        self.known.push(key);
        match self.settings.get(key) {
            None | Some(Value::Null) => default,
            Some(Value::Bool(value)) => *value,
            Some(_) => {
                self.violations.push(FieldViolation::new(
                    format!("settings.{key}"),
                    "must be a boolean",
                ));
                default
            }
        }
    }

    pub fn uuid(&mut self, key: &'static str) -> Option<Uuid> {
        self.known.push(key);
        match self.settings.get(key) {
            None | Some(Value::Null) => None,
            Some(Value::String(text)) => match Uuid::parse_str(text) {
                Ok(id) => Some(id),
                Err(_) => {
                    self.violations.push(FieldViolation::new(
                        format!("settings.{key}"),
                        "must be a UUID",
                    ));
                    None
                }
            },
            Some(_) => {
                self.violations.push(FieldViolation::new(
                    format!("settings.{key}"),
                    "must be a UUID",
                ));
                None
            }
        }
    }

    pub fn finish(mut self, normalized: Map<String, Value>) -> Result<Value, Vec<FieldViolation>> {
        for key in self.settings.keys() {
            if !self.known.contains(&key.as_str()) {
                self.violations.push(FieldViolation::new(
                    format!("settings.{key}"),
                    "is not a setting of this widget",
                ));
            }
        }
        if self.violations.is_empty() {
            Ok(Value::Object(normalized))
        } else {
            Err(self.violations)
        }
    }
}

pub struct WidgetRegistry<P> {
    providers: Vec<Arc<dyn WidgetProvider<P>>>,
}

impl<P> Clone for WidgetRegistry<P> {
    fn clone(&self) -> Self {
        Self {
            providers: self.providers.clone(),
        }
    }
}

impl<P> WidgetRegistry<P>
where
    P: DashboardPorts,
{
    pub fn empty() -> Self {
        Self {
            providers: Vec::new(),
        }
    }

    pub fn register(mut self, provider: Arc<dyn WidgetProvider<P>>) -> Result<Self, AppError> {
        let key = provider.key();
        let valid = !key.is_empty()
            && key.len() <= 64
            && key.chars().all(|character| {
                character.is_ascii_lowercase() || character == '_' || character == '.'
            });
        if !valid {
            return Err(AppError::Configuration(format!(
                "widget key {key:?} must use lowercase letters, dots and underscores"
            )));
        }
        if self.get(key).is_some() {
            return Err(AppError::Configuration(format!(
                "widget {key} is registered twice"
            )));
        }
        self.providers.push(provider);
        Ok(self)
    }

    pub fn get(&self, key: &str) -> Option<&Arc<dyn WidgetProvider<P>>> {
        self.providers.iter().find(|provider| provider.key() == key)
    }

    pub fn providers(&self) -> impl Iterator<Item = &Arc<dyn WidgetProvider<P>>> {
        self.providers.iter()
    }

    pub fn available<'a>(
        &'a self,
        scope: &'a AccessScope,
    ) -> impl Iterator<Item = &'a Arc<dyn WidgetProvider<P>>> + 'a {
        self.providers
            .iter()
            .filter(move |provider| provider.is_available(scope))
    }
}
