mod merge;

use std::collections::{BTreeMap, BTreeSet};

use serde_json::json;
use uuid::Uuid;

use crate::domain::{CandidateStatus, DuplicateRule};
use crate::store::{DuplicateStore, MergeStore};
use crate::{
    CANDIDATE_FLUSH_SIZE, CANDIDATE_RESOLVED, CLEANUP_BATCH_SIZE, CandidateQuery,
    CandidateResolution, CandidateView, CleanupSummary, DUPLICATE_DISMISSED,
    DUPLICATE_RULES_UPDATED, DUPLICATES_SCAN_JOB, DuplicateMatch, DuplicateProbeInput,
    DuplicateType, MAX_MATCHES, NewCandidate, PairWindow, RecordFingerprint, RuleSetting,
    SCAN_BATCH_SIZE, ScanSummary, candidate_not_found, duplicate_type, duplicate_types,
    enabled_rules, matched_rules, new_candidate, parse_duplicate_type, probe_fingerprint,
    rule_settings, scan_passes,
};
use aphel_kernel::Ctx;
use aphel_kernel::application::access::Authorizer;
use aphel_kernel::application::fields::{CustomFieldPorts, FieldSet, ReferenceChecker};
use aphel_kernel::application::organization_admin::{ListWindow, Listing};
use aphel_kernel::application::shared::{
    Actor, AuditEntry, AuditLog, DomainEvent, EventPublisher, FieldChange, Job, JobQueue,
    RecordRef, RecordType,
};
use aphel_kernel::domain::access::{Permission, RecordScopeFilter};
use aphel_kernel::errors::{AppError, FieldViolation};

pub(crate) use merge::merge;

#[derive(Debug, Clone)]
pub(crate) struct DuplicateTypeInfo {
    pub(crate) kind: DuplicateType,
    pub(crate) rules: Vec<RuleSetting>,
    pub(crate) merge_fields: Vec<String>,
}

pub(crate) async fn types(ctx: &Ctx<'_>) -> Result<Vec<DuplicateTypeInfo>, AppError> {
    let store = DuplicateStore::of(ctx);
    let mut found = Vec::new();
    for kind in duplicate_types(ctx.registry()) {
        if ctx.scope.require(&kind.read).is_err() {
            continue;
        }
        let stored = store.rule_settings(kind.as_str()).await?;
        let fields = field_set(ctx, &kind).await?;
        let merge_fields = merge::system_merge_keys(&kind)
            .into_iter()
            .filter(|key| !fields.is_hidden(key))
            .filter(|key| fields.find(key).is_none_or(|field| field.is_editable()))
            .collect();
        found.push(DuplicateTypeInfo {
            rules: rule_settings(&kind, &stored),
            merge_fields,
            kind,
        });
    }
    Ok(found)
}

pub(crate) async fn field_set(ctx: &Ctx<'_>, kind: &DuplicateType) -> Result<FieldSet, AppError> {
    FieldSet::for_scope(ctx.fields().definitions(), &kind.definition, ctx.scope).await
}

pub(crate) async fn subtitle_columns(
    ctx: &Ctx<'_>,
    kind: &DuplicateType,
) -> Result<Vec<String>, AppError> {
    let fields = field_set(ctx, kind).await?;
    Ok(kind
        .summary_columns()
        .into_iter()
        .filter(|column| {
            kind.field_for_column(column)
                .is_some_and(|field| fields.find_active(field.key.as_str()).is_some())
        })
        .collect())
}

pub(crate) async fn list(
    ctx: &Ctx<'_>,
    query: CandidateQuery,
    window: ListWindow,
) -> Result<Listing<CandidateView>, AppError> {
    let scope = ctx.scope;
    let mut violations = Vec::new();
    let record_type = duplicate_type(
        ctx.registry(),
        query.record_type.as_deref().unwrap_or("people.person"),
    )
    .or_else(|| {
        violations.push(FieldViolation::new(
            "record_type",
            crate::DUPLICATE_RECORD_TYPE_MESSAGE,
        ));
        None
    });
    let status = match query.status.as_deref() {
        None => Some(CandidateStatus::Pending),
        Some(value) => CandidateStatus::parse(value).or_else(|| {
            violations.push(FieldViolation::new(
                "status",
                "must be pending, dismissed or merged",
            ));
            None
        }),
    };
    let (Some(record_type), Some(status)) = (record_type, status) else {
        return Err(AppError::invalid_fields(
            "invalid duplicate query",
            violations,
        ));
    };
    let visibility = scope.record_filter(&record_type.read)?;
    let subtitles = subtitle_columns(ctx, &record_type).await?;
    DuplicateStore::of(ctx)
        .list_candidates(scope, &record_type, status, &visibility, &subtitles, window)
        .await
}

pub(crate) async fn get(ctx: &Ctx<'_>, candidate_id: &str) -> Result<CandidateView, AppError> {
    get_with_kind(ctx, candidate_id).await.map(|(_, view)| view)
}

async fn get_with_kind(
    ctx: &Ctx<'_>,
    candidate_id: &str,
) -> Result<(DuplicateType, CandidateView), AppError> {
    let scope = ctx.scope;
    let id = Uuid::parse_str(candidate_id.trim()).map_err(|_| candidate_not_found())?;
    let store = DuplicateStore::of(ctx);
    let kind = store
        .candidate_type(id)
        .await?
        .and_then(|record_type| duplicate_type(ctx.registry(), &record_type))
        .ok_or_else(candidate_not_found)?;
    let visibility = scope.record_filter(&kind.read)?;
    let subtitles = subtitle_columns(ctx, &kind).await?;
    let candidate = store
        .find_candidate(&kind, id, &subtitles)
        .await?
        .ok_or_else(candidate_not_found)?;
    let visible = store
        .visible_records(
            scope,
            &kind,
            &visibility,
            &[candidate.record.id, candidate.duplicate.id],
            candidate.status == CandidateStatus::Merged,
        )
        .await?;
    if visible.len() < 2 {
        return Err(candidate_not_found());
    }
    Ok((kind, candidate))
}

pub(crate) async fn dismiss(
    ctx: &Ctx<'_>,
    candidate_id: &str,
    ip: Option<String>,
) -> Result<CandidateView, AppError> {
    let scope = ctx.scope;
    let (kind, candidate) = get_with_kind(ctx, candidate_id).await?;
    scope.require(&kind.update)?;
    if candidate.status != CandidateStatus::Pending {
        return Err(AppError::conflict_code(
            CANDIDATE_RESOLVED,
            "the duplicate candidate was already resolved",
        ));
    }
    let now = ctx.now();
    let record = RecordRef::new(kind.record_type().clone(), candidate.record.id);
    let actor = Actor::from_scope(scope);
    let mut tx = ctx.begin().await?;
    let dismissed = DuplicateStore::of(ctx)
        .set_candidate_status(
            &mut tx,
            candidate.id,
            CandidateStatus::Dismissed,
            &CandidateResolution {
                resolved_by: scope.user_id().clone(),
                resolved_at: now,
            },
        )
        .await?;
    if !dismissed {
        return Err(AppError::conflict_code(
            CANDIDATE_RESOLVED,
            "the duplicate candidate was already resolved",
        ));
    }
    ctx.events()
        .publish(
            &mut tx,
            DomainEvent::new(actor.clone(), DUPLICATE_DISMISSED, now)
                .with_record(record.clone())
                .with_payload(json!({
                    "candidate_id": candidate.id.to_string(),
                    "record_id": candidate.record.id.to_string(),
                    "duplicate_id": candidate.duplicate.id.to_string(),
                })),
        )
        .await?;
    ctx.audit()
        .record(
            &mut tx,
            AuditEntry::new(actor, DUPLICATE_DISMISSED, now)
                .with_record(record)
                .with_changes(vec![FieldChange::new(
                    "status",
                    json!(CandidateStatus::Pending.as_str()),
                    json!(CandidateStatus::Dismissed.as_str()),
                )])
                .with_ip(ip),
        )
        .await?;
    tx.commit().await?;
    Ok(CandidateView {
        status: CandidateStatus::Dismissed,
        resolved_at: Some(now),
        ..candidate
    })
}

pub(crate) async fn rules(ctx: &Ctx<'_>, record_type: &str) -> Result<Vec<RuleSetting>, AppError> {
    let kind = parse_duplicate_type(ctx.registry(), record_type)?;
    ctx.scope.require(&kind.read)?;
    let stored = DuplicateStore::of(ctx).rule_settings(kind.as_str()).await?;
    Ok(rule_settings(&kind, &stored))
}

pub(crate) async fn save_rules(
    ctx: &Ctx<'_>,
    record_type: &str,
    requested: BTreeMap<String, bool>,
    ip: Option<String>,
) -> Result<Vec<RuleSetting>, AppError> {
    let scope = ctx.scope;
    scope.require(&Permission::SETTINGS_MANAGE)?;
    let kind = parse_duplicate_type(ctx.registry(), record_type)?;
    let record_type = kind.as_str();
    let mut violations = Vec::new();
    let mut updates = Vec::new();
    for (key, enabled) in requested {
        match DuplicateRule::parse(&key).filter(|rule| kind.supports(*rule)) {
            Some(rule) => updates.push(RuleSetting { rule, enabled }),
            None => violations.push(FieldViolation::new(
                format!("rules.{key}"),
                "is not a duplicate rule of this record type",
            )),
        }
    }
    if !violations.is_empty() {
        return Err(AppError::invalid_fields(
            "invalid duplicate rules",
            violations,
        ));
    }
    let store = DuplicateStore::of(ctx);
    let current = rule_settings(&kind, &store.rule_settings(record_type).await?);
    let changes: Vec<FieldChange> = updates
        .iter()
        .filter_map(|update| {
            let old = current
                .iter()
                .find(|setting| setting.rule == update.rule)
                .is_none_or(|setting| setting.enabled);
            (old != update.enabled).then(|| {
                FieldChange::new(
                    format!("{}.{}", record_type, update.rule.as_str()),
                    json!(old),
                    json!(update.enabled),
                )
            })
        })
        .collect();
    if changes.is_empty() {
        return Ok(current);
    }
    let now = ctx.now();
    let actor = Actor::from_scope(scope);
    let record = RecordRef::organization();
    let mut tx = ctx.begin().await?;
    for update in &updates {
        store.save_rule(&mut tx, record_type, update).await?;
    }
    ctx.events()
        .publish(
            &mut tx,
            DomainEvent::new(actor.clone(), DUPLICATE_RULES_UPDATED, now)
                .with_record(record.clone())
                .with_payload(json!({
                    "record_type": record_type,
                    "rules": updates
                        .iter()
                        .map(|update| json!({ "rule": update.rule.as_str(), "enabled": update.enabled }))
                        .collect::<Vec<_>>(),
                })),
        )
        .await?;
    ctx.audit()
        .record(
            &mut tx,
            AuditEntry::new(actor, DUPLICATE_RULES_UPDATED, now)
                .with_record(record)
                .with_changes(changes)
                .with_ip(ip),
        )
        .await?;
    tx.commit().await?;
    rules(ctx, record_type).await
}

pub(crate) async fn check(
    ctx: &Ctx<'_>,
    input: DuplicateProbeInput,
) -> Result<Vec<DuplicateMatch>, AppError> {
    let scope = ctx.scope;
    let kind = parse_duplicate_type(ctx.registry(), &input.record_type)?;
    if scope.require(&kind.create).is_err() {
        scope.require(&kind.update)?;
    }
    let Ok(visibility) = scope.record_filter(&kind.read) else {
        return Ok(Vec::new());
    };
    let probe = probe_fingerprint(&kind, &input);
    let mut matches = matches_for(ctx, &kind, &probe, Some(&visibility)).await?;
    matches.truncate(MAX_MATCHES);
    Ok(matches)
}

pub(crate) async fn detect_for_record(
    ctx: &Ctx<'_>,
    record_type: &RecordType,
    record_id: Uuid,
) -> Result<Vec<DuplicateMatch>, AppError> {
    let Some(kind) = duplicate_type(ctx.registry(), record_type.as_str()) else {
        return Ok(Vec::new());
    };
    let store = DuplicateStore::of(ctx);
    let Some(probe) = store
        .fingerprints_by_ids(&kind, &[record_id])
        .await?
        .into_iter()
        .next()
    else {
        return Ok(Vec::new());
    };
    let matches = matches_for(ctx, &kind, &probe, None).await?;
    if matches.is_empty() {
        return Ok(matches);
    }
    let now = ctx.now();
    let candidates: Vec<NewCandidate> = matches
        .iter()
        .take(MAX_MATCHES)
        .map(|found| new_candidate(record_id, found.record_id, found.rules.clone(), now))
        .collect();
    let mut tx = ctx.begin().await?;
    store
        .upsert_candidates(&mut tx, kind.as_str(), &candidates)
        .await?;
    tx.commit().await?;
    let mut visible = visible_matches(ctx, &kind, matches).await?;
    visible.truncate(MAX_MATCHES);
    Ok(visible)
}

pub(crate) async fn request_scan(ctx: &Ctx<'_>) -> Result<(), AppError> {
    ctx.scope.require(&Permission::SETTINGS_MANAGE)?;
    ctx.jobs()
        .enqueue_now(
            Job::new(DUPLICATES_SCAN_JOB, json!({}))
                .with_max_attempts(3)
                .with_unique_key(DUPLICATES_SCAN_JOB.to_owned()),
        )
        .await?;
    Ok(())
}

pub(crate) async fn redirect(
    ctx: &Ctx<'_>,
    record_type: &RecordType,
    record_id: &str,
) -> Result<Option<Uuid>, AppError> {
    let Some(kind) = duplicate_type(ctx.registry(), record_type.as_str()) else {
        return Ok(None);
    };
    ctx.scope.require(&kind.read)?;
    let Ok(id) = Uuid::parse_str(record_id.trim()) else {
        return Ok(None);
    };
    MergeStore::of(ctx).merged_into(kind.as_str(), id).await
}

pub(crate) async fn scan_all(ctx: &Ctx<'_>) -> Result<ScanSummary, AppError> {
    let mut summary = ScanSummary::default();
    for kind in duplicate_types(ctx.registry()) {
        let found = scan_type(ctx, &kind).await?;
        summary.records += found.records;
        summary.candidates += found.candidates;
    }
    Ok(summary)
}

async fn flush(
    ctx: &Ctx<'_>,
    store: &DuplicateStore,
    kind: &DuplicateType,
    pending: &mut Vec<NewCandidate>,
) -> Result<(), AppError> {
    if pending.is_empty() {
        return Ok(());
    }
    let mut tx = ctx.begin().await?;
    store
        .upsert_candidates(&mut tx, kind.as_str(), pending)
        .await?;
    tx.commit().await?;
    pending.clear();
    Ok(())
}

pub(crate) async fn scan_type(
    ctx: &Ctx<'_>,
    kind: &DuplicateType,
) -> Result<ScanSummary, AppError> {
    let store = DuplicateStore::of(ctx);
    let stored = store.rule_settings(kind.as_str()).await?;
    let rules = enabled_rules(kind, &stored);
    if rules.is_empty() {
        return Ok(ScanSummary::default());
    }
    let mut summary = ScanSummary {
        records: store.count_records(kind).await? as usize,
        candidates: 0,
    };
    let now = ctx.now();
    let mut seen: BTreeSet<(Uuid, Uuid)> = BTreeSet::new();
    let mut pending: Vec<NewCandidate> = Vec::new();
    for pass in scan_passes(kind, &rules) {
        let mut window = PairWindow::new(&pass);
        let mut after: Option<(Vec<u8>, Uuid)> = None;
        loop {
            let page = store
                .scan_page(kind, &pass, after.as_ref(), SCAN_BATCH_SIZE)
                .await?;
            let full = page.len() == SCAN_BATCH_SIZE as usize;
            for (key, fingerprint) in page {
                let id = fingerprint.id;
                for (pair, matched) in window.push(kind, &rules, pass.group(&key), fingerprint) {
                    if seen.insert(pair) {
                        pending.push(new_candidate(pair.0, pair.1, matched, now));
                        summary.candidates += 1;
                    }
                }
                if pending.len() >= CANDIDATE_FLUSH_SIZE {
                    flush(ctx, &store, kind, &mut pending).await?;
                }
                after = Some((key, id));
            }
            if !full {
                break;
            }
        }
    }
    flush(ctx, &store, kind, &mut pending).await?;
    Ok(summary)
}

async fn matches_for(
    ctx: &Ctx<'_>,
    kind: &DuplicateType,
    probe: &RecordFingerprint,
    visibility: Option<&RecordScopeFilter>,
) -> Result<Vec<DuplicateMatch>, AppError> {
    let store = DuplicateStore::of(ctx);
    let stored = store.rule_settings(kind.as_str()).await?;
    let rules = enabled_rules(kind, &stored);
    if rules.is_empty() {
        return Ok(Vec::new());
    }
    let found = store
        .probable_matches(
            kind,
            probe,
            &rules,
            visibility.map(|filter| (ctx.scope, filter)),
        )
        .await?;
    Ok(found
        .into_iter()
        .filter(|candidate| candidate.id != probe.id)
        .filter_map(|candidate| {
            let rules = matched_rules(kind, probe, &candidate, &rules);
            (!rules.is_empty()).then(|| DuplicateMatch {
                record_id: candidate.id,
                title: candidate.name.clone(),
                rules,
            })
        })
        .collect())
}

async fn visible_matches(
    ctx: &Ctx<'_>,
    kind: &DuplicateType,
    matches: Vec<DuplicateMatch>,
) -> Result<Vec<DuplicateMatch>, AppError> {
    if matches.is_empty() {
        return Ok(matches);
    }
    let Ok(visibility) = ctx.scope.record_filter(&kind.read) else {
        return Ok(Vec::new());
    };
    let ids: Vec<Uuid> = matches.iter().map(|found| found.record_id).collect();
    let visible = visible_ids(ctx, kind, &visibility, &ids).await?;
    Ok(matches
        .into_iter()
        .filter(|found| visible.contains(&found.record_id))
        .collect())
}

pub(crate) async fn visible_ids(
    ctx: &Ctx<'_>,
    kind: &DuplicateType,
    visibility: &RecordScopeFilter,
    ids: &[Uuid],
) -> Result<Vec<Uuid>, AppError> {
    ctx.fields()
        .references()
        .existing_records(ctx.scope, &kind.definition, visibility, ids)
        .await
}

pub(crate) async fn cleanup_all(ctx: &Ctx<'_>) -> Result<CleanupSummary, AppError> {
    let mut summary = CleanupSummary::default();
    for kind in duplicate_types(ctx.registry()) {
        let found = cleanup_type(ctx, &kind).await?;
        summary.removed += found.removed;
        summary.updated += found.updated;
    }
    Ok(summary)
}

async fn cleanup_type(ctx: &Ctx<'_>, kind: &DuplicateType) -> Result<CleanupSummary, AppError> {
    let store = DuplicateStore::of(ctx);
    let stored = store.rule_settings(kind.as_str()).await?;
    let rules = enabled_rules(kind, &stored);
    let mut summary = CleanupSummary::default();
    if rules.is_empty() {
        return Ok(summary);
    }
    let mut after = None;
    loop {
        let page = store
            .pending_page(kind.as_str(), after, CLEANUP_BATCH_SIZE)
            .await?;
        let Some(last) = page.last() else {
            break;
        };
        after = Some(last.id);
        let ids: BTreeSet<Uuid> = page
            .iter()
            .flat_map(|candidate| [candidate.record_id, candidate.duplicate_id])
            .collect();
        let ids: Vec<Uuid> = ids.into_iter().collect();
        let fingerprints: BTreeMap<Uuid, RecordFingerprint> = store
            .fingerprints_by_ids(kind, &ids)
            .await?
            .into_iter()
            .map(|fingerprint| (fingerprint.id, fingerprint))
            .collect();
        let mut stale = Vec::new();
        let mut changed = Vec::new();
        for candidate in &page {
            let (Some(left), Some(right)) = (
                fingerprints.get(&candidate.record_id),
                fingerprints.get(&candidate.duplicate_id),
            ) else {
                stale.push(candidate.id);
                continue;
            };
            let matched = matched_rules(kind, left, right, &rules);
            if matched.is_empty() {
                stale.push(candidate.id);
            } else if matched != candidate.rules {
                changed.push((candidate.id, matched));
            }
        }
        if !stale.is_empty() || !changed.is_empty() {
            let mut tx = ctx.begin().await?;
            summary.removed += store.delete_pending(&mut tx, &stale).await? as usize;
            for (id, matched) in &changed {
                store.update_pending_rules(&mut tx, *id, matched).await?;
            }
            tx.commit().await?;
            summary.updated += changed.len();
        }
        if page.len() < CLEANUP_BATCH_SIZE as usize {
            break;
        }
    }
    Ok(summary)
}
