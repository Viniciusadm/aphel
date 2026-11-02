use aphel_contract_crm::CrmRecordTypes;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::Duration;
use serde_json::{Value, json};
use sqlx::MySqlPool;
use uuid::Uuid;

use aphel_kernel::Ctx;
use aphel_kernel::application::custom_objects::RecordTypeCatalog;
use aphel_kernel::application::fields::RecordTypeRegistry;
use aphel_kernel::application::jobs::ScheduledJobDefinition;
use aphel_kernel::application::notifications::{NotificationIntent, dispatch};
use aphel_kernel::application::shared::{
    Actor, DomainEvent, EventConsumer, JobContext, JobHandler, RecordRef, RecordType,
};
use aphel_kernel::domain::UserId;
use aphel_kernel::domain::access::AccessScope;
use aphel_kernel::errors::AppError;
use aphel_kernel::infrastructure::audit::MySqlAuditLog;
use aphel_kernel::infrastructure::events::MySqlEventPublisher;
use aphel_kernel::infrastructure::jobs::MySqlJobQueue;
use aphel_kernel::infrastructure::notifications::MySqlNotificationPorts;

use crate::domain::{LEAD_HOT, LeadProSettings, TEMPERATURE_CHANGED, heated};
use crate::store;

pub(crate) const TOUCHES_CONSUMER: &str = "lead_pro.touches";
pub(crate) const COOLDOWN_JOB: &str = "lead_pro.cooldown";
pub(crate) const COOLDOWN_CRON: &str = "0 0 3 * * *";
pub(crate) const TOUCH_EVENTS: &[&str] = &[
    "activity.created",
    "note.created",
    "email.received",
    "email.sent",
    "meeting.booked",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Heating {
    pub(crate) before: Option<i64>,
    pub(crate) after: i64,
    pub(crate) became_hot: bool,
}

pub(crate) async fn heat(
    ctx: &Ctx<'_>,
    contact_id: Uuid,
    kind: &str,
    event_id: Option<Uuid>,
) -> Result<Option<Heating>, AppError> {
    let settings = LeadProSettings::load(ctx.pool()).await?;
    let now = ctx.now();
    let mut tx = ctx.begin().await?;
    if !contact_exists(&mut tx, contact_id).await? {
        return Ok(None);
    }
    if !store::insert_touch(&mut tx, contact_id, kind, event_id, now).await? {
        return Ok(None);
    }
    let before = store::temperature(&mut tx, contact_id).await?;
    let after = heated(before, &settings);
    store::set_temperature(&mut tx, contact_id, after).await?;
    let became_hot = !settings.is_hot(before) && settings.is_hot(Some(after));
    if before != Some(after) {
        ctx.events()
            .publish(
                &mut tx,
                DomainEvent::new(Actor::from_scope(ctx.scope), TEMPERATURE_CHANGED, now)
                    .with_record(RecordRef::new(RecordType::CONTACT, contact_id))
                    .with_payload(json!({
                        "touch": kind,
                        "old_temperature": before,
                        "temperature": after,
                        "hot": settings.is_hot(Some(after)),
                        "changes": [{ "field": "lead_temperature", "old": before, "new": after }],
                    })),
            )
            .await?;
    }
    tx.commit().await?;
    Ok(Some(Heating {
        before,
        after,
        became_hot,
    }))
}

async fn contact_exists(
    connection: &mut sqlx::MySqlConnection,
    contact_id: Uuid,
) -> Result<bool, AppError> {
    Ok(sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM contacts WHERE id = ? AND deleted_at IS NULL)",
    )
    .bind(contact_id.to_string())
    .fetch_one(connection)
    .await?)
}

fn touched_contact(event: &DomainEvent) -> Option<Uuid> {
    if let Some(record) = &event.record
        && record.record_type == RecordType::CONTACT
    {
        return Some(record.record_id);
    }
    let payload = &event.payload;
    (payload.get("record_type").and_then(Value::as_str) == Some(RecordType::CONTACT.as_str()))
        .then(|| payload.get("record_id").and_then(Value::as_str))
        .flatten()
        .and_then(|id| Uuid::parse_str(id).ok())
}

pub(crate) struct TouchConsumer {
    pool: MySqlPool,
    catalog: Arc<RecordTypeCatalog>,
}

impl TouchConsumer {
    pub(crate) fn new(pool: MySqlPool, catalog: Arc<RecordTypeCatalog>) -> Self {
        Self { pool, catalog }
    }

    async fn notify_owner(&self, contact_id: Uuid, heating: Heating) -> Result<(), AppError> {
        let mut connection = self.pool.acquire().await?;
        let Some(owner) = store::owner(&mut connection, contact_id).await? else {
            return Ok(());
        };
        let Ok(recipient) = UserId::new(owner) else {
            return Ok(());
        };
        let ports = MySqlNotificationPorts::new(
            self.pool.clone(),
            Arc::new(RecordTypeRegistry::builtin()),
            Arc::new(MySqlJobQueue::new(self.pool.clone())),
        );
        let mut intent = NotificationIntent::new(
            recipient,
            LEAD_HOT,
            format!("Lead quente: temperatura {}", heating.after),
            format!("lead_pro_hot:{contact_id}:{}", heating.after),
        );
        intent.record = Some(RecordRef::new(RecordType::CONTACT, contact_id));
        intent.link = Some(format!("/contacts/{contact_id}"));
        intent.data = json!({ "temperature": heating.after });
        dispatch(&ports, intent).await?;
        Ok(())
    }
}

#[async_trait]
impl EventConsumer for TouchConsumer {
    fn name(&self) -> &str {
        TOUCHES_CONSUMER
    }

    fn accepts(&self, event_type: &str) -> bool {
        TOUCH_EVENTS.contains(&event_type)
    }

    async fn consume(&self, event: &DomainEvent) -> Result<(), AppError> {
        let Some(contact_id) = touched_contact(event) else {
            return Ok(());
        };
        let scope = AccessScope::system();
        let ctx = Ctx::from_parts(
            &scope,
            self.pool.clone(),
            self.catalog.registry().await?,
            Arc::new(MySqlEventPublisher::new()),
            Arc::new(MySqlAuditLog::new()),
        );
        let Some(heating) = heat(&ctx, contact_id, &event.event_type, Some(event.id)).await? else {
            return Ok(());
        };
        if heating.became_hot {
            self.notify_owner(contact_id, heating).await?;
        }
        Ok(())
    }
}

pub(crate) fn cooldown_schedule() -> ScheduledJobDefinition {
    ScheduledJobDefinition::new(COOLDOWN_JOB, COOLDOWN_CRON)
        .expect("the lead_pro cooldown cron expression is valid")
        .with_max_attempts(1)
}

pub(crate) struct Cooldown {
    pool: MySqlPool,
}

impl Cooldown {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl JobHandler for Cooldown {
    fn kind(&self) -> &str {
        COOLDOWN_JOB
    }

    async fn handle(&self, _context: &JobContext, _payload: Value) -> Result<(), AppError> {
        let settings = LeadProSettings::load(&self.pool).await?;
        if settings.cooldown_points == 0 {
            return Ok(());
        }
        let idle_since = chrono::Utc::now() - Duration::days(settings.cooldown_days.max(1));
        let cooled = store::cool_down(&self.pool, settings.cooldown_points, idle_since).await?;
        tracing::debug!(contacts = cooled, "lead_pro cooldown applied");
        Ok(())
    }
}
