use std::collections::BTreeSet;

use async_trait::async_trait;
use serde_json::{Value, json};

use aphel_kernel::application::config_packages::{
    ApplyContext, ConfigPackage, ConfigSection, Decisions, PackageIssue, PlanChange, PlanItem,
};
use aphel_kernel::context::Ctx;
use aphel_kernel::database::transaction::MySqlTx;
use aphel_kernel::domain::access::AccessScope;
use aphel_kernel::errors::AppError;
use aphel_kernel::state::runtime_state;

use crate::errors::TEMPLATE_INVALID;
use crate::markup;
use crate::service::{self, clean_description};
use crate::store::{self, TemplateValues};
use crate::variables;

pub(crate) const SECTION: &str = "proposals.templates";
const KIND: &str = "template";

pub(crate) struct TemplatesSection;

fn entries(content: &Value) -> Vec<Value> {
    content
        .get("templates")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

fn values(entry: &Value) -> TemplateValues {
    let text = |key: &str| {
        entry
            .get(key)
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned()
    };
    TemplateValues {
        key: text("key").trim().to_owned(),
        name: text("name").trim().to_owned(),
        description: clean_description(
            entry
                .get("description")
                .and_then(Value::as_str)
                .map(str::to_owned),
        ),
        body: text("body"),
        is_active: entry
            .get("is_active")
            .and_then(Value::as_bool)
            .unwrap_or(true),
        is_default: entry
            .get("is_default")
            .and_then(Value::as_bool)
            .unwrap_or(false),
    }
}

fn changes(current: &TemplateValues, proposed: &TemplateValues) -> Vec<PlanChange> {
    let pairs = [
        ("name", json!(current.name), json!(proposed.name)),
        (
            "description",
            json!(current.description),
            json!(proposed.description),
        ),
        ("body", json!(current.body), json!(proposed.body)),
        (
            "is_active",
            json!(current.is_active),
            json!(proposed.is_active),
        ),
        (
            "is_default",
            json!(current.is_default),
            json!(proposed.is_default),
        ),
    ];
    pairs
        .into_iter()
        .filter(|(_, current, proposed)| current != proposed)
        .map(|(property, current, proposed)| PlanChange::new(property, current, proposed))
        .collect()
}

#[async_trait]
impl ConfigSection<MySqlTx> for TemplatesSection {
    fn key(&self) -> &'static str {
        SECTION
    }

    async fn export(&self, _scope: &AccessScope) -> Result<Option<Value>, AppError> {
        let templates = store::list_templates(&runtime_state()?.db()?, false).await?;
        if templates.is_empty() {
            return Ok(None);
        }
        Ok(Some(json!({
            "templates": templates
                .iter()
                .map(|template| json!({
                    "key": template.key,
                    "name": template.name,
                    "description": template.description,
                    "body": template.body,
                    "is_active": template.is_active,
                    "is_default": template.is_default,
                }))
                .collect::<Vec<_>>()
        })))
    }

    async fn validate(
        &self,
        scope: &AccessScope,
        _package: &ConfigPackage,
        content: &Value,
    ) -> Result<Vec<PackageIssue>, AppError> {
        let Some(list) = content.get("templates").and_then(Value::as_array) else {
            return Ok(vec![PackageIssue::new(
                "templates",
                "invalid_section",
                "templates must be a list",
            )]);
        };
        let state = runtime_state()?;
        let ctx = Ctx::from_state(&state, scope).await?;
        let catalog = variables::catalog(&ctx).await?;
        let mut issues = Vec::new();
        let mut keys = BTreeSet::new();
        let mut defaults = 0;
        for (index, entry) in list.iter().enumerate() {
            let path = format!("templates[{index}]");
            if !entry.is_object() {
                issues.push(PackageIssue::new(
                    &path,
                    "invalid_section",
                    "must be an object",
                ));
                continue;
            }
            let values = values(entry);
            if !keys.insert(values.key.clone()) {
                issues.push(PackageIssue::new(
                    format!("{path}.key"),
                    "duplicated_key",
                    format!("the key {} appears more than once", values.key),
                ));
            }
            defaults += usize::from(values.is_default);
            for violation in service::value_violations(&values) {
                issues.push(PackageIssue::new(
                    format!("{path}.{}", violation.field),
                    TEMPLATE_INVALID,
                    violation.message,
                ));
            }
            for issue in catalog.validate(&markup::parse(&values.body)) {
                issues.push(PackageIssue::new(
                    format!("{path}.body"),
                    TEMPLATE_INVALID,
                    issue.describe(),
                ));
            }
        }
        if defaults > 1 {
            issues.push(PackageIssue::new(
                "templates",
                "multiple_defaults",
                "only one template can be the default",
            ));
        }
        Ok(issues)
    }

    async fn plan(
        &self,
        _scope: &AccessScope,
        _package: &ConfigPackage,
        content: &Value,
    ) -> Result<Vec<PlanItem>, AppError> {
        let pool = runtime_state()?.db()?;
        let mut items = Vec::new();
        for entry in entries(content) {
            let proposed = values(&entry);
            let item = PlanItem::new(SECTION, KIND, proposed.key.clone(), proposed.name.clone());
            let item = match store::find_template_by_key(&pool, &proposed.key).await? {
                None => item.created(Vec::new()),
                Some(current) => item.compared(changes(&TemplateValues::from(&current), &proposed)),
            };
            items.push(item);
        }
        Ok(items)
    }

    async fn apply(
        &self,
        tx: &mut MySqlTx,
        scope: &AccessScope,
        _package: &ConfigPackage,
        content: &Value,
        _decisions: &Decisions,
        _context: &mut ApplyContext,
    ) -> Result<(), AppError> {
        let state = runtime_state()?;
        let ctx = Ctx::from_state(&state, scope).await?;
        for entry in entries(content) {
            let proposed = values(&entry);
            let current = store::find_template_by_key(&mut **tx, &proposed.key).await?;
            match current {
                None => {
                    service::insert_template(&ctx, tx, &proposed).await?;
                }
                Some(current) => {
                    if TemplateValues::from(&current) != proposed {
                        service::write_template_update(&ctx, tx, &current, &proposed).await?;
                    }
                }
            }
        }
        Ok(())
    }
}
