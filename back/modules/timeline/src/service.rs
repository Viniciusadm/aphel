use aphel_contract_crm::{CrmPermissions, CrmRecordTypes};
use std::cmp::Ordering;
use std::collections::{BTreeSet, HashMap};

use serde_json::{Map, Value, json};

use crate::domain::{
    DEFAULT_TIMELINE_PAGE_SIZE, MAX_TIMELINE_PAGE_SIZE, TimelineCursor, TimelineEntry,
    TimelineKind, TimelinePage, TimelineQuery, TimelineRequest,
};
use crate::store::TimelineStore;
use aphel_kernel::Ctx;
use aphel_kernel::application::access::{Authorizer, PERMISSION_DENIED};
use aphel_kernel::application::files::RecordAccessChecker;
use aphel_kernel::application::shared::{RecordRef, RecordType};
use aphel_kernel::domain::access::Permission;
use aphel_kernel::errors::{AppError, FieldViolation};
use aphel_kernel::infrastructure::records::MySqlRecordAccessChecker;
use aphel_kernel::records::serialize_records;

pub(crate) fn parse_kinds(value: Option<&str>) -> Result<BTreeSet<TimelineKind>, FieldViolation> {
    let requested: Vec<&str> = value
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|kind| !kind.is_empty())
        .collect();
    if requested.is_empty() {
        return Ok(TimelineKind::ALL.into_iter().collect());
    }
    requested
        .into_iter()
        .map(|kind| {
            TimelineKind::parse(kind).ok_or_else(|| {
                FieldViolation::new(
                    "kinds",
                    "must list activity, note, deal_stage, deal_change, record_change, attachment or event",
                )
            })
        })
        .collect()
}

pub(crate) fn compare_entries(left: &TimelineEntry, right: &TimelineEntry) -> Ordering {
    right
        .occurred_at
        .cmp(&left.occurred_at)
        .then_with(|| right.id.cmp(&left.id))
}

pub(crate) async fn page(ctx: &Ctx<'_>, query: TimelineQuery) -> Result<TimelinePage, AppError> {
    let scope = ctx.scope;
    let subject = RecordRef::parse(&query.record_type, &query.record_id)?;
    let mut violations = Vec::new();
    let kinds = parse_kinds(query.kinds.as_deref())
        .map_err(|violation| violations.push(violation))
        .unwrap_or_default();
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
        violations.push(FieldViolation::new(
            "per_page",
            "must be an integer between 1 and 100",
        ));
    }
    if !violations.is_empty() {
        return Err(AppError::invalid_fields(
            "invalid timeline query",
            violations,
        ));
    }
    if !MySqlRecordAccessChecker::new(ctx.pool().clone(), ctx.registry().clone())
        .can_read(scope, &subject)
        .await?
    {
        return Err(AppError::forbidden_code(
            PERMISSION_DENIED,
            format!(
                "missing permission to access {} records",
                subject.record_type
            ),
        ));
    }

    let is_company = subject.record_type == RecordType::COMPANY;
    let request = TimelineRequest {
        activities: if kinds.contains(&TimelineKind::Activity) {
            scope.record_filter(&Permission::ACTIVITIES_READ).ok()
        } else {
            None
        },
        notes: kinds.contains(&TimelineKind::Note)
            && scope.require(&Permission::NOTES_READ).is_ok(),
        related_contacts: if is_company {
            scope.record_filter(&Permission::CONTACTS_READ).ok()
        } else {
            None
        },
        related_deals: if is_company {
            scope.record_filter(&Permission::DEALS_READ).ok()
        } else {
            None
        },
        emails: scope.require(&Permission::EMAILS_READ).is_ok(),
        subject,
        kinds,
        cursor,
        limit: per_page + 1,
    };
    let mut entries = TimelineStore::of(ctx).entries(scope, &request).await?;
    entries.retain(|entry| request.kinds.contains(&entry.kind));
    if let Some(cursor) = &request.cursor {
        entries.retain(|entry| {
            entry.occurred_at < cursor.occurred_at
                || (entry.occurred_at == cursor.occurred_at && entry.id < cursor.item_id)
        });
    }
    entries.sort_by(compare_entries);
    let has_more = entries.len() > per_page as usize;
    entries.truncate(per_page as usize);
    attach_activity_contributions(ctx, &mut entries).await?;
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

async fn attach_activity_contributions(
    ctx: &Ctx<'_>,
    entries: &mut [TimelineEntry],
) -> Result<(), AppError> {
    let ids: Vec<Value> = entries
        .iter()
        .filter(|entry| entry.kind == TimelineKind::Activity)
        .filter_map(|entry| entry.metadata.get("activity_id").cloned())
        .map(|id| json!({ "id": id }))
        .collect();
    if ids.is_empty() {
        return Ok(());
    }
    let mut contributions: HashMap<String, Map<String, Value>> =
        serialize_records(ctx.pool(), ctx.scope, &RecordType::ACTIVITY, ids)
            .await?
            .into_iter()
            .filter_map(|item| match item {
                Value::Object(mut object) => {
                    let id = object.remove("id")?.as_str()?.to_owned();
                    Some((id, object))
                }
                _ => None,
            })
            .collect();
    for entry in entries
        .iter_mut()
        .filter(|entry| entry.kind == TimelineKind::Activity)
    {
        let Some(extra) = entry
            .metadata
            .get("activity_id")
            .and_then(Value::as_str)
            .and_then(|id| contributions.remove(id))
        else {
            continue;
        };
        if let Some(metadata) = entry.metadata.as_object_mut() {
            for (key, value) in extra {
                metadata.entry(key).or_insert(value);
            }
        }
    }
    Ok(())
}
