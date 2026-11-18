use aphel_contract_crm::CrmPermissions;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::domain::LinkOrigin;
use crate::linking::EMAIL_LINK_NOT_FOUND;
use crate::reading::{ThreadDetail, ThreadSummary};
use crate::service::reading;
use crate::store::LinkStore;
use crate::{EMAIL_THREAD_LINKED, EMAIL_THREAD_UNLINKED, NewRecordLink};
use aphel_kernel::Ctx;
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::application::access::{Authorizer, PERMISSION_DENIED};
use aphel_sdk::platform::application::files::RecordAccessChecker;
use aphel_sdk::platform::application::shared::{
    Actor, AuditEntry, AuditLog, DomainEvent, EventPublisher, FieldChange, RecordRef,
};
use aphel_sdk::platform::database::transaction::Tx;
use aphel_sdk::platform::domain::access::Permission;

pub(crate) async fn link(
    ctx: &Ctx<'_>,
    thread_id: &str,
    record_type: &str,
    record_id: &str,
    ip: Option<String>,
) -> Result<ThreadDetail, AppError> {
    let scope = ctx.scope;
    let (summary, record) = prepare(ctx, thread_id, record_type, record_id).await?;
    let existing = summary
        .records
        .iter()
        .find(|link| link.record == record)
        .map(|link| link.origin);
    if existing == Some(LinkOrigin::Manual) {
        return reading::get(ctx, thread_id).await;
    }
    let now = ctx.now();
    let mut tx = ctx.begin().await?;
    LinkStore::of(ctx)
        .link(
            &mut tx,
            &NewRecordLink {
                thread_id: summary.thread.id,
                record: record.clone(),
                origin: LinkOrigin::Manual,
                created_by: Some(scope.user_id().clone()),
            },
            now,
        )
        .await?;
    let changes = vec![FieldChange::new(
        "origin",
        existing.map_or(Value::Null, |origin| json!(origin.as_str())),
        json!(LinkOrigin::Manual.as_str()),
    )];
    record_change(
        ctx,
        &mut tx,
        EMAIL_THREAD_LINKED,
        &summary.thread.id,
        &summary.thread.mailbox_id,
        &record,
        changes,
        ip,
    )
    .await?;
    tx.commit().await?;
    reading::get(ctx, thread_id).await
}

pub(crate) async fn unlink(
    ctx: &Ctx<'_>,
    thread_id: &str,
    record_type: &str,
    record_id: &str,
    ip: Option<String>,
) -> Result<(), AppError> {
    let (summary, record) = prepare(ctx, thread_id, record_type, record_id).await?;
    let Some(existing) = summary.records.iter().find(|link| link.record == record) else {
        return Err(link_not_found());
    };
    let changes = vec![FieldChange::new(
        "origin",
        json!(existing.origin.as_str()),
        Value::Null,
    )];
    let mut tx = ctx.begin().await?;
    if !LinkStore::of(ctx)
        .unlink(&mut tx, summary.thread.id, &record)
        .await?
    {
        return Err(link_not_found());
    }
    record_change(
        ctx,
        &mut tx,
        EMAIL_THREAD_UNLINKED,
        &summary.thread.id,
        &summary.thread.mailbox_id,
        &record,
        changes,
        ip,
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

fn link_not_found() -> AppError {
    AppError::not_found_code(
        EMAIL_LINK_NOT_FOUND,
        "the conversation is not linked to this record",
    )
}

async fn prepare(
    ctx: &Ctx<'_>,
    thread_id: &str,
    record_type: &str,
    record_id: &str,
) -> Result<(ThreadSummary, RecordRef), AppError> {
    ctx.scope.require(&Permission::EMAILS_READ)?;
    let record = RecordRef::parse(record_type, record_id)?;
    let summary = reading::visible(ctx, thread_id).await?;
    if !summary.access.can_read_content() {
        return Err(AppError::forbidden_code(
            PERMISSION_DENIED,
            "full access to the conversation is required to change its links",
        ));
    }
    if !reading::record_access(ctx)
        .can_read(ctx.scope, &record)
        .await?
    {
        return Err(AppError::forbidden_code(
            PERMISSION_DENIED,
            format!(
                "missing permission to access {} records",
                record.record_type
            ),
        ));
    }
    Ok((summary, record))
}

#[allow(clippy::too_many_arguments)]
async fn record_change(
    ctx: &Ctx<'_>,
    tx: &mut Tx,
    event_type: &'static str,
    thread_id: &Uuid,
    mailbox_id: &Uuid,
    record: &RecordRef,
    changes: Vec<FieldChange>,
    ip: Option<String>,
) -> Result<(), AppError> {
    let now = ctx.now();
    let actor = Actor::from_scope(ctx.scope);
    ctx.events()
        .publish(
            tx,
            DomainEvent::new(actor.clone(), event_type, now)
                .with_record(record.clone())
                .with_payload(json!({
                    "thread_id": thread_id,
                    "mailbox_id": mailbox_id,
                    "record_type": record.record_type.as_str(),
                    "record_id": record.record_id,
                    "origin": LinkOrigin::Manual.as_str(),
                })),
        )
        .await?;
    ctx.audit()
        .record(
            tx,
            AuditEntry::new(actor, event_type, now)
                .with_record(record.clone())
                .with_changes(changes)
                .with_ip(ip),
        )
        .await
}
