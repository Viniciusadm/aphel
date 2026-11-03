use std::collections::HashMap;

use serde_json::Value;

use crate::incoming::gateway;
use crate::incoming::{
    ActionKind, ActionTarget, ContactAction, ContactMatch, MappableField, MappingExpression,
    WebhookAction,
};
use aphel_kernel::Ctx;
use aphel_kernel::domain::fields::FieldDefinition;
use aphel_kernel::errors::{AppError, FieldViolation};

pub(crate) const MAX_ACTIONS: usize = 10;
pub(crate) const MAX_MAPPED_FIELDS: usize = 60;

pub(crate) struct FieldCatalog {
    definitions: HashMap<ActionKind, Vec<FieldDefinition>>,
}

impl FieldCatalog {
    pub(crate) async fn load(ctx: &Ctx<'_>) -> Result<Self, AppError> {
        let mut definitions = HashMap::new();
        for kind in ActionKind::ALL {
            definitions.insert(
                kind,
                gateway::field_definitions(ctx, &kind.record_type()).await?,
            );
        }
        Ok(Self { definitions })
    }

    fn definitions(&self, kind: ActionKind) -> &[FieldDefinition] {
        self.definitions
            .get(&kind)
            .map(Vec::as_slice)
            .unwrap_or_default()
    }

    pub(crate) fn is_custom(&self, kind: ActionKind, key: &str) -> bool {
        self.definitions(kind)
            .iter()
            .any(|field| !field.is_system && field.is_active && field.key.as_str() == key)
    }

    pub(crate) fn is_unique_custom(&self, kind: ActionKind, key: &str) -> bool {
        self.definitions(kind).iter().any(|field| {
            !field.is_system && field.is_active && field.is_unique && field.key.as_str() == key
        })
    }

    pub(crate) fn is_system(kind: ActionKind, key: &str) -> bool {
        kind.system_fields().iter().any(|(name, _)| *name == key)
    }

    pub(crate) fn targets(&self) -> Vec<ActionTarget> {
        ActionKind::ALL
            .into_iter()
            .map(|kind| {
                let definitions = self.definitions(kind);
                let mut fields: Vec<MappableField> = kind
                    .system_fields()
                    .iter()
                    .map(|(key, required)| {
                        let definition = definitions
                            .iter()
                            .find(|field| field.is_system && field.key.as_str() == *key);
                        MappableField {
                            key: (*key).to_owned(),
                            label: definition
                                .map(|field| field.label.clone())
                                .unwrap_or_else(|| (*key).to_owned()),
                            field_type: definition
                                .map(|field| field.field_type.as_str())
                                .unwrap_or("text")
                                .to_owned(),
                            is_custom: false,
                            is_required: *required,
                            is_unique: definition.is_some_and(|field| field.is_unique),
                        }
                    })
                    .collect();
                fields.extend(
                    definitions
                        .iter()
                        .filter(|field| !field.is_system && field.is_active)
                        .map(|field| MappableField {
                            key: field.key.as_str().to_owned(),
                            label: field.label.clone(),
                            field_type: field.field_type.as_str().to_owned(),
                            is_custom: true,
                            is_required: field.is_required,
                            is_unique: field.is_unique,
                        }),
                );
                ActionTarget {
                    action_type: kind,
                    record_type: kind.record_type().as_str().to_owned(),
                    fields,
                }
            })
            .collect()
    }
}

pub(crate) async fn validate_actions(
    ctx: &Ctx<'_>,
    raw: Vec<Value>,
    violations: &mut Vec<FieldViolation>,
) -> Result<Vec<WebhookAction>, AppError> {
    if raw.len() > MAX_ACTIONS {
        violations.push(FieldViolation::new(
            "actions",
            format!("must have at most {MAX_ACTIONS} actions"),
        ));
        return Ok(Vec::new());
    }
    let catalog = FieldCatalog::load(ctx).await?;
    let mut actions = Vec::with_capacity(raw.len());
    for (index, value) in raw.into_iter().enumerate() {
        let path = format!("actions.{index}");
        let action = match serde_json::from_value::<WebhookAction>(value) {
            Ok(action) => action,
            Err(error) => {
                violations.push(FieldViolation::new(
                    path,
                    format!(
                        "invalid action ({error}); type must be upsert_contact, create_deal or create_activity"
                    ),
                ));
                continue;
            }
        };
        validate_fields(&catalog, &action, &path, violations);
        let distribution = match &action {
            WebhookAction::UpsertContact(contact) => contact.distribution_rule_id.as_deref(),
            WebhookAction::CreateDeal(deal) => deal.distribution_rule_id.as_deref(),
            WebhookAction::CreateActivity(_) => None,
        };
        if let Some(rule_id) = distribution
            && !gateway::distribution_rule_usable(ctx, rule_id, &action.kind().record_type())
                .await?
        {
            violations.push(FieldViolation::new(
                format!("{path}.distribution_rule_id"),
                "must be an active lead distribution rule for this record type",
            ));
        }
        match &action {
            WebhookAction::UpsertContact(contact) => {
                validate_contact_match(&catalog, contact, &path, violations);
            }
            WebhookAction::CreateDeal(deal) => {
                if !gateway::stage_in_pipeline(ctx, &deal.pipeline_id, &deal.stage_id).await? {
                    violations.push(FieldViolation::new(
                        format!("{path}.stage_id"),
                        "must be an active stage of the chosen pipeline",
                    ));
                }
            }
            WebhookAction::CreateActivity(activity) => {
                if !gateway::activity_type_active(ctx, &activity.activity_type_id).await? {
                    violations.push(FieldViolation::new(
                        format!("{path}.activity_type_id"),
                        "must be an active activity type",
                    ));
                }
            }
        }
        actions.push(action);
    }
    Ok(actions)
}

fn validate_contact_match(
    catalog: &FieldCatalog,
    contact: &ContactAction,
    path: &str,
    violations: &mut Vec<FieldViolation>,
) {
    let match_field = contact
        .match_field
        .as_deref()
        .map(str::trim)
        .filter(|key| !key.is_empty());
    match (contact.match_by, match_field) {
        (Some(ContactMatch::Field), None) => violations.push(FieldViolation::new(
            format!("{path}.match_field"),
            "is required when match_by is field",
        )),
        (Some(ContactMatch::Field), Some(key)) => {
            if !catalog.is_unique_custom(ActionKind::UpsertContact, key) {
                violations.push(FieldViolation::new(
                    format!("{path}.match_field"),
                    "must be an active unique custom field of people",
                ));
            } else if !contact.fields.contains_key(key) {
                violations.push(FieldViolation::new(
                    format!("{path}.match_field"),
                    "must also be mapped in fields",
                ));
            }
        }
        (_, Some(_)) => violations.push(FieldViolation::new(
            format!("{path}.match_field"),
            "is only allowed when match_by is field",
        )),
        (_, None) => {}
    }
}

fn validate_fields(
    catalog: &FieldCatalog,
    action: &WebhookAction,
    path: &str,
    violations: &mut Vec<FieldViolation>,
) {
    let kind = action.kind();
    let fields = action.fields();
    if fields.len() > MAX_MAPPED_FIELDS {
        violations.push(FieldViolation::new(
            format!("{path}.fields"),
            format!("must map at most {MAX_MAPPED_FIELDS} fields"),
        ));
    }
    for (key, expression) in fields {
        let field_path = format!("{path}.fields.{key}");
        if !FieldCatalog::is_system(kind, key) && !catalog.is_custom(kind, key) {
            violations.push(FieldViolation::new(
                field_path,
                format!("unknown field for {}", kind.as_str()),
            ));
            continue;
        }
        if let Err(message) = MappingExpression::parse(expression) {
            violations.push(FieldViolation::new(field_path, message));
        }
    }
    for (key, required) in kind.system_fields() {
        if *required && !fields.contains_key(*key) {
            violations.push(FieldViolation::new(
                format!("{path}.fields.{key}"),
                "is required for this action",
            ));
        }
    }
}
