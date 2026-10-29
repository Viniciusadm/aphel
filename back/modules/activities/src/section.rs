use std::collections::BTreeSet;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::domain::{
    ActivityTypeView, normalize_activity_type_name, validate_activity_type_color,
    validate_activity_type_icon,
};
use crate::store::ActivityTypeStore;
use aphel_kernel::application::config_packages::sections::common::{check_key, diff};
use aphel_kernel::application::config_packages::{
    ApplyContext, ConfigPackage, ConfigSection, Decisions, PackageIssue, PlanChange, PlanItem,
    parse_section,
};
use aphel_kernel::database::transaction::MySqlTx;
use aphel_kernel::domain::access::AccessScope;
use aphel_kernel::errors::AppError;

pub const ACTIVITY_TYPES_SECTION: &str = "activity_types";
const ACTIVITY_TYPE_KIND: &str = "activity_type";
pub const ACTIVITY_TYPE_ITEM: &str = "activity_type";
pub(crate) const MAX_PACKAGE_ACTIVITY_TYPES: usize = 100;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackageActivityType {
    pub key: String,
    pub(crate) name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) icon: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) color: Option<String>,
}

struct ActivityTypePlan {
    item: PlanItem,
    activity_type: ActivityTypeView,
    is_new: bool,
}

pub(crate) struct ActivityTypesSection {
    activity_types: ActivityTypeStore,
}

pub(crate) fn activity_types_section(pool: sqlx::MySqlPool) -> ActivityTypesSection {
    ActivityTypesSection::new(ActivityTypeStore::new(pool))
}

impl ActivityTypesSection {
    pub(crate) fn new(activity_types: ActivityTypeStore) -> Self {
        Self { activity_types }
    }

    async fn reconcile(
        &self,
        package: &[PackageActivityType],
        at: DateTime<Utc>,
    ) -> Result<Vec<ActivityTypePlan>, AppError> {
        let existing = self.activity_types.list(true).await?;
        let mut position = existing
            .iter()
            .map(|activity_type| activity_type.position)
            .max()
            .unwrap_or(-1);
        let mut plans = Vec::new();
        for entry in package {
            let name =
                normalize_activity_type_name(&entry.name).unwrap_or_else(|_| entry.name.clone());
            let icon = entry
                .icon
                .as_deref()
                .and_then(|icon| validate_activity_type_icon(icon).ok());
            let color = entry
                .color
                .as_deref()
                .and_then(|color| validate_activity_type_color(color).ok());
            let item = PlanItem::new(
                ACTIVITY_TYPES_SECTION,
                ACTIVITY_TYPE_KIND,
                entry.key.clone(),
                name.clone(),
            );
            match existing.iter().find(|current| current.key == entry.key) {
                None => {
                    position += 1;
                    let created = ActivityTypeView {
                        id: Uuid::now_v7(),
                        key: entry.key.clone(),
                        name,
                        icon,
                        color,
                        is_system: false,
                        is_active: true,
                        position,
                        created_at: at,
                        updated_at: at,
                    };
                    let changes = snapshot(&created)
                        .into_iter()
                        .map(|(property, value)| PlanChange::new(property, Value::Null, value))
                        .collect();
                    plans.push(ActivityTypePlan {
                        item: item.created(changes),
                        activity_type: created,
                        is_new: true,
                    });
                }
                Some(current) => {
                    let updated = ActivityTypeView {
                        name,
                        icon,
                        color,
                        is_active: true,
                        updated_at: at,
                        ..current.clone()
                    };
                    let changes = diff(&snapshot(current), &snapshot(&updated));
                    plans.push(ActivityTypePlan {
                        item: item.compared(changes),
                        activity_type: updated,
                        is_new: false,
                    });
                }
            }
        }
        Ok(plans)
    }
}

fn snapshot(activity_type: &ActivityTypeView) -> Vec<(String, Value)> {
    vec![
        ("name".to_owned(), json!(activity_type.name)),
        ("icon".to_owned(), json!(activity_type.icon)),
        ("color".to_owned(), json!(activity_type.color)),
        ("active".to_owned(), json!(activity_type.is_active)),
    ]
}

fn parse(content: &Value) -> Result<Vec<PackageActivityType>, AppError> {
    parse_section(content).map_err(|issue| AppError::Validation(issue.message))
}

#[async_trait]
impl ConfigSection<MySqlTx> for ActivityTypesSection {
    fn key(&self) -> &'static str {
        ACTIVITY_TYPES_SECTION
    }

    async fn export(&self, _scope: &AccessScope) -> Result<Option<Value>, AppError> {
        let mut types = self.activity_types.list(false).await?;
        types.sort_by_key(|activity_type| (activity_type.position, activity_type.id));
        let types: Vec<PackageActivityType> = types
            .into_iter()
            .filter(|activity_type| activity_type.is_active)
            .map(|activity_type| PackageActivityType {
                key: activity_type.key,
                name: activity_type.name,
                icon: activity_type.icon,
                color: activity_type.color,
            })
            .collect();
        Ok(Some(json!(types)))
    }

    async fn validate(
        &self,
        _scope: &AccessScope,
        _package: &ConfigPackage,
        content: &Value,
    ) -> Result<Vec<PackageIssue>, AppError> {
        let types: Vec<PackageActivityType> = match parse_section(content) {
            Ok(types) => types,
            Err(issue) => return Ok(vec![issue]),
        };
        let mut issues = Vec::new();
        if types.len() > MAX_PACKAGE_ACTIVITY_TYPES {
            issues.push(PackageIssue::new(
                "",
                "too_many_activity_types",
                format!("a package can have at most {MAX_PACKAGE_ACTIVITY_TYPES} activity types"),
            ));
        }
        let mut keys = BTreeSet::new();
        for (index, activity_type) in types.iter().enumerate() {
            check_key(
                &activity_type.key,
                &format!("{index}.key"),
                &mut keys,
                &mut issues,
            );
            if let Err(message) = normalize_activity_type_name(&activity_type.name) {
                issues.push(PackageIssue::new(
                    format!("{index}.name"),
                    "invalid_name",
                    message,
                ));
            }
            if let Some(Err(message)) = activity_type
                .icon
                .as_deref()
                .map(validate_activity_type_icon)
            {
                issues.push(PackageIssue::new(
                    format!("{index}.icon"),
                    "invalid_icon",
                    message,
                ));
            }
            if let Some(Err(message)) = activity_type
                .color
                .as_deref()
                .map(validate_activity_type_color)
            {
                issues.push(PackageIssue::new(
                    format!("{index}.color"),
                    "invalid_color",
                    message,
                ));
            }
        }
        Ok(issues)
    }

    async fn plan(
        &self,
        _scope: &AccessScope,
        _package: &ConfigPackage,
        content: &Value,
    ) -> Result<Vec<PlanItem>, AppError> {
        Ok(self
            .reconcile(&parse(content)?, DateTime::<Utc>::UNIX_EPOCH)
            .await?
            .into_iter()
            .map(|plan| plan.item)
            .collect())
    }

    async fn apply(
        &self,
        tx: &mut MySqlTx,
        _scope: &AccessScope,
        _package: &ConfigPackage,
        content: &Value,
        decisions: &Decisions,
        context: &mut ApplyContext,
    ) -> Result<(), AppError> {
        for plan in self.reconcile(&parse(content)?, context.at).await? {
            if plan.is_new {
                self.activity_types.insert(tx, &plan.activity_type).await?;
            } else if decisions.executes(&plan.item) {
                self.activity_types.update(tx, &plan.activity_type).await?;
            }
            context.resolve(
                ACTIVITY_TYPE_ITEM,
                &plan.activity_type.key,
                plan.activity_type.id,
            );
        }
        Ok(())
    }
}
