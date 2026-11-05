use crate::incoming::{ActionKind, ActionTarget, MappableField, MappingExpression, WebhookAction};
use aphel_kernel::{
    Ctx,
    errors::{AppError, FieldViolation},
};
use aphel_sdk::platform::application::access::Authorizer;
use aphel_sdk::platform::application::fields::FieldSet;
use aphel_sdk::platform::application::query::RecordQueryPorts;
use serde_json::Value;
use std::collections::{HashMap, HashSet};

pub(crate) const MAX_ACTIONS: usize = 10;
pub(crate) const MAX_MAPPED_FIELDS: usize = 60;
pub(crate) struct FieldCatalog {
    pub(crate) fields: HashMap<String, FieldSet>,
    upsert: HashSet<String>,
}
impl FieldCatalog {
    pub(crate) async fn load(ctx: &Ctx<'_>) -> Result<Self, AppError> {
        let mut fields = HashMap::new();
        let mut upsert = HashSet::new();
        for definition in ctx.registry().definitions() {
            let key = definition.record_type.as_str();
            if (!definition.generic_writes
                && definition.handler.is_none()
                && definition.storage
                    != aphel_sdk::platform::application::fields::EntityStorage::Dynamic)
                || !aphel_sdk::records::enabled(ctx, &definition.record_type)
                || ctx.registry().handler(key).is_err()
                || !definition
                    .action_permission("create")
                    .is_some_and(|p| ctx.scope.require(&p).is_ok())
            {
                continue;
            }
            if ["read", "update"].into_iter().all(|action| {
                definition
                    .action_permission(action)
                    .is_some_and(|p| ctx.scope.require(&p).is_ok())
            }) {
                upsert.insert(key.to_owned());
            }
            let query = ctx.query();
            fields.insert(
                key.to_owned(),
                FieldSet::for_scope(query.definitions(), definition, ctx.scope).await?,
            );
        }
        Ok(Self { fields, upsert })
    }
    pub(crate) fn targets(&self) -> Vec<ActionTarget> {
        let mut targets = Vec::new();
        for (key, fields) in &self.fields {
            for kind in [ActionKind::CreateRecord, ActionKind::UpsertRecord] {
                if kind == ActionKind::UpsertRecord && !self.upsert.contains(key) {
                    continue;
                }
                targets.push(ActionTarget {
                    action_type: kind,
                    record_type: key.clone(),
                    fields: fields
                        .active()
                        .filter(|f| f.is_editable())
                        .map(|f| MappableField {
                            key: f.key().to_owned(),
                            label: f.definition.label.clone(),
                            field_type: f.definition.field_type.as_str().to_owned(),
                            is_custom: !f.is_system(),
                            is_required: f.definition.is_required,
                            is_unique: f.definition.is_unique,
                        })
                        .collect(),
                });
            }
        }
        targets.sort_by(|a, b| {
            (&a.record_type, a.action_type.as_str()).cmp(&(&b.record_type, b.action_type.as_str()))
        });
        targets
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
    let mut actions = Vec::new();
    for (index, value) in raw.into_iter().enumerate() {
        let path = format!("actions.{index}");
        let action: WebhookAction = match serde_json::from_value(value) {
            Ok(a) => a,
            Err(e) => {
                violations.push(FieldViolation::new(path, format!("invalid action: {e}")));
                continue;
            }
        };
        let Some(fields) = catalog.fields.get(&action.record_type) else {
            violations.push(FieldViolation::new(
                format!("{path}.record_type"),
                "must be an enabled writable entity",
            ));
            continue;
        };
        if action.fields.len() > MAX_MAPPED_FIELDS {
            violations.push(FieldViolation::new(
                format!("{path}.fields"),
                format!("must map at most {MAX_MAPPED_FIELDS} fields"),
            ));
        }
        for (key, expression) in &action.fields {
            if !fields.active().any(|f| f.key() == key && f.is_editable()) {
                violations.push(FieldViolation::new(
                    format!("{path}.fields.{key}"),
                    "must be an editable entity field",
                ));
            }
            match MappingExpression::parse(expression) {
                Err(message) => {
                    violations.push(FieldViolation::new(format!("{path}.fields.{key}"), message));
                }
                Ok(parsed) => {
                    for reference in parsed.paths() {
                        let message = match reference.root() {
                            Some("payload") => None,
                            Some("actions") => match reference.action_index() {
                                Some(earlier) if earlier < index => None,
                                _ => Some("must reference an earlier action as $.actions[n]"),
                            },
                            _ => Some("must start with $.payload or $.actions[n]"),
                        };
                        if let Some(message) = message {
                            violations
                                .push(FieldViolation::new(format!("{path}.fields.{key}"), message));
                        }
                    }
                }
            }
        }
        if action.kind() == ActionKind::UpsertRecord {
            for permission in ["read", "update"] {
                if !fields
                    .record_type
                    .action_permission(permission)
                    .is_some_and(|p| ctx.scope.require(&p).is_ok())
                {
                    violations.push(FieldViolation::new(
                        format!("{path}.record_type"),
                        format!("requires {permission} permission"),
                    ));
                }
            }
            if !action.match_field.as_deref().is_some_and(|key| {
                action.fields.contains_key(key)
                    && fields.active().any(|f| {
                        f.key() == key
                            && f.is_editable()
                            && f.filterable()
                            && f.definition.is_unique
                    })
            }) {
                violations.push(FieldViolation::new(
                    format!("{path}.match_field"),
                    "must be a mapped editable unique field",
                ));
            }
        } else if action.match_field.is_some() {
            violations.push(FieldViolation::new(
                format!("{path}.match_field"),
                "is only allowed for upsert_record",
            ));
        }
        if let Some(rule) = &action.assignment_rule_id
            && !super::gateway::distribution_rule_usable(ctx, rule, &action.record_type).await?
        {
            violations.push(FieldViolation::new(
                format!("{path}.assignment_rule_id"),
                "must be an active assignment rule for this entity",
            ));
        }
        actions.push(action);
    }
    Ok(actions)
}
