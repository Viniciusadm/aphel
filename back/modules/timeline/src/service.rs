use crate::domain::{
    DEFAULT_TIMELINE_PAGE_SIZE, MAX_TIMELINE_PAGE_SIZE, TimelineCursor, TimelineEntry,
    TimelinePage, TimelineQuery, TimelineRequest, TimelineSource,
};
use aphel_kernel::{
    Ctx,
    application::{files::RecordAccessChecker, shared::RecordRef},
    errors::{AppError, FieldViolation},
    infrastructure::records::MySqlRecordAccessChecker,
};
use std::cmp::Ordering;
use std::collections::{BTreeSet, HashSet};
pub(crate) fn compare_entries(left: &TimelineEntry, right: &TimelineEntry) -> Ordering {
    right
        .occurred_at
        .cmp(&left.occurred_at)
        .then_with(|| right.id.cmp(&left.id))
}
pub(crate) async fn authorize(ctx: &Ctx<'_>, subject: &RecordRef) -> Result<(), AppError> {
    if !aphel_sdk::records::enabled(ctx, &subject.record_type) {
        return Err(aphel_kernel::modules::module_disabled());
    }
    if !MySqlRecordAccessChecker::new(ctx.pool().clone(), ctx.registry().clone())
        .can_read(ctx.scope, subject)
        .await?
    {
        return Err(AppError::forbidden_code(
            "permission_denied",
            "missing read access to the record",
        ));
    }
    Ok(())
}
pub(crate) async fn page(ctx: &Ctx<'_>, query: TimelineQuery) -> Result<TimelinePage, AppError> {
    let subject = RecordRef::parse(&query.record_type, &query.record_id)?;
    authorize(ctx, &subject).await?;
    let catalog = crate::domain::kinds(ctx)?;
    let mut kinds: BTreeSet<String> = query
        .kinds
        .as_deref()
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|key| !key.is_empty())
        .map(str::to_owned)
        .collect();
    let mut violations = Vec::new();
    if kinds.is_empty() {
        kinds = catalog.iter().map(|kind| kind.key.clone()).collect();
    }
    if kinds
        .iter()
        .any(|key| !catalog.iter().any(|kind| &kind.key == key))
    {
        violations.push(FieldViolation::new(
            "kinds",
            "must list contributed timeline kinds",
        ));
    }
    let cursor = match query.cursor.as_deref().map(str::trim) {
        None | Some("") => None,
        Some(value) => {
            let cursor = TimelineCursor::decode(value);
            if cursor.is_none() {
                violations.push(FieldViolation::new("cursor", "is not a valid cursor"));
            }
            cursor
        }
    };
    let per_page = query.per_page.unwrap_or(DEFAULT_TIMELINE_PAGE_SIZE);
    if !(1..=MAX_TIMELINE_PAGE_SIZE).contains(&per_page) {
        violations.push(FieldViolation::new("per_page", "must be between 1 and 100"));
    }
    if !violations.is_empty() {
        return Err(AppError::invalid_fields(
            "invalid timeline query",
            violations,
        ));
    }
    let request = TimelineRequest {
        subject,
        kinds,
        cursor,
        limit: per_page + 1,
    };
    let mut entries = Vec::new();
    let mut sources = HashSet::new();
    for source in ctx.app().contributions::<dyn TimelineSource>() {
        if !aphel_sdk::timeline::valid_kind(source.key()) || !sources.insert(source.key()) {
            return Err(AppError::Configuration(
                "invalid or duplicate timeline source".to_owned(),
            ));
        }
        let prefix = format!("{}:", source.key());
        let mut page = source.entries(ctx, &request).await?;
        if page.len() > request.limit as usize
            || page.iter().any(|entry| {
                !entry.id.starts_with(&prefix)
                    || entry.id.len() > 200
                    || !entry.id.chars().all(|character| {
                        character.is_ascii_alphanumeric()
                            || matches!(character, ':' | '-' | '_' | '.')
                    })
                    || !source.kinds().iter().any(|kind| kind.key == entry.kind)
                        && source.key() != "event"
            })
        {
            return Err(AppError::Configuration(
                "timeline source returned an invalid page".to_owned(),
            ));
        }
        page.retain(|entry| {
            request.wants(&entry.kind)
                && request.cursor.as_ref().is_none_or(|cursor| {
                    entry.occurred_at < cursor.occurred_at
                        || entry.occurred_at == cursor.occurred_at && entry.id < cursor.item_id
                })
        });
        entries.extend(page);
    }
    entries.sort_by(compare_entries);
    let mut ids = HashSet::new();
    entries.retain(|entry| ids.insert(entry.id.clone()));
    let has_more = entries.len() > per_page as usize;
    entries.truncate(per_page as usize);
    let next_cursor = if has_more {
        entries
            .last()
            .map(|entry| TimelineCursor::new(entry.occurred_at, entry.id.clone()).encode())
    } else {
        None
    };
    Ok(TimelinePage {
        items: entries,
        next_cursor,
    })
}
