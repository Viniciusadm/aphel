use serde_json::{Value, json};
use uuid::Uuid;

use crate::application::{
    FORM_LAYOUT_RECORD_TYPE, FormLayout, FormLayoutEntry, FormLayoutRepository, LayoutContext,
    LayoutPorts, LayoutsTx, SaveLayoutCommand, StoredFormLayout, validate_layout,
    with_required_fields,
};
use aphel_kernel::application::access::Authorizer;
use aphel_kernel::application::audit::{DiffOptions, diff_values};
use aphel_kernel::application::fields::{EntityDefinition, FieldDefinitionRepository, FieldSet};
use aphel_kernel::application::query::ResolveContext;
use aphel_kernel::application::security::Clock;
use aphel_kernel::application::shared::{
    Actor, AuditEntry, AuditLog, DomainEvent, EventPublisher, FieldChange, RecordRef,
    TransactionManager,
};
use aphel_kernel::domain::access::{AccessScope, Permission};
use aphel_kernel::errors::AppError;

pub const LAYOUT_CONTEXT_NOT_FOUND: &str = "layout_context_not_found";
pub const FORM_LAYOUT_UPDATED: &str = "form_layout.updated";
pub const FORM_LAYOUT_RESET: &str = "form_layout.reset";

pub struct LayoutUseCases<P> {
    ports: P,
}

impl<P> LayoutUseCases<P>
where
    P: LayoutPorts,
{
    pub fn new(ports: P) -> Self {
        Self { ports }
    }

    pub async fn get_layout(
        &self,
        record_type: &str,
        context: &str,
    ) -> Result<FormLayoutEntry, AppError> {
        let definition = self.ports.registry().require(record_type)?;
        let context = parse_context(context)?;
        Ok(self
            .ports
            .layouts()
            .find(&definition.record_type, context)
            .await?
            .map(FormLayoutEntry::from)
            .unwrap_or_else(|| FormLayoutEntry::empty(definition.record_type.clone(), context)))
    }

    pub async fn save_layout(
        &self,
        scope: &AccessScope,
        record_type: &str,
        context: &str,
        command: SaveLayoutCommand,
    ) -> Result<FormLayoutEntry, AppError> {
        let definition = self.ports.registry().require(record_type)?;
        let context = parse_context(context)?;
        scope.require(&Permission::SETTINGS_MANAGE)?;
        let fields = self.field_set(definition).await?;
        let now = self.ports.clock().now();
        let resolve = ResolveContext {
            now,
            timezone: chrono_tz::UTC,
            user_id: scope.user_id().clone(),
        };
        let mut violations = validate_layout(&command.layout, &fields, context, &resolve);
        if !violations.is_empty() {
            violations.sort_by(|left, right| left.field.cmp(&right.field));
            return Err(AppError::invalid_fields("invalid layout", violations));
        }
        let layout = match context {
            LayoutContext::Create => with_required_fields(command.layout, &fields),
            _ => command.layout,
        };

        let current = self
            .ports
            .layouts()
            .find(&definition.record_type, context)
            .await?;
        let changes = layout_changes(current.as_ref().map(|stored| &stored.layout), Some(&layout))?;
        if changes.is_empty()
            && let Some(current) = current
        {
            return Ok(current.into());
        }
        let stored = StoredFormLayout {
            id: current
                .as_ref()
                .map_or_else(Uuid::now_v7, |stored| stored.id),
            record_type: definition.record_type.clone(),
            context,
            layout,
            updated_by: Some(scope.user_id().clone()),
            created_at: current.as_ref().map_or(now, |stored| stored.created_at),
            updated_at: now,
        };
        let mut tx = self.ports.transactions().begin().await?;
        self.ports.layouts().upsert(&mut tx, &stored).await?;
        self.record(
            &mut tx,
            scope,
            FORM_LAYOUT_UPDATED,
            &stored,
            changes,
            command.ip,
        )
        .await?;
        self.ports.transactions().commit(tx).await?;
        Ok(stored.into())
    }

    pub async fn reset_layout(
        &self,
        scope: &AccessScope,
        record_type: &str,
        context: &str,
        ip: Option<String>,
    ) -> Result<(), AppError> {
        let definition = self.ports.registry().require(record_type)?;
        let context = parse_context(context)?;
        scope.require(&Permission::SETTINGS_MANAGE)?;
        let Some(current) = self
            .ports
            .layouts()
            .find(&definition.record_type, context)
            .await?
        else {
            return Ok(());
        };
        let changes = layout_changes(Some(&current.layout), None)?;
        let mut tx = self.ports.transactions().begin().await?;
        self.ports
            .layouts()
            .delete(&mut tx, &definition.record_type, context)
            .await?;
        self.record(&mut tx, scope, FORM_LAYOUT_RESET, &current, changes, ip)
            .await?;
        self.ports.transactions().commit(tx).await
    }

    async fn field_set(&self, definition: &EntityDefinition) -> Result<FieldSet, AppError> {
        Ok(FieldSet::new(
            definition,
            self.ports
                .definitions()
                .list(&definition.record_type)
                .await?,
        ))
    }

    async fn record(
        &self,
        tx: &mut LayoutsTx<P>,
        scope: &AccessScope,
        action: &str,
        layout: &StoredFormLayout,
        changes: Vec<FieldChange>,
        ip: Option<String>,
    ) -> Result<(), AppError> {
        let actor = Actor::from_scope(scope);
        let now = self.ports.clock().now();
        let record = RecordRef::new(FORM_LAYOUT_RECORD_TYPE, layout.id);
        let event = DomainEvent::new(actor.clone(), action, now)
            .with_record(record.clone())
            .with_payload(json!({
                "record_type": layout.record_type.as_str(),
                "context": layout.context.as_str(),
            }));
        let entry = AuditEntry::new(actor, action, now)
            .with_record(record)
            .with_changes(changes)
            .with_ip(ip);
        self.ports.events().publish(tx, event).await?;
        self.ports.audit().record(tx, entry).await
    }
}

fn parse_context(value: &str) -> Result<LayoutContext, AppError> {
    LayoutContext::parse(value).ok_or_else(|| {
        AppError::not_found_code(
            LAYOUT_CONTEXT_NOT_FOUND,
            "layout context must be create, edit or view",
        )
    })
}

fn layout_changes(
    old: Option<&FormLayout>,
    new: Option<&FormLayout>,
) -> Result<Vec<FieldChange>, AppError> {
    let snapshot = |layout: Option<&FormLayout>| -> Result<Value, AppError> {
        match layout {
            Some(layout) => serde_json::to_value(layout)
                .map(|layout| json!({ "layout": layout }))
                .map_err(|error| AppError::Internal(format!("cannot serialize layout: {error}"))),
            None => Ok(Value::Null),
        }
    };
    diff_values(&snapshot(old)?, &snapshot(new)?, &DiffOptions::default())
}
