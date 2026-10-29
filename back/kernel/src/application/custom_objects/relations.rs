use serde_json::json;
use uuid::Uuid;

use crate::application::access::Authorizer;
use crate::application::custom_objects::{
    ChangeRecord, RecordLink, RecordLinkRepository, RecordSummary, RelationPorts, publish_change,
};
use crate::application::fields::{RecordRelation, RecordTypeDefinition};
use crate::application::files::RecordAccessChecker;
use crate::application::organization_admin::{ListWindow, Listing};
use crate::application::security::Clock;
use crate::application::shared::{FieldChange, RecordRef, RecordType, TransactionManager};
use crate::domain::access::AccessScope;
use crate::domain::custom_objects::{RelationSide, RelationshipDefinition, is_custom_record_type};
use crate::errors::AppError;

pub const RECORD_LINKED: &str = "record.linked";
pub const RECORD_UNLINKED: &str = "record.unlinked";

pub const RELATION_NOT_FOUND: &str = "relation_not_found";
pub const RELATION_LIMIT_REACHED: &str = "relation_limit_reached";
pub const RELATION_REQUIRED: &str = "relation_required";
pub const RECORD_HAS_RELATIONS: &str = "record_has_relations";
pub const LINKED_RECORD_NOT_FOUND: &str = "linked_record_not_found";
pub const RECORD_NOT_ACCESSIBLE: &str = "record_not_accessible";

pub fn relation_not_found() -> AppError {
    AppError::not_found_code(
        RELATION_NOT_FOUND,
        "relation was not found on this record type",
    )
}

pub fn record_has_relations(labels: &[String]) -> AppError {
    AppError::conflict_code(
        RECORD_HAS_RELATIONS,
        format!(
            "the record still has linked records that block its deletion: {}",
            labels.join(", ")
        ),
    )
}

fn record_not_accessible() -> AppError {
    AppError::forbidden_code(
        RECORD_NOT_ACCESSIBLE,
        "the record does not exist or you cannot access it",
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelationInfo {
    pub key: String,
    pub label: String,
    pub target_type: String,
    pub relationship_key: String,
    pub side: RelationSide,
    pub single: bool,
    pub required: bool,
    pub readable: bool,
    pub total: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedRelation {
    pub relationship: RelationshipDefinition,
    pub side: RelationSide,
}

impl ResolvedRelation {
    pub fn inverse(&self) -> bool {
        self.side == RelationSide::To
    }

    pub fn ends(&self, source_id: Uuid, target_id: Uuid) -> (Uuid, Uuid) {
        match self.side {
            RelationSide::From => (source_id, target_id),
            RelationSide::To => (target_id, source_id),
        }
    }
}

pub fn resolve_relation(
    relationships: &[RelationshipDefinition],
    relation: &RecordRelation,
) -> Option<ResolvedRelation> {
    let (relationship_id, inverse) = relation.relationship()?;
    let relationship = relationships
        .iter()
        .find(|relationship| relationship.id == relationship_id)?;
    Some(ResolvedRelation {
        relationship: relationship.clone(),
        side: if inverse {
            RelationSide::To
        } else {
            RelationSide::From
        },
    })
}

pub struct RelationUseCases<P> {
    ports: P,
}

impl<P> RelationUseCases<P>
where
    P: RelationPorts,
{
    pub fn new(ports: P) -> Self {
        Self { ports }
    }

    pub async fn relations(
        &self,
        scope: &AccessScope,
        record_type: &str,
        record_id: &str,
    ) -> Result<Vec<RelationInfo>, AppError> {
        let (definition, record) = self.readable_record(scope, record_type, record_id).await?;
        let mut relations = Vec::new();
        for relation in &definition.relations {
            let Some(resolved) = resolve_relation(self.ports.relationships(), relation) else {
                continue;
            };
            let target = self.ports.registry().require(relation.target.as_str())?;
            let visibility = scope.record_filter(&target.read_permission).ok();
            let total = match &visibility {
                Some(visibility) => Some(
                    self.ports
                        .links()
                        .linked_ids(
                            scope,
                            target,
                            visibility,
                            resolved.relationship.id,
                            resolved.inverse(),
                            record.record_id,
                            ListWindow {
                                offset: 0,
                                limit: 0,
                            },
                        )
                        .await?
                        .total,
                ),
                None => None,
            };
            relations.push(RelationInfo {
                key: relation.key.clone(),
                label: relation.label.clone(),
                target_type: relation.target.as_str().to_owned(),
                relationship_key: resolved.relationship.key.clone(),
                side: resolved.side,
                single: resolved.relationship.single(resolved.side),
                required: resolved.relationship.required && resolved.side == RelationSide::To,
                readable: visibility.is_some(),
                total,
            });
        }
        Ok(relations)
    }

    pub async fn related(
        &self,
        scope: &AccessScope,
        record_type: &str,
        record_id: &str,
        relation_key: &str,
        window: ListWindow,
    ) -> Result<(RecordType, Listing<RecordSummary>), AppError> {
        let (definition, record) = self.readable_record(scope, record_type, record_id).await?;
        let (relation, resolved) = self.relation(definition, relation_key)?;
        let target = self.ports.registry().require(relation.target.as_str())?;
        let visibility = scope.record_filter(&target.read_permission)?;
        let page = self
            .ports
            .links()
            .linked_ids(
                scope,
                target,
                &visibility,
                resolved.relationship.id,
                resolved.inverse(),
                record.record_id,
                window,
            )
            .await?;
        let titles = self.ports.links().titles(target, &page.items).await?;
        Ok((
            relation.target.clone(),
            Listing {
                items: page
                    .items
                    .into_iter()
                    .map(|id| RecordSummary {
                        title: titles.get(&id).cloned(),
                        record: RecordRef::new(relation.target.clone(), id),
                    })
                    .collect(),
                total: page.total,
            },
        ))
    }

    pub async fn link(
        &self,
        scope: &AccessScope,
        record_type: &str,
        record_id: &str,
        relation_key: &str,
        target_id: &str,
        ip: Option<String>,
    ) -> Result<bool, AppError> {
        let (record, relation, resolved, target) = self
            .changeable(scope, record_type, record_id, relation_key, target_id)
            .await?;
        let (from_id, to_id) = resolved.ends(record.record_id, target.record_id);
        let relationship = &resolved.relationship;
        let mut tx = self.ports.transactions().begin().await?;
        let links = self.ports.links();
        let from_taken = links
            .lock_links(&mut tx, relationship.id, false, from_id)
            .await?;
        let to_taken = links
            .lock_links(&mut tx, relationship.id, true, to_id)
            .await?;
        let single_from = relationship.single(RelationSide::From);
        let single_to = relationship.single(RelationSide::To);
        if (single_from && from_taken > 0) || (single_to && to_taken > 0) {
            return Err(AppError::conflict_code(
                RELATION_LIMIT_REACHED,
                format!(
                    "the {} relationship allows a single linked record on this side",
                    relationship.cardinality.as_str()
                ),
            ));
        }
        let now = self.ports.clock().now();
        let inserted = links
            .insert(
                &mut tx,
                &RecordLink {
                    relationship_id: relationship.id,
                    from_id,
                    to_id,
                    created_by: Some(scope.user_id().clone()),
                    created_at: now,
                },
            )
            .await?;
        if !inserted {
            return Ok(false);
        }
        self.publish(
            &mut tx,
            scope,
            &record,
            &relation,
            &resolved,
            &target,
            RECORD_LINKED,
            ip,
        )
        .await?;
        self.ports.transactions().commit(tx).await?;
        Ok(true)
    }

    pub async fn unlink(
        &self,
        scope: &AccessScope,
        record_type: &str,
        record_id: &str,
        relation_key: &str,
        target_id: &str,
        ip: Option<String>,
    ) -> Result<(), AppError> {
        let (record, relation, resolved, target) = self
            .changeable(scope, record_type, record_id, relation_key, target_id)
            .await?;
        let (from_id, to_id) = resolved.ends(record.record_id, target.record_id);
        let relationship = &resolved.relationship;
        let mut tx = self.ports.transactions().begin().await?;
        let links = self.ports.links();
        let remaining = links
            .lock_links(&mut tx, relationship.id, true, to_id)
            .await?;
        if relationship.required && is_custom_record_type(&relationship.to_type) && remaining <= 1 {
            return Err(AppError::conflict_code(
                RELATION_REQUIRED,
                format!(
                    "{} requires at least one linked record",
                    relationship.to_label
                ),
            ));
        }
        if !links
            .delete(&mut tx, relationship.id, from_id, to_id)
            .await?
        {
            return Ok(());
        }
        self.publish(
            &mut tx,
            scope,
            &record,
            &relation,
            &resolved,
            &target,
            RECORD_UNLINKED,
            ip,
        )
        .await?;
        self.ports.transactions().commit(tx).await
    }

    async fn readable_record(
        &self,
        scope: &AccessScope,
        record_type: &str,
        record_id: &str,
    ) -> Result<(&RecordTypeDefinition, RecordRef), AppError> {
        let definition = self.ports.registry().require(record_type.trim())?;
        let record_id = Uuid::parse_str(record_id.trim()).map_err(|_| record_not_accessible())?;
        let record = RecordRef::new(definition.record_type.clone(), record_id);
        scope.record_filter(&definition.read_permission)?;
        if !self.ports.access().can_read(scope, &record).await? {
            return Err(record_not_accessible());
        }
        Ok((definition, record))
    }

    fn relation(
        &self,
        definition: &RecordTypeDefinition,
        relation_key: &str,
    ) -> Result<(RecordRelation, ResolvedRelation), AppError> {
        let relation = definition
            .relation_by_key(relation_key.trim())
            .ok_or_else(relation_not_found)?;
        let resolved = resolve_relation(self.ports.relationships(), relation)
            .ok_or_else(relation_not_found)?;
        Ok((relation.clone(), resolved))
    }

    async fn changeable(
        &self,
        scope: &AccessScope,
        record_type: &str,
        record_id: &str,
        relation_key: &str,
        target_id: &str,
    ) -> Result<(RecordRef, RecordRelation, ResolvedRelation, RecordRef), AppError> {
        let (definition, record) = self.readable_record(scope, record_type, record_id).await?;
        let (relation, resolved) = self.relation(definition, relation_key)?;
        if !self.ports.access().can_update(scope, &record).await? {
            return Err(crate::records::permission_denied());
        }
        let target_id = Uuid::parse_str(target_id.trim()).map_err(|_| {
            AppError::not_found_code(LINKED_RECORD_NOT_FOUND, "linked record was not found")
        })?;
        let target = RecordRef::new(relation.target.clone(), target_id);
        if !self.ports.access().can_read(scope, &target).await? {
            return Err(AppError::not_found_code(
                LINKED_RECORD_NOT_FOUND,
                "linked record was not found",
            ));
        }
        Ok((record, relation, resolved, target))
    }

    #[allow(clippy::too_many_arguments)]
    async fn publish(
        &self,
        tx: &mut crate::application::custom_objects::RelationsTx<P>,
        scope: &AccessScope,
        record: &RecordRef,
        relation: &RecordRelation,
        resolved: &ResolvedRelation,
        target: &RecordRef,
        event_type: &str,
        ip: Option<String>,
    ) -> Result<(), AppError> {
        let target_value = json!({
            "record_type": target.record_type.as_str(),
            "record_id": target.record_id.to_string(),
        });
        let (old, new) = if event_type == RECORD_LINKED {
            (serde_json::Value::Null, target_value.clone())
        } else {
            (target_value.clone(), serde_json::Value::Null)
        };
        publish_change(
            self.ports.events(),
            self.ports.audit(),
            tx,
            scope,
            self.ports.clock().now(),
            ChangeRecord {
                record: record.clone(),
                event_type: event_type.to_owned(),
                payload: json!({
                    "relationship": resolved.relationship.key,
                    "relation": relation.key,
                    "side": resolved.side.as_str(),
                    "target": target_value,
                }),
                changes: vec![FieldChange::new(
                    format!("relations.{}", relation.key),
                    old,
                    new,
                )],
                ip,
            },
        )
        .await
    }
}
