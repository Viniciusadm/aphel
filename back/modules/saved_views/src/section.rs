use aphel_sdk::configuration::ConfigReferenceKind;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use uuid::Uuid;

use crate::application::{
    MAX_VIEW_COLUMNS, MAX_VIEW_NAME_LENGTH, MAX_VIEW_SETTINGS_BYTES, SavedView,
    SavedViewRepository, ViewType, ViewVisibility,
};
use aphel_kernel::errors::AppError;
use aphel_sdk::configuration::sections::common::{check_key, diff};
use aphel_sdk::configuration::sections::custom_objects::package_registry;
use aphel_sdk::configuration::sections::fields::{package_fields, projected_field_set};
use aphel_sdk::configuration::{
    ApplyContext, ConfigKeyRepository, ConfigPackage, ConfigSection, Decisions, KeyedItem,
    PackageIssue, PlanAction, PlanChange, PlanItem, UnavailableFields, assign_keys,
    field_reference, parse_section,
};
use aphel_sdk::platform::application::fields::{FieldDefinitionRepository, RecordTypeRegistry};
use aphel_sdk::platform::application::query::{
    Filter, FilterCondition, FilterOperator, ResolveContext, SortSpec, validate_view_query,
};
use aphel_sdk::platform::application::shared::RecordType;
use aphel_sdk::platform::domain::access::AccessScope;

pub const VIEWS_SECTION: &str = "views";
const VIEW_KIND: &str = "view";
pub const MAX_PACKAGE_VIEWS: usize = 200;
pub const NON_PORTABLE_REFERENCE: &str = "non_portable_reference";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackageView {
    pub key: String,
    pub record_type: String,
    pub name: String,
    #[serde(default = "default_view_type")]
    pub view_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filter: Option<Filter>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sort: Vec<SortSpec>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub columns: Vec<String>,
    #[serde(default, skip_serializing_if = "Map::is_empty")]
    pub settings: Map<String, Value>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub default: bool,
}

fn default_view_type() -> String {
    ViewType::Table.as_str().to_owned()
}

fn is_false(value: &bool) -> bool {
    !*value
}

struct ViewPlan {
    item: PlanItem,
    view: SavedView,
    is_new: bool,
    key: String,
}

pub struct ViewsSection<V, K, D> {
    registry: Arc<RecordTypeRegistry>,
    views: V,
    keys: K,
    definitions: D,
}

impl<V, K, D> ViewsSection<V, K, D>
where
    V: SavedViewRepository,
    K: ConfigKeyRepository,
    D: FieldDefinitionRepository,
{
    pub fn new(registry: Arc<RecordTypeRegistry>, views: V, keys: K, definitions: D) -> Self {
        Self {
            registry,
            views,
            keys,
            definitions,
        }
    }

    async fn keyed(
        &self,
        scope: &AccessScope,
        record_type: &RecordType,
    ) -> Result<Vec<(SavedView, KeyedItem)>, AppError> {
        let mut views: Vec<SavedView> = self
            .views
            .list_visible(scope, record_type)
            .await?
            .into_iter()
            .filter(|view| view.visibility == ViewVisibility::Organization)
            .collect();
        views.sort_by_key(|view| (view.position, view.id));
        let stored = self
            .keys
            .keys(ConfigReferenceKind::new("views.view"))
            .await?;
        let keyed = assign_keys(
            views.iter().map(|view| (view.id, view.name.as_str())),
            &stored,
        );
        Ok(views.into_iter().zip(keyed).collect())
    }

    async fn reconcile(
        &self,
        scope: &AccessScope,
        registry: &RecordTypeRegistry,
        package: &[PackageView],
        at: DateTime<Utc>,
    ) -> Result<Vec<ViewPlan>, AppError> {
        let mut existing: BTreeMap<String, Vec<(SavedView, KeyedItem)>> = BTreeMap::new();
        let mut positions: BTreeMap<String, i32> = BTreeMap::new();
        let mut plans = Vec::new();
        for entry in package {
            let definition = registry.require(&entry.record_type)?;
            if !existing.contains_key(&entry.record_type) {
                existing.insert(
                    entry.record_type.clone(),
                    self.keyed(scope, &definition.record_type).await?,
                );
                positions.insert(
                    entry.record_type.clone(),
                    self.views.next_position(&definition.record_type).await?,
                );
            }
            let item = PlanItem::new(
                VIEWS_SECTION,
                VIEW_KIND,
                format!("{}.{}", entry.record_type, entry.key),
                entry.name.trim(),
            );
            let current = existing
                .get(&entry.record_type)
                .and_then(|views| views.iter().find(|(_, keyed)| keyed.key == entry.key))
                .map(|(view, _)| view.clone());
            let view_type = ViewType::parse(&entry.view_type).unwrap_or(ViewType::Table);
            match current {
                None => {
                    let position = positions
                        .get_mut(&entry.record_type)
                        .expect("positions are loaded with the views");
                    let created = SavedView {
                        id: Uuid::now_v7(),
                        record_type: definition.record_type.clone(),
                        name: entry.name.trim().to_owned(),
                        owner_id: Some(scope.user_id().clone()),
                        visibility: ViewVisibility::Organization,
                        team_id: None,
                        view_type,
                        filter: entry.filter.clone(),
                        sort: entry.sort.clone(),
                        columns: entry.columns.clone(),
                        settings: entry.settings.clone(),
                        is_default: entry.default,
                        position: *position,
                        created_at: at,
                        updated_at: at,
                    };
                    *position += 1;
                    let changes = snapshot(&created)
                        .into_iter()
                        .map(|(property, value)| PlanChange::new(property, Value::Null, value))
                        .collect();
                    plans.push(ViewPlan {
                        item: portability_checked(item.created(changes), entry),
                        view: created,
                        is_new: true,
                        key: entry.key.clone(),
                    });
                }
                Some(current) => {
                    let updated = SavedView {
                        name: entry.name.trim().to_owned(),
                        view_type,
                        filter: entry.filter.clone(),
                        sort: entry.sort.clone(),
                        columns: entry.columns.clone(),
                        settings: entry.settings.clone(),
                        is_default: entry.default,
                        updated_at: at,
                        ..current.clone()
                    };
                    let changes = diff(&snapshot(&current), &snapshot(&updated));
                    plans.push(ViewPlan {
                        item: portability_checked(item.compared(changes), entry),
                        view: updated,
                        is_new: false,
                        key: entry.key.clone(),
                    });
                }
            }
        }
        Ok(plans)
    }
}

fn portability_checked(item: PlanItem, entry: &PackageView) -> PlanItem {
    if item.action == PlanAction::Unchanged
        || !entry
            .filter
            .as_ref()
            .is_some_and(has_non_portable_reference)
    {
        return item;
    }
    let changes = item.changes.clone();
    item.conflicting(
        changes,
        NON_PORTABLE_REFERENCE,
        "the filter references users, teams, records, tags, stages or segments by id, which do not exist in another organization; applying keeps the view without those conditions",
        true,
    )
}

pub fn has_non_portable_reference(filter: &Filter) -> bool {
    match filter {
        Filter::Group(group) => group.conditions.iter().any(has_non_portable_reference),
        Filter::Condition(condition) => is_non_portable(condition),
        Filter::Related(related) => has_non_portable_reference(&related.related.filter),
    }
}

pub fn portable_filter(filter: Filter) -> Option<Filter> {
    match filter {
        Filter::Group(mut group) => {
            let had_conditions = !group.conditions.is_empty();
            group.conditions = group
                .conditions
                .into_iter()
                .filter_map(portable_filter)
                .collect();
            (!had_conditions || !group.conditions.is_empty()).then_some(Filter::Group(group))
        }
        Filter::Condition(condition) => {
            (!is_non_portable(&condition)).then_some(Filter::Condition(condition))
        }
        Filter::Related(mut related) => {
            let inner = std::mem::replace(&mut *related.related.filter, Filter::and(Vec::new()));
            *related.related.filter = portable_filter(inner)?;
            Some(Filter::Related(related))
        }
    }
}

fn is_non_portable(condition: &FilterCondition) -> bool {
    condition.operator == FilterOperator::InSavedFilter || holds_identifier(&condition.value)
}

fn holds_identifier(value: &Value) -> bool {
    match value {
        Value::String(text) => {
            let text = text.trim();
            text.len() == 36 && Uuid::parse_str(text).is_ok()
        }
        Value::Array(items) => items.iter().any(holds_identifier),
        _ => false,
    }
}

fn snapshot(view: &SavedView) -> Vec<(String, Value)> {
    vec![
        ("name".to_owned(), json!(view.name)),
        ("view_type".to_owned(), json!(view.view_type.as_str())),
        ("filter".to_owned(), json!(view.filter)),
        ("sort".to_owned(), json!(view.sort)),
        ("columns".to_owned(), json!(view.columns)),
        ("settings".to_owned(), Value::Object(view.settings.clone())),
        ("default".to_owned(), json!(view.is_default)),
    ]
}

fn parse(content: &Value) -> Result<Vec<PackageView>, AppError> {
    parse_section(content).map_err(|issue| AppError::Validation(issue.message))
}

fn prune_views(content: &mut Value, unavailable: &UnavailableFields) -> Vec<String> {
    let mut removed = Vec::new();
    let Some(views) = content.as_array_mut() else {
        return removed;
    };
    views.retain_mut(|view| {
        let Some(record_type) = view
            .get("record_type")
            .and_then(Value::as_str)
            .map(str::to_owned)
        else {
            return true;
        };
        if let Some(key) = view
            .get("filter")
            .and_then(|filter| unavailable.referenced_in(&record_type, filter))
        {
            removed.push(field_reference(&record_type, &key));
            return false;
        }
        if let Some(columns) = view.get_mut("columns").and_then(Value::as_array_mut) {
            columns.retain(|column| match column.as_str() {
                Some(key) if unavailable.contains(&record_type, key) => {
                    removed.push(field_reference(&record_type, key));
                    false
                }
                _ => true,
            });
        }
        if let Some(sort) = view.get_mut("sort").and_then(Value::as_array_mut) {
            sort.retain(|entry| match entry.get("field").and_then(Value::as_str) {
                Some(key) if unavailable.contains(&record_type, key) => {
                    removed.push(field_reference(&record_type, key));
                    false
                }
                _ => true,
            });
        }
        true
    });
    removed
}

#[async_trait]
impl<V, K, D, Tx> ConfigSection<Tx> for ViewsSection<V, K, D>
where
    Tx: Send,
    V: SavedViewRepository<Tx = Tx>,
    K: ConfigKeyRepository<Tx = Tx>,
    D: FieldDefinitionRepository,
{
    fn key(&self) -> &'static str {
        VIEWS_SECTION
    }

    fn prune(&self, content: &mut Value, unavailable: &UnavailableFields) -> Vec<String> {
        prune_views(content, unavailable)
    }

    async fn export(&self, scope: &AccessScope) -> Result<Option<Value>, AppError> {
        let mut record_types: Vec<RecordType> = self
            .registry
            .definitions()
            .map(|definition| definition.record_type.clone())
            .collect();
        record_types.sort();
        let mut views = Vec::new();
        for record_type in record_types {
            for (view, keyed) in self.keyed(scope, &record_type).await? {
                views.push(PackageView {
                    key: keyed.key,
                    record_type: record_type.as_str().to_owned(),
                    name: view.name,
                    view_type: view.view_type.as_str().to_owned(),
                    filter: view.filter,
                    sort: view.sort,
                    columns: view.columns,
                    settings: view.settings,
                    default: view.is_default,
                });
            }
        }
        Ok(Some(json!(views)))
    }

    async fn validate(
        &self,
        scope: &AccessScope,
        package: &ConfigPackage,
        content: &Value,
    ) -> Result<Vec<PackageIssue>, AppError> {
        let views: Vec<PackageView> = match parse_section(content) {
            Ok(views) => views,
            Err(issue) => return Ok(vec![issue]),
        };
        let mut issues = Vec::new();
        if views.len() > MAX_PACKAGE_VIEWS {
            issues.push(PackageIssue::new(
                "",
                "too_many_views",
                format!("a package can have at most {MAX_PACKAGE_VIEWS} views"),
            ));
        }
        let resolve = ResolveContext {
            now: DateTime::<Utc>::UNIX_EPOCH,
            timezone: chrono_tz::UTC,
            user_id: scope.user_id().clone(),
        };
        let fields_in_package = package_fields(package);
        let registry = package_registry(&self.registry, package);
        let mut keys: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        let mut defaults: BTreeSet<String> = BTreeSet::new();
        for (index, view) in views.iter().enumerate() {
            let Ok(definition) = registry.require(&view.record_type) else {
                issues.push(PackageIssue::new(
                    format!("{index}.record_type"),
                    "unknown_record_type",
                    "is not a registered record type",
                ));
                continue;
            };
            check_key(
                &view.key,
                &format!("{index}.key"),
                keys.entry(view.record_type.clone()).or_default(),
                &mut issues,
            );
            let name = view.name.trim();
            if name.is_empty() || name.chars().count() > MAX_VIEW_NAME_LENGTH {
                issues.push(PackageIssue::new(
                    format!("{index}.name"),
                    "invalid_name",
                    format!("must have between 1 and {MAX_VIEW_NAME_LENGTH} characters"),
                ));
            }
            if ViewType::parse(&view.view_type).is_none() {
                issues.push(PackageIssue::new(
                    format!("{index}.view_type"),
                    "invalid_view_type",
                    "must be table, kanban or calendar",
                ));
            }
            if Value::Object(view.settings.clone()).to_string().len() > MAX_VIEW_SETTINGS_BYTES {
                issues.push(PackageIssue::new(
                    format!("{index}.settings"),
                    "settings_too_large",
                    format!("must have at most {MAX_VIEW_SETTINGS_BYTES} bytes"),
                ));
            }
            if view.default && !defaults.insert(view.record_type.clone()) {
                issues.push(PackageIssue::new(
                    format!("{index}.default"),
                    "multiple_defaults",
                    "only one view per record type can be the default",
                ));
            }
            let existing = self.definitions.list(&definition.record_type).await?;
            let fields = projected_field_set(definition, existing, &fields_in_package);
            issues.extend(
                validate_view_query(&fields, &resolve, view.filter.as_ref(), &view.sort)
                    .into_iter()
                    .map(|violation| {
                        PackageIssue::new(
                            format!("{index}.{}", violation.field),
                            "invalid_view",
                            violation.message,
                        )
                    }),
            );
            if view.columns.len() > MAX_VIEW_COLUMNS {
                issues.push(PackageIssue::new(
                    format!("{index}.columns"),
                    "too_many_columns",
                    format!("at most {MAX_VIEW_COLUMNS} columns are allowed"),
                ));
            }
            let mut seen = BTreeSet::new();
            for (column_index, column) in view.columns.iter().enumerate() {
                if fields.find_active(column).is_none() {
                    issues.push(PackageIssue::new(
                        format!("{index}.columns.{column_index}"),
                        "unknown_field",
                        "is not a known field",
                    ));
                } else if !seen.insert(column) {
                    issues.push(PackageIssue::new(
                        format!("{index}.columns.{column_index}"),
                        "duplicated_column",
                        "is duplicated",
                    ));
                }
            }
        }
        Ok(issues)
    }

    async fn plan(
        &self,
        scope: &AccessScope,
        package: &ConfigPackage,
        content: &Value,
    ) -> Result<Vec<PlanItem>, AppError> {
        let registry = package_registry(&self.registry, package);
        Ok(self
            .reconcile(
                scope,
                &registry,
                &parse(content)?,
                DateTime::<Utc>::UNIX_EPOCH,
            )
            .await?
            .into_iter()
            .map(|plan| plan.item)
            .collect())
    }

    async fn apply(
        &self,
        tx: &mut Tx,
        scope: &AccessScope,
        package: &ConfigPackage,
        content: &Value,
        decisions: &Decisions,
        context: &mut ApplyContext,
    ) -> Result<(), AppError> {
        let registry = package_registry(&self.registry, package);
        for mut plan in self
            .reconcile(scope, &registry, &parse(content)?, context.at)
            .await?
        {
            if decisions.executes(&plan.item) {
                if plan.item.action == PlanAction::SkipConflict {
                    plan.view.filter = plan.view.filter.take().and_then(portable_filter);
                }
                if plan.is_new {
                    self.views.insert(tx, &plan.view).await?;
                } else {
                    self.views.update(tx, &plan.view).await?;
                }
                if plan.view.is_default {
                    self.views
                        .clear_organization_default(tx, &plan.view.record_type, plan.view.id)
                        .await?;
                }
            }
            self.keys
                .assign(
                    tx,
                    ConfigReferenceKind::new("views.view"),
                    plan.view.id,
                    &plan.key,
                )
                .await?;
            context.resolve(
                ConfigReferenceKind::new("views.view"),
                &plan.key,
                plan.view.id,
            );
        }
        Ok(())
    }
}
