use serde_json::{Value, json};
use uuid::Uuid;

use crate::domain::{
    CreateSegmentCommand, MAX_SEGMENTS_PER_TYPE, SEGMENT_CREATED, SEGMENT_DELETED,
    SEGMENT_RECORD_TYPE, SEGMENT_UPDATED, Segment, SegmentMember, SegmentMembersQuery, SegmentView,
    UpdateSegmentCommand, normalize_description, normalize_name, segment_name_taken,
    segment_not_found,
};
use crate::store::SegmentStore;
use aphel_kernel::Ctx;
use aphel_kernel::application::access::Authorizer;
use aphel_kernel::application::audit::{DiffOptions, diff_values};
use aphel_kernel::application::fields::permissions::query_error;
use aphel_kernel::application::fields::{EntityDefinition, FieldSet};
use aphel_kernel::application::organization_admin::{ListWindow, Listing};
use aphel_kernel::application::query::{
    Filter, RecordQuery, RecordQueryEngine, RecordQueryExecutor, RecordQueryPorts, ResolveContext,
    SegmentResolution, in_segment, resolve_filter_with,
};
use aphel_kernel::application::shared::{RecordRef, RecordType, ensure_version};
use aphel_kernel::database::transaction::MySqlTx;
use aphel_kernel::domain::access::Permission;
use aphel_kernel::errors::{AppError, FieldViolation};
use aphel_kernel::records::{RecordMutation, publish_mutation};

pub(crate) async fn list(
    ctx: &Ctx<'_>,
    record_type: Option<&str>,
    with_counts: bool,
) -> Result<Vec<SegmentView>, AppError> {
    let record_type = match record_type.map(str::trim).filter(|value| !value.is_empty()) {
        Some(record_type) => Some(readable(ctx, record_type)?.record_type.clone()),
        None => None,
    };
    let segments = SegmentStore::of(ctx).list(record_type.as_ref()).await?;
    let mut views = Vec::new();
    for segment in segments {
        if readable(ctx, segment.record_type.as_str()).is_err() {
            continue;
        }
        let member_count = if with_counts {
            count(ctx, &segment.record_type, segment.id).await
        } else {
            None
        };
        views.push(SegmentView {
            segment,
            member_count,
        });
    }
    Ok(views)
}

pub(crate) async fn get(ctx: &Ctx<'_>, segment_id: &str) -> Result<SegmentView, AppError> {
    let segment = visible(ctx, segment_id).await?;
    let member_count = count(ctx, &segment.record_type, segment.id).await;
    Ok(SegmentView {
        segment,
        member_count,
    })
}

pub(crate) async fn create(
    ctx: &Ctx<'_>,
    command: CreateSegmentCommand,
) -> Result<SegmentView, AppError> {
    ctx.scope.require(&Permission::SETTINGS_MANAGE)?;
    let definition = readable(ctx, &command.record_type)?.clone();
    let mut violations = Vec::new();
    let name = normalize_name(&command.name, &mut violations);
    let description = normalize_description(command.description.as_deref(), &mut violations);
    match &command.filter {
        Some(filter) => validate_filter(ctx, &definition, filter, &mut violations).await?,
        None => violations.push(FieldViolation::new("filter", "is required")),
    }
    let (Some(name), Some(filter)) = (name, command.filter) else {
        return Err(query_error("invalid segment", violations));
    };
    if !violations.is_empty() {
        return Err(query_error("invalid segment", violations));
    }
    let store = SegmentStore::of(ctx);
    ensure_name_free(&store, &definition.record_type, &name, None).await?;
    if store.list(Some(&definition.record_type)).await?.len() >= MAX_SEGMENTS_PER_TYPE {
        return Err(AppError::validation_code(
            "too_many_segments",
            format!("at most {MAX_SEGMENTS_PER_TYPE} segments per record type are allowed"),
        ));
    }
    let now = ctx.now();
    let segment = Segment {
        id: Uuid::now_v7(),
        record_type: definition.record_type.clone(),
        name,
        description,
        filter,
        created_by: Some(ctx.scope.user_id().clone()),
        created_at: now,
        updated_at: now,
    };
    let mut tx = ctx.begin().await?;
    store.insert(&mut tx, &segment).await?;
    publish(
        ctx,
        &mut tx,
        &segment,
        SEGMENT_CREATED,
        (&Value::Null, &segment.snapshot()),
        command.ip,
    )
    .await?;
    tx.commit().await?;
    let member_count = count(ctx, &segment.record_type, segment.id).await;
    Ok(SegmentView {
        segment,
        member_count,
    })
}

pub(crate) async fn update(
    ctx: &Ctx<'_>,
    segment_id: &str,
    command: UpdateSegmentCommand,
) -> Result<SegmentView, AppError> {
    ctx.scope.require(&Permission::SETTINGS_MANAGE)?;
    let current = visible(ctx, segment_id).await?;
    let definition = readable(ctx, current.record_type.as_str())?.clone();
    ensure_version(command.expected_updated_at, current.updated_at)?;
    let mut segment = current.clone();
    let mut violations = Vec::new();
    if let Some(name) = &command.name
        && let Some(name) = normalize_name(name, &mut violations)
    {
        segment.name = name;
    }
    if let Some(description) = &command.description {
        segment.description = normalize_description(description.as_deref(), &mut violations);
    }
    if let Some(filter) = command.filter {
        validate_filter(ctx, &definition, &filter, &mut violations).await?;
        segment.filter = filter;
    }
    if !violations.is_empty() {
        return Err(query_error("invalid segment", violations));
    }
    let before = current.snapshot();
    let after = segment.snapshot();
    if before != after {
        let store = SegmentStore::of(ctx);
        if segment.name != current.name {
            ensure_name_free(
                &store,
                &segment.record_type,
                &segment.name,
                Some(segment.id),
            )
            .await?;
        }
        segment.updated_at = ctx.now();
        let mut tx = ctx.begin().await?;
        store.update(&mut tx, &segment).await?;
        publish(
            ctx,
            &mut tx,
            &segment,
            SEGMENT_UPDATED,
            (&before, &after),
            command.ip,
        )
        .await?;
        tx.commit().await?;
    }
    let member_count = count(ctx, &segment.record_type, segment.id).await;
    Ok(SegmentView {
        segment,
        member_count,
    })
}

pub(crate) async fn delete(
    ctx: &Ctx<'_>,
    segment_id: &str,
    ip: Option<String>,
) -> Result<(), AppError> {
    ctx.scope.require(&Permission::SETTINGS_MANAGE)?;
    let segment = visible(ctx, segment_id).await?;
    let mut tx = ctx.begin().await?;
    SegmentStore::of(ctx).delete(&mut tx, segment.id).await?;
    publish(
        ctx,
        &mut tx,
        &segment,
        SEGMENT_DELETED,
        (&segment.snapshot(), &Value::Null),
        ip,
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

pub(crate) async fn members(
    ctx: &Ctx<'_>,
    segment_id: &str,
    query: SegmentMembersQuery,
) -> Result<Listing<SegmentMember>, AppError> {
    let segment = visible(ctx, segment_id).await?;
    let membership = in_segment(segment.id);
    let filter = match query.filter {
        Some(extra) => Filter::and(vec![membership, extra]),
        None => membership,
    };
    let ports = ctx.query();
    let listing = RecordQueryEngine::new(&ports)
        .query(
            ctx.scope,
            segment.record_type.as_str(),
            RecordQuery {
                filter: Some(filter),
                sort: query.sort,
                search: query.search,
                archived: query.archived,
                window: query.window,
            },
        )
        .await?;
    let definition = ctx.registry().require(segment.record_type.as_str())?;
    let labels = SegmentStore::of(ctx)
        .labels(definition, &listing.items)
        .await?;
    Ok(Listing {
        items: listing
            .items
            .into_iter()
            .map(|id| SegmentMember {
                label: labels.get(&id).cloned(),
                id,
            })
            .collect(),
        total: listing.total,
    })
}

async fn count(ctx: &Ctx<'_>, record_type: &RecordType, segment_id: Uuid) -> Option<u64> {
    let ports = ctx.query();
    RecordQueryEngine::new(&ports)
        .query(
            ctx.scope,
            record_type.as_str(),
            RecordQuery {
                filter: Some(in_segment(segment_id)),
                window: ListWindow {
                    offset: 0,
                    limit: 1,
                },
                ..RecordQuery::default()
            },
        )
        .await
        .ok()
        .map(|listing| listing.total)
}

async fn validate_filter(
    ctx: &Ctx<'_>,
    definition: &EntityDefinition,
    filter: &Filter,
    violations: &mut Vec<FieldViolation>,
) -> Result<(), AppError> {
    let query = ctx.query();
    let fields = FieldSet::for_scope(query.definitions(), definition, ctx.scope).await?;
    let context = ResolveContext {
        now: ctx.now(),
        timezone: query.executor().timezone().await?,
        user_id: ctx.scope.user_id().clone(),
    };
    let before = violations.len();
    let node = resolve_filter_with(
        &fields,
        &context,
        filter,
        SegmentResolution::Reject,
        violations,
    );
    if node.is_none() && violations.len() == before {
        violations.push(FieldViolation::new(
            "filter",
            "must have at least one condition",
        ));
    }
    Ok(())
}

fn readable<'c>(ctx: &'c Ctx<'_>, record_type: &str) -> Result<&'c EntityDefinition, AppError> {
    let definition = ctx.registry().require(record_type.trim())?;
    ctx.scope.record_filter(&definition.read_permission)?;
    Ok(definition)
}

async fn visible(ctx: &Ctx<'_>, segment_id: &str) -> Result<Segment, AppError> {
    let id = Uuid::parse_str(segment_id.trim()).map_err(|_| segment_not_found())?;
    let segment = SegmentStore::of(ctx)
        .find(id)
        .await?
        .ok_or_else(segment_not_found)?;
    if readable(ctx, segment.record_type.as_str()).is_err() {
        return Err(segment_not_found());
    }
    Ok(segment)
}

async fn ensure_name_free(
    store: &SegmentStore,
    record_type: &RecordType,
    name: &str,
    except: Option<Uuid>,
) -> Result<(), AppError> {
    match store.find_by_name(record_type, name).await? {
        Some(existing) if Some(existing.id) != except => Err(segment_name_taken(name)),
        _ => Ok(()),
    }
}

async fn publish(
    ctx: &Ctx<'_>,
    tx: &mut MySqlTx,
    segment: &Segment,
    event_type: &'static str,
    (before, after): (&Value, &Value),
    ip: Option<String>,
) -> Result<(), AppError> {
    let changes = diff_values(before, after, &DiffOptions::default())?;
    publish_mutation(
        ctx.events(),
        ctx.audit(),
        tx,
        ctx.scope,
        ctx.now(),
        RecordMutation {
            record: RecordRef::new(SEGMENT_RECORD_TYPE, segment.id),
            event_type,
            payload: json!({
                "name": segment.name,
                "record_type": segment.record_type.as_str(),
            }),
            changes,
            ip,
        },
    )
    .await
}
