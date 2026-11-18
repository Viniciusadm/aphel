use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;
use uuid::Uuid;

use aphel_kernel::domain::UserId;
use aphel_kernel::errors::{AppError, CodedKind};
use aphel_kernel::records::RecordCtx;
use aphel_sdk::Runtime;
use aphel_sdk::platform::api::routes::v1::field_permissions::user_field_redaction;
use aphel_sdk::platform::application::public_events::{
    PublicEvent, PublicEventRenderer, PublicResource, SharedPublicEventRenderer,
    deleted_resource_data, find_public_event, payload_resource_data, previous_values,
    public_definition_for, public_record_type, public_resource_loader,
};
use aphel_sdk::platform::application::shared::{DomainEvent, RecordType};
use aphel_sdk::platform::database::Db;
use aphel_sdk::platform::domain::access::AccessScope;

#[derive(Clone)]
pub(crate) struct ApiPublicEventRenderer {
    state: Runtime,
}

impl ApiPublicEventRenderer {
    pub(crate) fn new(state: Runtime) -> Self {
        Self { state }
    }

    pub(crate) fn shared(pool: Db) -> SharedPublicEventRenderer {
        Arc::new(Self::new(Runtime::from(
            aphel_kernel::state::AppState::new(Some(pool)).with_mysql_platform(),
        )))
    }

    async fn resource(
        &self,
        resource: PublicResource,
        scope: &AccessScope,
        record_type: &RecordType,
        id: Uuid,
    ) -> Result<Value, AppError> {
        let state = &self.state;
        let registry = state.registry().await?;
        if let Ok(handler) = registry.handler(record_type.as_str()) {
            return handler
                .load(&RecordCtx::from_runtime(scope, state), id)
                .await?
                .ok_or_else(|| AppError::NotFound(format!("{} {id}", record_type.as_str())));
        }
        match public_resource_loader(resource) {
            Some(loader) => loader.load(&state.ctx(scope).await?, id).await,
            None => Ok(serde_json::json!({ "id": id.to_string() })),
        }
    }
}

fn is_not_found(error: &AppError) -> bool {
    matches!(
        error,
        AppError::NotFound(_)
            | AppError::Coded {
                kind: CodedKind::NotFound,
                ..
            }
    )
}

#[async_trait]
impl PublicEventRenderer for ApiPublicEventRenderer {
    async fn render(&self, event: &DomainEvent) -> Result<Option<PublicEvent>, AppError> {
        aphel_sdk::platform::application::public_events::refresh_public_events(&self.state.db()?)
            .await?;
        let Some(definition) = public_definition_for(event) else {
            return Ok(None);
        };
        let data = if definition.verbatim_payload {
            event.payload.clone()
        } else if !definition.resource.has_record() {
            payload_resource_data(event)
        } else if definition.deleted {
            deleted_resource_data(event)
        } else {
            let Some(record) = &event.record else {
                return Ok(None);
            };
            let scope = AccessScope::system();
            match self
                .resource(
                    definition.resource,
                    &scope,
                    &record.record_type,
                    record.record_id,
                )
                .await
            {
                Ok(data) => data,
                Err(error) if is_not_found(&error) => return Ok(None),
                Err(error) => return Err(error),
            }
        };
        Ok(Some(PublicEvent {
            id: event.id,
            event_type: event.event_type.clone(),
            actor: event.actor.clone(),
            occurred_at: event.occurred_at,
            data,
            previous: previous_values(&definition, &event.payload),
            context: public_context(definition.context, &event.payload),
        }))
    }

    async fn restrict(
        &self,
        event: &PublicEvent,
        viewer: Option<&UserId>,
    ) -> Result<PublicEvent, AppError> {
        let Some(definition) = find_public_event(&event.event_type) else {
            return Ok(event.clone());
        };
        if !definition.resource.has_record() {
            return Ok(event.clone());
        }
        let pool = self.state.db()?;
        let record_type = public_record_type(&event.event_type, &definition);
        let redaction = user_field_redaction(&pool, viewer, &record_type).await?;
        let mut restricted = event.clone();
        redaction.redact_value(&mut restricted.data);
        for (field, entity) in definition.resource.nested() {
            user_field_redaction(&pool, viewer, &RecordType::parse(*entity)?)
                .await?
                .redact_nested(&mut restricted.data, field);
        }
        if let Some(previous) = restricted.previous.as_mut() {
            redaction.redact_previous(previous);
            if previous.as_object().is_some_and(serde_json::Map::is_empty) {
                restricted.previous = None;
            }
        }
        Ok(restricted)
    }
}

fn public_context(keys: &[&str], payload: &serde_json::Value) -> Option<serde_json::Value> {
    let context: serde_json::Map<String, serde_json::Value> = keys
        .iter()
        .filter_map(|key| {
            payload
                .get(*key)
                .map(|value| ((*key).to_owned(), value.clone()))
        })
        .collect();
    (!context.is_empty()).then_some(serde_json::Value::Object(context))
}
