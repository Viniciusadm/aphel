use crate::sync::ContactMatch;
use aphel_kernel::application::fields::{CustomFieldPorts, FieldSet};
use aphel_kernel::application::organization_admin::ListWindow;
use aphel_kernel::application::query::{
    ArchivedFilter, Filter, FilterOperator, RecordQuery, SortSpec,
};
use aphel_kernel::application::shared::RecordType;
use aphel_sdk::{AppError, Ctx};
use serde_json::{Value, json};
use uuid::Uuid;

async fn readable(ctx: &Ctx<'_>, record_type: &str, field: &str) -> Result<bool, AppError> {
    let Some(definition) = ctx.registry().get(&RecordType::parse(record_type)?) else {
        return Ok(false);
    };
    if ctx.scope.scope_for(&definition.read_permission).is_none() {
        return Ok(false);
    }
    Ok(
        !FieldSet::for_scope(ctx.fields().definitions(), definition, ctx.scope)
            .await?
            .is_hidden(field),
    )
}
pub(crate) async fn contacts(
    ctx: &Ctx<'_>,
    addresses: &[String],
    include_archived: bool,
) -> Result<Vec<ContactMatch>, AppError> {
    if addresses.is_empty() || !readable(ctx, aphel_sdk::people::RECORD_TYPE, "email").await? {
        return Ok(Vec::new());
    }
    let addresses: Vec<_> = addresses
        .iter()
        .map(|address| address.to_lowercase())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    let mut matches = Vec::new();
    for batch in addresses.chunks(100) {
        let mut offset = 0;
        loop {
            let listing = aphel_sdk::record_context::list(
                ctx,
                aphel_sdk::people::RECORD_TYPE,
                RecordQuery {
                    filter: Some(Filter::condition("email", FilterOperator::In, json!(batch))),
                    sort: vec![SortSpec::asc("created_at"), SortSpec::asc("id")],
                    archived: if include_archived {
                        ArchivedFilter::Include
                    } else {
                        ArchivedFilter::Exclude
                    },
                    window: ListWindow {
                        offset,
                        limit: 1000,
                    },
                    ..Default::default()
                },
            )
            .await?;
            let count = listing.items.len();
            for record in listing.items {
                if let Some((address, contact_id)) =
                    record.get("email").and_then(Value::as_str).zip(
                        record
                            .get("id")
                            .and_then(Value::as_str)
                            .and_then(|id| Uuid::parse_str(id).ok()),
                    )
                {
                    matches.push(ContactMatch {
                        address: address.to_lowercase(),
                        contact_id,
                    });
                }
            }
            if count < 1000 {
                break;
            }
            offset += 1000;
        }
    }
    Ok(matches)
}
pub(crate) async fn company_by_domain(
    ctx: &Ctx<'_>,
    domain: &str,
) -> Result<Option<Uuid>, AppError> {
    if !readable(ctx, aphel_sdk::organizations::RECORD_TYPE, "domain").await? {
        return Ok(None);
    }
    for archived in [ArchivedFilter::Exclude, ArchivedFilter::Only] {
        let listing = aphel_sdk::record_context::list(
            ctx,
            aphel_sdk::organizations::RECORD_TYPE,
            RecordQuery {
                filter: Some(Filter::condition(
                    "domain",
                    FilterOperator::Eq,
                    json!(domain.to_lowercase()),
                )),
                sort: vec![SortSpec::asc("created_at"), SortSpec::asc("id")],
                archived,
                window: ListWindow {
                    offset: 0,
                    limit: 1,
                },
                ..Default::default()
            },
        )
        .await?;
        if let Some(record) = listing.items.first() {
            return record
                .get("id")
                .and_then(Value::as_str)
                .map(|id| {
                    Uuid::parse_str(id).map_err(|_| {
                        AppError::Internal("invalid company directory identity".into())
                    })
                })
                .transpose();
        }
    }
    Ok(None)
}

pub(crate) async fn contact_records(
    ctx: &Ctx<'_>,
    ids: &[Uuid],
    include_open_deals: bool,
) -> Result<Vec<aphel_sdk::shared::RecordRef>, AppError> {
    let mut records = std::collections::HashSet::new();
    let party_type = RecordType::parse(aphel_sdk::people::RECORD_TYPE)?;
    for source in ctx
        .app()
        .contributions::<dyn aphel_sdk::party::PartyAssociationSource>()
    {
        let target = source.target_type();
        if target != aphel_sdk::organizations::RECORD_TYPE
            && !(include_open_deals && target == aphel_sdk::deals::RECORD_TYPE)
        {
            continue;
        }
        let Some(definition) = ctx.registry().get(&RecordType::parse(target)?) else {
            continue;
        };
        if ctx.scope.scope_for(&definition.read_permission).is_none() {
            continue;
        }
        let mut candidates = std::collections::BTreeSet::new();
        for batch in ids.chunks(1000) {
            candidates.extend(source.linked_ids(ctx, &party_type, batch).await?);
        }
        let candidates: Vec<_> = candidates.into_iter().collect();
        for batch in candidates.chunks(100) {
            let listing = aphel_sdk::record_context::list(
                ctx,
                target,
                RecordQuery {
                    filter: Some(Filter::condition("id", FilterOperator::In, json!(batch))),
                    archived: ArchivedFilter::Exclude,
                    window: ListWindow {
                        offset: 0,
                        limit: 100,
                    },
                    ..Default::default()
                },
            )
            .await?;
            for record in listing.items {
                if let Some(id) = record
                    .get("id")
                    .and_then(Value::as_str)
                    .and_then(|id| Uuid::parse_str(id).ok())
                    && batch.contains(&id)
                {
                    records.insert(aphel_sdk::shared::RecordRef::new(
                        RecordType::parse(target)?,
                        id,
                    ));
                }
            }
        }
    }
    let mut records: Vec<_> = records.into_iter().collect();
    records.sort_by(|left, right| {
        left.record_type
            .as_str()
            .cmp(right.record_type.as_str())
            .then(left.record_id.cmp(&right.record_id))
    });
    Ok(records)
}
