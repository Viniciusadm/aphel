use aphel_contract_crm::CrmRecordTypes;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use chrono::{DateTime, Duration, Utc};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::directory::ScoringDirectory;
use crate::domain::{
    RecordScore, ScoreBand, ScoringModel, band_changed_event, calculate_score, signal_hit,
};
use crate::store::{ModelStore, ScoreStore};
use aphel_kernel::Ctx;
use aphel_sdk::platform::application::shared::{
    Actor, AuditEntry, AuditLog, DomainEvent, EventPublisher, FieldChange, RecordRef, RecordType,
};
use aphel_sdk::platform::database::transaction::MySqlTx;
use aphel_sdk::platform::domain::access::AccessScope;

use aphel_kernel::errors::AppError;

pub(crate) const SCORING_BATCH: usize = 100;
const LIFECYCLE_ACTIONS: [&str; 5] = ["created", "updated", "restored", "deleted", "purged"];
const COMPANY_FIELD: &str = "company_id";

pub(crate) fn is_scored_record_type(record_type: &str) -> bool {
    aphel_sdk::platform::modules::app()
        .record_types()
        .require(record_type)
        .is_ok_and(|definition| {
            definition.capabilities.has_trait("scoring.subject") || definition.is_custom()
        })
}

pub(crate) fn is_lifecycle_event(event_type: &str) -> bool {
    event_type.rsplit_once('.').is_some_and(|(prefix, action)| {
        LIFECYCLE_ACTIONS.contains(&action)
            && aphel_sdk::platform::application::shared::RecordType::parse(prefix)
                .is_ok_and(|key| is_scored_record_type(key.as_str()))
    })
}

fn uuid_value(value: &Value) -> Option<Uuid> {
    value.as_str().and_then(|value| Uuid::parse_str(value).ok())
}

fn changed_companies(event: &DomainEvent) -> Vec<Uuid> {
    event
        .payload
        .get("changes")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|change| change.get("field").and_then(Value::as_str) == Some(COMPANY_FIELD))
        .flat_map(|change| {
            [change.get("old"), change.get("new")]
                .into_iter()
                .flatten()
                .filter_map(uuid_value)
                .collect::<Vec<_>>()
        })
        .collect()
}

fn same_result(previous: &RecordScore, next: &RecordScore) -> bool {
    previous.score == next.score
        && previous.band == next.band
        && previous.explanation == next.explanation
}

pub(crate) struct ScoringEngine<'a> {
    ctx: &'a Ctx<'a>,
}

impl<'a> ScoringEngine<'a> {
    pub(crate) fn new(ctx: &'a Ctx<'a>) -> Self {
        Self { ctx }
    }

    fn since(&self, model: &ScoringModel, now: DateTime<Utc>) -> Option<DateTime<Utc>> {
        model
            .longest_decay()
            .map(|days| now - Duration::days(i64::from(days)))
    }

    pub(crate) async fn handle_event(&self, event: &DomainEvent) -> Result<(), AppError> {
        let scope = AccessScope::system();
        let models = ModelStore::of(self.ctx).list(None).await?;
        if models.is_empty() {
            return Ok(());
        }
        let active: Vec<&ScoringModel> = models.iter().filter(|model| model.is_active).collect();
        let mut affected: BTreeMap<String, BTreeSet<Uuid>> = BTreeMap::new();
        let mut people = Vec::new();
        if let Some(hit) = signal_hit(event) {
            for record in &hit.records {
                ScoreStore::of(self.ctx)
                    .record_signal(
                        record,
                        hit.signal,
                        &hit.source_key,
                        Some(event.id),
                        hit.occurred_at,
                    )
                    .await?;
                affected
                    .entry(record.record_type.as_str().to_owned())
                    .or_default()
                    .insert(record.record_id);
                if record.record_type == RecordType::CONTACT {
                    people.push(record.record_id);
                }
            }
        } else if let Some(record) = &event.record
            && is_lifecycle_event(&event.event_type)
            && event
                .event_type
                .rsplit_once('.')
                .is_some_and(|(prefix, _)| {
                    RecordType::parse(prefix).is_ok_and(|key| key == record.record_type)
                })
        {
            let purged = event.event_type.ends_with(".purged");
            let deleted = purged || event.event_type.ends_with(".deleted");
            if purged {
                ScoreStore::of(self.ctx)
                    .delete_record_signals(record)
                    .await?;
            }
            if deleted {
                ScoreStore::of(self.ctx)
                    .delete_record_scores(record)
                    .await?;
            } else {
                affected
                    .entry(record.record_type.as_str().to_owned())
                    .or_default()
                    .insert(record.record_id);
            }
            if record.record_type == RecordType::CONTACT {
                let organizations = affected
                    .entry(RecordType::COMPANY.as_str().to_owned())
                    .or_default();
                organizations.extend(changed_companies(event));
                people.push(record.record_id);
            }
        }
        self.recalculate_affected(&scope, &active, affected, &people)
            .await
    }

    pub(crate) async fn recalculate_signal(&self, record: &RecordRef) -> Result<(), AppError> {
        let scope = AccessScope::system();
        let models = ModelStore::of(self.ctx).list(None).await?;
        let active: Vec<&ScoringModel> = models.iter().filter(|model| model.is_active).collect();
        if active.is_empty() {
            return Ok(());
        }
        let mut affected: BTreeMap<String, BTreeSet<Uuid>> = BTreeMap::new();
        affected
            .entry(record.record_type.as_str().to_owned())
            .or_default()
            .insert(record.record_id);
        let people = if record.record_type == RecordType::CONTACT {
            vec![record.record_id]
        } else {
            Vec::new()
        };
        self.recalculate_affected(&scope, &active, affected, &people)
            .await?;
        Ok(())
    }

    async fn recalculate_affected(
        &self,
        scope: &AccessScope,
        active: &[&ScoringModel],
        mut affected: BTreeMap<String, BTreeSet<Uuid>>,
        people: &[Uuid],
    ) -> Result<(), AppError> {
        let company_type = RecordType::COMPANY;
        let company = company_type.as_str();
        if !people.is_empty() && active.iter().any(|model| model.record_type == company) {
            let organizations = ScoringDirectory::of(self.ctx)
                .companies_of_contacts(people)
                .await?;
            affected
                .entry(company.to_owned())
                .or_default()
                .extend(organizations.into_values());
        }
        for model in active {
            let Some(ids) = affected.get(&model.record_type) else {
                continue;
            };
            if ids.is_empty() {
                continue;
            }
            let ids: Vec<Uuid> = ids.iter().copied().collect();
            self.recalculate_records(scope, model, &ids).await?;
        }
        Ok(())
    }

    pub(crate) async fn recalculate_records(
        &self,
        scope: &AccessScope,
        model: &ScoringModel,
        record_ids: &[Uuid],
    ) -> Result<(), AppError> {
        let definition = self.ctx.registry().require(&model.record_type)?;
        for chunk in record_ids.chunks(SCORING_BATCH) {
            let mut matches: HashMap<Uuid, HashSet<Uuid>> = HashMap::new();
            for (criterion, filter) in model.profile_criteria() {
                for id in ScoringDirectory::of(self.ctx)
                    .matching_ids(scope, definition, filter, Some(chunk))
                    .await?
                {
                    matches.entry(id).or_default().insert(criterion.id);
                }
            }
            self.score_chunk(scope, model, chunk, &matches).await?;
        }
        Ok(())
    }

    pub(crate) async fn recalculate_model(
        &self,
        scope: &AccessScope,
        model: &ScoringModel,
    ) -> Result<u64, AppError> {
        let definition = self.ctx.registry().require(&model.record_type)?;
        let now = self.ctx.now();
        let mut matches: HashMap<Uuid, HashSet<Uuid>> = HashMap::new();
        let mut ids: BTreeSet<Uuid> = BTreeSet::new();
        for (criterion, filter) in model.profile_criteria() {
            for id in ScoringDirectory::of(self.ctx)
                .matching_ids(scope, definition, filter, None)
                .await?
            {
                matches.entry(id).or_default().insert(criterion.id);
                ids.insert(id);
            }
        }
        let signals = model.signals();
        if !signals.is_empty() {
            ids.extend(
                ScoreStore::of(self.ctx)
                    .signal_record_ids(&model.record_type, &signals, self.since(model, now))
                    .await?,
            );
        }
        ids.extend(ScoreStore::of(self.ctx).scored_record_ids(model.id).await?);
        let ids: Vec<Uuid> = ids.into_iter().collect();
        for chunk in ids.chunks(SCORING_BATCH) {
            self.score_chunk(scope, model, chunk, &matches).await?;
        }
        ModelStore::of(self.ctx)
            .mark_recalculated(model.id, now)
            .await?;
        Ok(ids.len() as u64)
    }

    async fn score_chunk(
        &self,
        scope: &AccessScope,
        model: &ScoringModel,
        chunk: &[Uuid],
        matches: &HashMap<Uuid, HashSet<Uuid>>,
    ) -> Result<(), AppError> {
        let definition = self.ctx.registry().require(&model.record_type)?;
        let now = self.ctx.now();
        let existing: HashSet<Uuid> = ScoringDirectory::of(self.ctx)
            .existing_ids(scope, definition, chunk)
            .await?
            .into_iter()
            .collect();
        let signals = model.signals();
        let occurrences = if signals.is_empty() || existing.is_empty() {
            Vec::new()
        } else {
            let present: Vec<Uuid> = chunk
                .iter()
                .copied()
                .filter(|id| existing.contains(id))
                .collect();
            ScoreStore::of(self.ctx)
                .occurrences(
                    &model.record_type,
                    &present,
                    &signals,
                    self.since(model, now),
                )
                .await?
        };
        let empty = HashSet::new();
        let mut computed = Vec::new();
        let mut missing = Vec::new();
        for id in chunk {
            if !existing.contains(id) {
                missing.push(*id);
                continue;
            }
            let own: Vec<_> = occurrences
                .iter()
                .filter(|occurrence| occurrence.record_id == *id)
                .copied()
                .collect();
            computed.push(calculate_score(
                model,
                *id,
                matches.get(id).unwrap_or(&empty),
                &own,
                now,
            ));
        }
        self.persist(model, computed, missing).await
    }

    async fn persist(
        &self,
        model: &ScoringModel,
        computed: Vec<RecordScore>,
        missing: Vec<Uuid>,
    ) -> Result<(), AppError> {
        let ids: Vec<Uuid> = computed
            .iter()
            .map(|score| score.record_id)
            .chain(missing.iter().copied())
            .collect();
        if ids.is_empty() {
            return Ok(());
        }
        let mut tx = self.ctx.begin().await?;
        let previous = ScoreStore::of(self.ctx)
            .lock_scores(&mut tx, model.id, &ids)
            .await?;
        let mut removed: Vec<Uuid> = missing
            .into_iter()
            .filter(|id| previous.contains_key(id))
            .collect();
        for score in computed {
            let before = previous.get(&score.record_id);
            let blank = score.score == 0 && score.explanation.is_empty();
            if blank {
                if before.is_some() {
                    removed.push(score.record_id);
                }
            } else if before.is_none_or(|before| !same_result(before, &score)) {
                ScoreStore::of(self.ctx).save_score(&mut tx, &score).await?;
            }
            let old_band = before.map_or(ScoreBand::Cold, |before| before.band);
            if old_band != score.band {
                self.publish_band_change(&mut tx, model, before, &score)
                    .await?;
            }
        }
        if !removed.is_empty() {
            ScoreStore::of(self.ctx)
                .delete_scores(&mut tx, model.id, &removed)
                .await?;
        }
        Ok(tx.commit().await?)
    }

    async fn publish_band_change(
        &self,
        tx: &mut MySqlTx,
        model: &ScoringModel,
        before: Option<&RecordScore>,
        score: &RecordScore,
    ) -> Result<(), AppError> {
        let record_type = RecordType::parse(score.record_type.clone())?;
        let record = RecordRef::new(record_type, score.record_id);
        let old_band = before.map_or(ScoreBand::Cold, |before| before.band);
        let old_score = before.map_or(0, |before| before.score);
        let direction = if score.band.rank() > old_band.rank() {
            "up"
        } else {
            "down"
        };
        let now = self.ctx.now();
        let event_type = band_changed_event(&score.record_type);
        let payload = json!({
            "model": model.summary(),
            "model_key": model.key,
            "record_type": score.record_type,
            "record_id": score.record_id.to_string(),
            "old_band": old_band.as_str(),
            "new_band": score.band.as_str(),
            "band": score.band.as_str(),
            "direction": direction,
            "old_score": old_score,
            "score": score.score,
            "changes": [
                { "field": "score_band", "old": old_band.as_str(), "new": score.band.as_str() },
            ],
        });
        self.ctx
            .events()
            .publish(
                tx,
                DomainEvent::new(Actor::System, event_type, now)
                    .with_record(record.clone())
                    .with_payload(payload),
            )
            .await?;
        self.ctx
            .audit()
            .record(
                tx,
                AuditEntry::new(Actor::System, event_type, now)
                    .with_record(record)
                    .with_changes(vec![
                        FieldChange::new(
                            format!("score.{}", model.key),
                            json!(old_score),
                            json!(score.score),
                        ),
                        FieldChange::new(
                            format!("score_band.{}", model.key),
                            json!(old_band.as_str()),
                            json!(score.band.as_str()),
                        ),
                    ]),
            )
            .await
    }
}
