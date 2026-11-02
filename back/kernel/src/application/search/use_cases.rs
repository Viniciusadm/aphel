use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::time::Instant;

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::application::access::Authorizer;
use crate::application::fields::{
    CustomFieldValueRepository, EntityDefinition, FieldDefinitionRepository, FieldSet,
};
use crate::application::organization_admin::ListWindow;
use crate::application::query::{
    ArchivedFilter, FieldTarget, PlanCondition, PlanNode, PlanValue, Predicate, QueryPlan,
    RecordQueryExecutor,
};
use crate::application::search::document::{
    Display, IndexEntry, IndexedField, build_document, compose_display, indexed_fields,
    system_columns,
};
use crate::application::search::ports::{
    IndexCandidate, IndexHeader, IndexQuery, SearchIndexRepository, SearchPorts, SearchSource,
};
use crate::application::search::text::parse_terms;
use crate::application::security::Clock;
use crate::application::shared::RecordType;
use crate::domain::access::{AccessScope, RecordScopeFilter};
use crate::domain::fields::{FieldType, FieldValue};
use crate::errors::{AppError, FieldViolation};

pub const DEFAULT_LIMIT: u32 = 5;
pub const MAX_LIMIT: u32 = 20;
pub const MAX_CANDIDATES: u64 = 100;
pub const REINDEX_BATCH: u64 = 200;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SearchRequest {
    pub q: String,
    pub types: Option<String>,
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SearchHit {
    pub id: Uuid,
    pub record_type: RecordType,
    pub title: String,
    pub subtitle: Option<String>,
    pub score: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SearchGroup {
    pub record_type: RecordType,
    pub total: u64,
    pub items: Vec<SearchHit>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SearchResult {
    pub query: String,
    pub groups: Vec<SearchGroup>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReindexCursor {
    pub record_type: RecordType,
    pub after: Option<Uuid>,
    pub started_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ReindexSummary {
    pub indexed: u64,
    pub removed: u64,
}

impl ReindexSummary {
    fn add(&mut self, other: ReindexSummary) {
        self.indexed += other.indexed;
        self.removed += other.removed;
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReindexOutcome {
    Finished(ReindexSummary),
    Paused(ReindexSummary, ReindexCursor),
}

struct TypeIndex {
    fields: FieldSet,
    indexed: Vec<IndexedField>,
    columns: Vec<(String, String)>,
}

pub struct SearchUseCases<P> {
    ports: P,
}

impl<P> SearchUseCases<P>
where
    P: SearchPorts,
{
    pub fn new(ports: P) -> Self {
        Self { ports }
    }

    pub fn ports(&self) -> &P {
        &self.ports
    }

    pub async fn search(
        &self,
        scope: &AccessScope,
        request: SearchRequest,
    ) -> Result<SearchResult, AppError> {
        let terms = parse_terms(&request.q)?;
        let limit = request.limit.unwrap_or(DEFAULT_LIMIT);
        if !(1..=MAX_LIMIT).contains(&limit) {
            return Err(AppError::invalid_fields(
                "invalid search",
                vec![FieldViolation::new(
                    "limit",
                    format!("must be between 1 and {MAX_LIMIT}"),
                )],
            ));
        }
        let requested = self.requested_types(request.types.as_deref())?;
        let ordered_by_request = requested.is_some();
        let definitions: Vec<&EntityDefinition> = match requested {
            Some(types) => types
                .iter()
                .filter_map(|record_type| self.ports.registry().get(record_type))
                .collect(),
            None => self.ports.registry().definitions().collect(),
        };
        let mut groups: Vec<(f64, SearchGroup)> = Vec::new();
        if !terms.terms.is_empty() {
            for definition in definitions {
                let Ok(visibility) = scope.record_filter(&definition.read_permission) else {
                    continue;
                };
                if let Some(group) = self
                    .search_type(scope, definition, &visibility, &terms.terms, limit)
                    .await?
                {
                    let best = group.items.first().map_or(0.0, |hit| hit.score);
                    groups.push((best, group));
                }
            }
        }
        if !ordered_by_request {
            groups.sort_by(|left, right| right.0.partial_cmp(&left.0).unwrap_or(Ordering::Equal));
        }
        Ok(SearchResult {
            query: terms.query,
            groups: groups.into_iter().map(|(_, group)| group).collect(),
        })
    }

    fn requested_types(&self, types: Option<&str>) -> Result<Option<Vec<RecordType>>, AppError> {
        let Some(types) = types.map(str::trim).filter(|types| !types.is_empty()) else {
            return Ok(None);
        };
        let mut requested: Vec<RecordType> = Vec::new();
        for item in types
            .split(',')
            .map(str::trim)
            .filter(|item| !item.is_empty())
        {
            let record_type = RecordType::parse(item)
                .ok()
                .filter(|record_type| self.ports.registry().get(record_type).is_some())
                .ok_or_else(|| {
                    AppError::invalid_fields(
                        "invalid search",
                        vec![FieldViolation::new(
                            "types",
                            format!("unknown record type {item:?}"),
                        )],
                    )
                })?;
            if !requested.contains(&record_type) {
                requested.push(record_type);
            }
        }
        Ok(Some(requested))
    }

    async fn search_type(
        &self,
        scope: &AccessScope,
        definition: &EntityDefinition,
        visibility: &RecordScopeFilter,
        terms: &[String],
        limit: u32,
    ) -> Result<Option<SearchGroup>, AppError> {
        let fields = FieldSet::for_scope(self.ports.definitions(), definition, scope).await?;
        let mut visible_keys: Vec<String> = indexed_fields(&fields)
            .into_iter()
            .filter(|field| !fields.is_hidden(&field.key))
            .map(|field| field.key)
            .collect();
        visible_keys.extend(
            self.ports
                .contents()
                .visible_keys(scope, &definition.record_type),
        );
        if visible_keys.is_empty() {
            return Ok(None);
        }
        let candidates = self
            .ports
            .index()
            .candidates(
                scope,
                &IndexQuery {
                    definition,
                    visibility,
                    field_keys: &visible_keys,
                    terms,
                    limit: MAX_CANDIDATES,
                },
            )
            .await?;
        if candidates.is_empty() {
            return Ok(None);
        }
        let (allowed, total) = self
            .revalidate(scope, definition, visibility, &candidates)
            .await?;
        let page: Vec<IndexCandidate> = candidates
            .into_iter()
            .filter(|candidate| allowed.contains(&candidate.record_id))
            .take(limit as usize)
            .collect();
        if page.is_empty() {
            return Ok(None);
        }
        let items = self.hits(definition, &fields, &visible_keys, page).await?;
        Ok(Some(SearchGroup {
            record_type: definition.record_type.clone(),
            total,
            items,
        }))
    }

    async fn revalidate(
        &self,
        scope: &AccessScope,
        definition: &EntityDefinition,
        visibility: &RecordScopeFilter,
        candidates: &[IndexCandidate],
    ) -> Result<(HashSet<Uuid>, u64), AppError> {
        let plan = QueryPlan {
            record_type: definition.clone(),
            visibility: visibility.clone(),
            filter: Some(PlanNode::Condition(PlanCondition {
                target: FieldTarget::System {
                    column: "id".to_owned(),
                },
                field_type: FieldType::Record,
                predicate: Predicate::In(
                    candidates
                        .iter()
                        .map(|candidate| PlanValue::Text(candidate.record_id.to_string()))
                        .collect(),
                ),
                negated: false,
            })),
            search: None,
            sort: Vec::new(),
            archived: ArchivedFilter::Exclude,
            window: ListWindow {
                offset: 0,
                limit: candidates.len() as u64,
            },
        };
        let plan = self.ports.restrict(scope, plan).await?;
        let listing = self.ports.executor().execute(scope, &plan).await?;
        Ok((listing.items.into_iter().collect(), listing.total))
    }

    async fn hits(
        &self,
        definition: &EntityDefinition,
        fields: &FieldSet,
        visible_keys: &[String],
        page: Vec<IndexCandidate>,
    ) -> Result<Vec<SearchHit>, AppError> {
        let ids: Vec<Uuid> = page.iter().map(|candidate| candidate.record_id).collect();
        let record_type = &definition.record_type;
        let headers: HashMap<Uuid, IndexHeader> = self
            .ports
            .index()
            .headers(record_type, &ids)
            .await?
            .into_iter()
            .map(|header| (header.record_id, header))
            .collect();
        let exposes_hidden = |key: Option<&String>| key.is_some_and(|key| fields.is_hidden(key));
        let restricted: Vec<Uuid> = ids
            .iter()
            .copied()
            .filter(|id| {
                headers.get(id).is_none_or(|header| {
                    header.title_field.is_none()
                        || exposes_hidden(header.title_field.as_ref())
                        || exposes_hidden(header.subtitle_field.as_ref())
                })
            })
            .collect();
        let mut recomputed: HashMap<Uuid, Display> = HashMap::new();
        if !restricted.is_empty() {
            let mut entries: HashMap<Uuid, Vec<IndexEntry>> = HashMap::new();
            for (record_id, entry) in self
                .ports
                .index()
                .entries(record_type, &restricted, visible_keys)
                .await?
            {
                entries.entry(record_id).or_default().push(entry);
            }
            for (record_id, mut list) in entries {
                list.sort_by_key(|entry| entry.position);
                recomputed.insert(record_id, compose_display(&list));
            }
        }
        Ok(page
            .into_iter()
            .map(|candidate| {
                let id = candidate.record_id;
                let (title, subtitle) = match recomputed.remove(&id) {
                    Some(display) => (display.title, display.subtitle),
                    None if restricted.contains(&id) => (None, None),
                    None => headers
                        .get(&id)
                        .map(|header| (Some(header.title.clone()), header.subtitle.clone()))
                        .unwrap_or_default(),
                };
                SearchHit {
                    id,
                    record_type: record_type.clone(),
                    title: title
                        .filter(|title| !title.is_empty())
                        .unwrap_or_else(|| id.to_string()),
                    subtitle,
                    score: candidate.score,
                }
            })
            .collect())
    }

    async fn type_index(&self, definition: &EntityDefinition) -> Result<TypeIndex, AppError> {
        let fields = FieldSet::new(
            definition,
            self.ports
                .definitions()
                .list(&definition.record_type)
                .await?,
        );
        let indexed = indexed_fields(&fields);
        let columns = system_columns(&indexed);
        Ok(TypeIndex {
            fields,
            indexed,
            columns,
        })
    }

    pub async fn reindex_records(
        &self,
        record_type: &RecordType,
        record_ids: &[Uuid],
    ) -> Result<ReindexSummary, AppError> {
        let Some(definition) = self.ports.registry().get(record_type) else {
            return Ok(ReindexSummary::default());
        };
        if record_ids.is_empty() {
            return Ok(ReindexSummary::default());
        }
        let index = self.type_index(definition).await?;
        let rows = self
            .ports
            .source()
            .load(definition, &index.columns, record_ids)
            .await?;
        let found: HashSet<Uuid> = rows.iter().map(|row| row.id).collect();
        let missing: Vec<Uuid> = record_ids
            .iter()
            .copied()
            .filter(|id| !found.contains(id))
            .collect();
        let mut summary = self.apply_rows(definition, &index, rows).await?;
        if !missing.is_empty() {
            self.ports.index().remove(record_type, &missing).await?;
            summary.removed += missing.len() as u64;
        }
        Ok(summary)
    }

    async fn apply_rows(
        &self,
        definition: &EntityDefinition,
        index: &TypeIndex,
        rows: Vec<crate::application::search::document::SourceRow>,
    ) -> Result<ReindexSummary, AppError> {
        let mut summary = ReindexSummary::default();
        let live: Vec<Uuid> = rows
            .iter()
            .filter(|row| !row.removed)
            .map(|row| row.id)
            .collect();
        let custom = self.custom_values(index, &live).await?;
        let contents = self
            .ports
            .contents()
            .load(&definition.record_type, &live)
            .await?;
        let now = self.ports.clock().now();
        let mut removed: Vec<Uuid> = Vec::new();
        for row in rows {
            if row.removed {
                removed.push(row.id);
                continue;
            }
            let empty = HashMap::new();
            let document = build_document(
                definition,
                &index.indexed,
                &row,
                custom.get(&row.id).unwrap_or(&empty),
                contents.get(&row.id).map(Vec::as_slice).unwrap_or_default(),
            );
            if document.entries.is_empty() {
                removed.push(row.id);
                continue;
            }
            self.ports.index().upsert(&document, now).await?;
            summary.indexed += 1;
        }
        if !removed.is_empty() {
            self.ports
                .index()
                .remove(&definition.record_type, &removed)
                .await?;
            summary.removed += removed.len() as u64;
        }
        Ok(summary)
    }

    async fn custom_values(
        &self,
        index: &TypeIndex,
        record_ids: &[Uuid],
    ) -> Result<HashMap<Uuid, HashMap<Uuid, String>>, AppError> {
        let custom_ids: HashSet<Uuid> = index
            .indexed
            .iter()
            .filter_map(|field| field.field_id)
            .collect();
        let definitions: Vec<_> = index
            .fields
            .all()
            .iter()
            .filter(|field| custom_ids.contains(&field.definition.id))
            .map(|field| field.definition.clone())
            .collect();
        let mut result: HashMap<Uuid, HashMap<Uuid, String>> = HashMap::new();
        if definitions.is_empty() || record_ids.is_empty() {
            return Ok(result);
        }
        let stored = self
            .ports
            .values()
            .load(
                &index.fields.record_type.record_type,
                &definitions,
                record_ids,
            )
            .await?;
        for value in stored {
            let text = match value.value {
                FieldValue::Text(text) | FieldValue::LongText(text) => text,
                _ => continue,
            };
            result
                .entry(value.record_id)
                .or_default()
                .insert(value.field_id, text);
        }
        Ok(result)
    }

    pub async fn reindex_type(
        &self,
        cursor: ReindexCursor,
        deadline: Option<Instant>,
    ) -> Result<ReindexOutcome, AppError> {
        let mut summary = ReindexSummary::default();
        let Some(definition) = self.ports.registry().get(&cursor.record_type) else {
            return Ok(ReindexOutcome::Finished(summary));
        };
        let index = self.type_index(definition).await?;
        let mut after = cursor.after;
        loop {
            let rows = self
                .ports
                .source()
                .scan(definition, &index.columns, after, REINDEX_BATCH)
                .await?;
            let Some(last) = rows.last().map(|row| row.id) else {
                break;
            };
            let full = rows.len() as u64 == REINDEX_BATCH;
            summary.add(self.apply_rows(definition, &index, rows).await?);
            after = Some(last);
            if !full {
                break;
            }
            if deadline.is_some_and(|deadline| Instant::now() >= deadline) {
                return Ok(ReindexOutcome::Paused(
                    summary,
                    ReindexCursor {
                        record_type: cursor.record_type,
                        after,
                        started_at: cursor.started_at,
                    },
                ));
            }
        }
        summary.removed += self
            .ports
            .index()
            .remove_stale(&cursor.record_type, cursor.started_at)
            .await?;
        Ok(ReindexOutcome::Finished(summary))
    }

    pub async fn reindex_organization(
        &self,
        from: Option<ReindexCursor>,
        only: Option<&RecordType>,
        deadline: Option<Instant>,
    ) -> Result<ReindexOutcome, AppError> {
        let started_at = from
            .as_ref()
            .map_or_else(|| self.ports.clock().now(), |cursor| cursor.started_at);
        let types: Vec<RecordType> = self
            .ports
            .registry()
            .definitions()
            .map(|definition| definition.record_type.clone())
            .filter(|record_type| only.is_none_or(|only| only == record_type))
            .collect();
        let start = from
            .as_ref()
            .and_then(|cursor| {
                types
                    .iter()
                    .position(|record_type| *record_type == cursor.record_type)
            })
            .unwrap_or(0);
        let mut summary = ReindexSummary::default();
        for (position, record_type) in types.iter().enumerate().skip(start) {
            let after = from
                .as_ref()
                .filter(|_| position == start)
                .and_then(|cursor| cursor.after);
            let cursor = ReindexCursor {
                record_type: record_type.clone(),
                after,
                started_at,
            };
            match self.reindex_type(cursor, deadline).await? {
                ReindexOutcome::Finished(partial) => summary.add(partial),
                ReindexOutcome::Paused(partial, cursor) => {
                    summary.add(partial);
                    return Ok(ReindexOutcome::Paused(summary, cursor));
                }
            }
        }
        Ok(ReindexOutcome::Finished(summary))
    }
}
