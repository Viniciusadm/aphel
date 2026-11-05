use std::collections::BTreeSet;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::compose::html::sanitize_html;
use crate::store::sending::TemplateStore;
use crate::templates::{
    EmailTemplateView, MAX_TEMPLATE_BODY_BYTES, MAX_TEMPLATE_NAME_LENGTH,
    MAX_TEMPLATE_SUBJECT_LENGTH, NewEmailTemplate, TemplateFilter, TemplateListScope,
    TemplateScope, is_email_record_type,
};
use aphel_kernel::errors::AppError;
use aphel_sdk::configuration::sections::common::{check_key, diff};
use aphel_sdk::configuration::{
    ApplyContext, ConfigPackage, ConfigSection, Decisions, PackageIssue, PlanAction, PlanChange,
    PlanItem, parse_section,
};
use aphel_sdk::platform::database::transaction::MySqlTx;
use aphel_sdk::platform::domain::access::AccessScope;

pub(crate) const EMAIL_TEMPLATES_SECTION: &str = "email_templates";
const EMAIL_TEMPLATE_KIND: &str = "email_template";
pub(crate) const MAX_PACKAGE_EMAIL_TEMPLATES: usize = 200;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PackageEmailTemplate {
    pub(crate) key: String,
    pub(crate) name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) record_type: Option<String>,
    #[serde(default)]
    pub(crate) subject: String,
    #[serde(default)]
    pub(crate) body_html: String,
}

struct TemplatePlan {
    item: PlanItem,
    template: EmailTemplateView,
    is_new: bool,
}

pub(crate) struct EmailTemplatesSection {
    templates: TemplateStore,
}

impl EmailTemplatesSection {
    pub(crate) fn new(templates: TemplateStore) -> Self {
        Self { templates }
    }

    async fn organization_templates(
        &self,
        scope: &AccessScope,
    ) -> Result<Vec<EmailTemplateView>, AppError> {
        self.templates
            .list(&TemplateFilter {
                scope: TemplateListScope::Organization,
                viewer: scope.user_id().clone(),
                record_type: None,
            })
            .await
    }

    async fn reconcile(
        &self,
        scope: &AccessScope,
        package: &[PackageEmailTemplate],
        at: chrono::DateTime<chrono::Utc>,
    ) -> Result<Vec<TemplatePlan>, AppError> {
        let existing = self.organization_templates(scope).await?;
        let mut plans = Vec::new();
        for entry in package {
            let proposed = |current: Option<&EmailTemplateView>| EmailTemplateView {
                id: current.map_or_else(Uuid::now_v7, |current| current.id),
                key: entry.key.clone(),
                name: entry.name.split_whitespace().collect::<Vec<_>>().join(" "),
                scope: TemplateScope::Organization,
                owner_id: None,
                record_type: entry
                    .record_type
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(str::to_owned),
                subject: entry.subject.replace(['\r', '\n'], " ").trim().to_owned(),
                body_html: sanitize_html(&entry.body_html),
                created_at: current.map_or(at, |current| current.created_at),
                updated_at: at,
            };
            let item = PlanItem::new(
                EMAIL_TEMPLATES_SECTION,
                EMAIL_TEMPLATE_KIND,
                entry.key.clone(),
                entry.name.clone(),
            );
            match existing.iter().find(|template| template.key == entry.key) {
                None => {
                    let template = proposed(None);
                    let changes = snapshot(&template)
                        .into_iter()
                        .map(|(property, value)| PlanChange::new(property, Value::Null, value))
                        .collect();
                    plans.push(TemplatePlan {
                        item: item.created(changes),
                        template,
                        is_new: true,
                    });
                }
                Some(current) => {
                    let template = proposed(Some(current));
                    let changes = diff(&snapshot(current), &snapshot(&template));
                    plans.push(TemplatePlan {
                        item: item.compared(changes),
                        template,
                        is_new: false,
                    });
                }
            }
        }
        Ok(plans)
    }
}

fn snapshot(template: &EmailTemplateView) -> Vec<(String, Value)> {
    vec![
        ("name".to_owned(), json!(template.name)),
        ("record_type".to_owned(), json!(template.record_type)),
        ("subject".to_owned(), json!(template.subject)),
        ("body_html".to_owned(), json!(template.body_html)),
    ]
}

fn parse(content: &Value) -> Result<Vec<PackageEmailTemplate>, AppError> {
    parse_section(content).map_err(|issue| AppError::Validation(issue.message))
}

#[async_trait]
impl ConfigSection<MySqlTx> for EmailTemplatesSection {
    fn key(&self) -> &'static str {
        EMAIL_TEMPLATES_SECTION
    }

    async fn export(&self, scope: &AccessScope) -> Result<Option<Value>, AppError> {
        let templates: Vec<PackageEmailTemplate> = self
            .organization_templates(scope)
            .await?
            .into_iter()
            .map(|template| PackageEmailTemplate {
                key: template.key,
                name: template.name,
                record_type: template.record_type,
                subject: template.subject,
                body_html: template.body_html,
            })
            .collect();
        Ok(Some(json!(templates)))
    }

    async fn validate(
        &self,
        _scope: &AccessScope,
        _package: &ConfigPackage,
        content: &Value,
    ) -> Result<Vec<PackageIssue>, AppError> {
        let templates: Vec<PackageEmailTemplate> = match parse_section(content) {
            Ok(templates) => templates,
            Err(issue) => return Ok(vec![issue]),
        };
        let mut issues = Vec::new();
        if templates.len() > MAX_PACKAGE_EMAIL_TEMPLATES {
            issues.push(PackageIssue::new(
                "",
                "too_many_email_templates",
                format!(
                    "a package can have at most {MAX_PACKAGE_EMAIL_TEMPLATES} e-mail templates"
                ),
            ));
        }
        let mut keys = BTreeSet::new();
        for (index, template) in templates.iter().enumerate() {
            check_key(
                &template.key,
                &format!("{index}.key"),
                &mut keys,
                &mut issues,
            );
            let name = template.name.trim();
            if name.is_empty() || name.chars().count() > MAX_TEMPLATE_NAME_LENGTH {
                issues.push(PackageIssue::new(
                    format!("{index}.name"),
                    "invalid_name",
                    format!("must have 1 to {MAX_TEMPLATE_NAME_LENGTH} characters"),
                ));
            }
            if template.subject.chars().count() > MAX_TEMPLATE_SUBJECT_LENGTH {
                issues.push(PackageIssue::new(
                    format!("{index}.subject"),
                    "invalid_subject",
                    format!("must have at most {MAX_TEMPLATE_SUBJECT_LENGTH} characters"),
                ));
            }
            if template.body_html.len() > MAX_TEMPLATE_BODY_BYTES {
                issues.push(PackageIssue::new(
                    format!("{index}.body_html"),
                    "invalid_body",
                    format!("must have at most {MAX_TEMPLATE_BODY_BYTES} bytes"),
                ));
            }
            if let Some(record_type) = template
                .record_type
                .as_deref()
                .filter(|value| !value.trim().is_empty())
                && !is_email_record_type(record_type.trim())
            {
                issues.push(PackageIssue::new(
                    format!("{index}.record_type"),
                    "invalid_record_type",
                    "must be contact, company, deal or a custom object",
                ));
            }
        }
        Ok(issues)
    }

    async fn plan(
        &self,
        scope: &AccessScope,
        _package: &ConfigPackage,
        content: &Value,
    ) -> Result<Vec<PlanItem>, AppError> {
        Ok(self
            .reconcile(
                scope,
                &parse(content)?,
                chrono::DateTime::<chrono::Utc>::UNIX_EPOCH,
            )
            .await?
            .into_iter()
            .map(|plan| plan.item)
            .collect())
    }

    async fn apply(
        &self,
        tx: &mut MySqlTx,
        scope: &AccessScope,
        _package: &ConfigPackage,
        content: &Value,
        decisions: &Decisions,
        context: &mut ApplyContext,
    ) -> Result<(), AppError> {
        for plan in self.reconcile(scope, &parse(content)?, context.at).await? {
            if plan.item.action == PlanAction::Unchanged || !decisions.executes(&plan.item) {
                continue;
            }
            if plan.is_new {
                self.templates
                    .insert(
                        tx,
                        &NewEmailTemplate {
                            id: plan.template.id,
                            key: plan.template.key.clone(),
                            name: plan.template.name.clone(),
                            scope: TemplateScope::Organization,
                            owner_id: None,
                            record_type: plan.template.record_type.clone(),
                            subject: plan.template.subject.clone(),
                            body_html: plan.template.body_html.clone(),
                            created_by: Some(scope.user_id().clone()),
                        },
                        context.at,
                    )
                    .await?;
            } else {
                self.templates
                    .update(tx, &plan.template, scope.user_id(), context.at)
                    .await?;
            }
        }
        Ok(())
    }
}
