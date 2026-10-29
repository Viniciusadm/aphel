use std::collections::BTreeSet;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::application::config_packages::sections::custom_objects::package_registry;
use crate::application::config_packages::sections::fields::{package_fields, projected_field_set};
use crate::application::config_packages::{
    ApplyContext, ConfigPackage, ConfigSection, Decisions, PackageIssue, PlanChange, PlanItem,
    parse_section,
};
use crate::application::fields::{FieldDefinitionRepository, FieldSet, RecordTypeRegistry};
use crate::application::layouts::{
    FormLayout, FormLayoutRepository, LayoutContext, StoredFormLayout, validate_layout,
    with_required_fields,
};
use crate::application::query::ResolveContext;
use crate::domain::access::AccessScope;
use crate::errors::AppError;

pub const LAYOUTS_SECTION: &str = "layouts";
const LAYOUT_KIND: &str = "layout";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackageLayout {
    pub record_type: String,
    pub context: LayoutContext,
    pub layout: FormLayout,
}

struct LayoutPlan {
    item: PlanItem,
    stored: StoredFormLayout,
}

pub struct LayoutsSection<L, D> {
    registry: Arc<RecordTypeRegistry>,
    layouts: L,
    definitions: D,
}

impl<L, D> LayoutsSection<L, D>
where
    L: FormLayoutRepository,
    D: FieldDefinitionRepository,
{
    pub fn new(registry: Arc<RecordTypeRegistry>, layouts: L, definitions: D) -> Self {
        Self {
            registry,
            layouts,
            definitions,
        }
    }

    async fn projected(
        &self,
        package: &ConfigPackage,
        record_type: &str,
    ) -> Result<FieldSet, AppError> {
        let registry = package_registry(&self.registry, package);
        let definition = registry.require(record_type)?;
        let existing = self.definitions.list(&definition.record_type).await?;
        Ok(projected_field_set(
            definition,
            existing,
            &package_fields(package),
        ))
    }

    async fn reconcile(
        &self,
        scope: &AccessScope,
        package: &ConfigPackage,
        layouts: &[PackageLayout],
        at: DateTime<Utc>,
    ) -> Result<Vec<LayoutPlan>, AppError> {
        let mut plans = Vec::new();
        let registry = package_registry(&self.registry, package);
        for entry in layouts {
            let definition = registry.require(&entry.record_type)?;
            let layout = match entry.context {
                LayoutContext::Create => with_required_fields(
                    entry.layout.clone(),
                    &self.projected(package, &entry.record_type).await?,
                ),
                _ => entry.layout.clone(),
            };
            let item = PlanItem::new(
                LAYOUTS_SECTION,
                LAYOUT_KIND,
                format!("{}.{}", entry.record_type, entry.context.as_str()),
                format!("{} · {}", entry.record_type, entry.context.as_str()),
            );
            let current = self
                .layouts
                .find(&definition.record_type, entry.context)
                .await?;
            let proposed = layout_json(&layout)?;
            let (item, stored) = match current {
                None => (
                    item.created(vec![PlanChange::new("layout", Value::Null, proposed)]),
                    StoredFormLayout {
                        id: Uuid::now_v7(),
                        record_type: definition.record_type.clone(),
                        context: entry.context,
                        layout,
                        updated_by: Some(scope.user_id().clone()),
                        created_at: at,
                        updated_at: at,
                    },
                ),
                Some(current) => {
                    let existing = layout_json(&current.layout)?;
                    let changes = if existing == proposed {
                        Vec::new()
                    } else {
                        vec![PlanChange::new("layout", existing, proposed)]
                    };
                    (
                        item.compared(changes),
                        StoredFormLayout {
                            layout,
                            updated_by: Some(scope.user_id().clone()),
                            updated_at: at,
                            ..current
                        },
                    )
                }
            };
            plans.push(LayoutPlan { item, stored });
        }
        Ok(plans)
    }
}

fn layout_json(layout: &FormLayout) -> Result<Value, AppError> {
    serde_json::to_value(layout).map_err(|error| AppError::Internal(error.to_string()))
}

fn parse(content: &Value) -> Result<Vec<PackageLayout>, AppError> {
    parse_section(content).map_err(|issue| AppError::Validation(issue.message))
}

#[async_trait]
impl<L, D, Tx> ConfigSection<Tx> for LayoutsSection<L, D>
where
    Tx: Send,
    L: FormLayoutRepository<Tx = Tx>,
    D: FieldDefinitionRepository,
{
    fn key(&self) -> &'static str {
        LAYOUTS_SECTION
    }

    async fn export(&self, _scope: &AccessScope) -> Result<Option<Value>, AppError> {
        let mut record_types: Vec<_> = self
            .registry
            .definitions()
            .map(|definition| definition.record_type.clone())
            .collect();
        record_types.sort();
        let mut layouts = Vec::new();
        for record_type in record_types {
            for context in LayoutContext::ALL {
                if let Some(stored) = self.layouts.find(&record_type, context).await? {
                    layouts.push(PackageLayout {
                        record_type: record_type.as_str().to_owned(),
                        context,
                        layout: stored.layout,
                    });
                }
            }
        }
        Ok(Some(json!(layouts)))
    }

    async fn validate(
        &self,
        scope: &AccessScope,
        package: &ConfigPackage,
        content: &Value,
    ) -> Result<Vec<PackageIssue>, AppError> {
        let layouts: Vec<PackageLayout> = match parse_section(content) {
            Ok(layouts) => layouts,
            Err(issue) => return Ok(vec![issue]),
        };
        let mut issues = Vec::new();
        let mut seen = BTreeSet::new();
        let resolve = ResolveContext {
            now: DateTime::<Utc>::UNIX_EPOCH,
            timezone: chrono_tz::UTC,
            user_id: scope.user_id().clone(),
        };
        let registry = package_registry(&self.registry, package);
        for (index, entry) in layouts.iter().enumerate() {
            if registry.require(&entry.record_type).is_err() {
                issues.push(PackageIssue::new(
                    format!("{index}.record_type"),
                    "unknown_record_type",
                    "is not a registered record type",
                ));
                continue;
            }
            if !seen.insert((entry.record_type.clone(), entry.context.as_str())) {
                issues.push(PackageIssue::new(
                    format!("{index}.context"),
                    "duplicated_layout",
                    "is declared more than once for this record type",
                ));
            }
            let fields = self.projected(package, &entry.record_type).await?;
            issues.extend(
                validate_layout(&entry.layout, &fields, entry.context, &resolve)
                    .into_iter()
                    .map(|violation| {
                        PackageIssue::new(
                            format!("{index}.{}", violation.field),
                            "invalid_layout",
                            violation.message,
                        )
                    }),
            );
        }
        Ok(issues)
    }

    async fn plan(
        &self,
        scope: &AccessScope,
        package: &ConfigPackage,
        content: &Value,
    ) -> Result<Vec<PlanItem>, AppError> {
        Ok(self
            .reconcile(
                scope,
                package,
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
        for plan in self
            .reconcile(scope, package, &parse(content)?, context.at)
            .await?
        {
            if decisions.executes(&plan.item) {
                self.layouts.upsert(tx, &plan.stored).await?;
            }
        }
        Ok(())
    }
}
