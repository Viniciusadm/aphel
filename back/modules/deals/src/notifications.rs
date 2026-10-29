use aphel_kernel::application::mail::MailLocale;
use aphel_kernel::application::notifications::NotificationView;
use aphel_kernel::application::notifications::texts::NotificationTexts;
use async_trait::async_trait;
use serde_json::{Value, json};

use aphel_kernel::application::notifications::{
    NotificationIntent, NotificationSource, NotificationSourceContext, event_dedupe, payload_text,
    payload_user,
};
use aphel_kernel::application::shared::DomainEvent;
use aphel_kernel::domain::UserId;
use aphel_kernel::domain::notifications::NotificationKind;
use aphel_kernel::errors::AppError;

use crate::domain::{DEAL_CREATED, DEAL_OWNER_CHANGED, DEAL_STAGE_CHANGED};

pub(crate) struct DealNotifications;

fn stage_name(payload: &Value, side: &str) -> String {
    payload
        .get(side)
        .and_then(|stage| stage.get("name"))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

async fn deal_intent(
    context: &NotificationSourceContext<'_>,
    event: &DomainEvent,
    kind: NotificationKind,
    owner: Option<UserId>,
) -> Result<Vec<NotificationIntent>, AppError> {
    let Some(record) = event.record.clone() else {
        return Ok(Vec::new());
    };
    let summary = context.directory.record_summary(&record).await?;
    let Some(owner) = owner.or_else(|| summary.as_ref().and_then(|deal| deal.owner_id.clone()))
    else {
        return Ok(Vec::new());
    };
    let title = summary
        .map(|deal| deal.title)
        .or_else(|| payload_text(&event.payload, "title"))
        .unwrap_or_default();
    let mut intent = NotificationIntent::new(owner, kind, title, event_dedupe(event, kind))
        .with_record(Some(record));
    if kind == DEAL_STAGE_CHANGED_NOTIFICATION {
        let (from, to) = (
            stage_name(&event.payload, "from"),
            stage_name(&event.payload, "to"),
        );
        intent = intent
            .with_body(Some(format!("{from} → {to}")))
            .with_data(json!({ "from": from, "to": to }));
    }
    Ok(vec![intent])
}

#[async_trait]
impl NotificationSource for DealNotifications {
    fn accepts(&self, event_type: &str) -> bool {
        matches!(
            event_type,
            DEAL_CREATED | DEAL_OWNER_CHANGED | DEAL_STAGE_CHANGED
        )
    }

    async fn intents(
        &self,
        context: &NotificationSourceContext<'_>,
        event: &DomainEvent,
    ) -> Result<Vec<NotificationIntent>, AppError> {
        match event.event_type.as_str() {
            DEAL_CREATED => deal_intent(context, event, DEAL_ASSIGNED_NOTIFICATION, None).await,
            DEAL_OWNER_CHANGED => {
                let owner = event
                    .payload
                    .get("new")
                    .and_then(|owner| payload_user(owner.get("id")));
                if owner.is_none() {
                    return Ok(Vec::new());
                }
                deal_intent(context, event, DEAL_ASSIGNED_NOTIFICATION, owner).await
            }
            DEAL_STAGE_CHANGED => {
                deal_intent(context, event, DEAL_STAGE_CHANGED_NOTIFICATION, None).await
            }
            _ => Ok(Vec::new()),
        }
    }
}

pub(crate) const DEAL_ASSIGNED_NOTIFICATION: NotificationKind =
    NotificationKind::from_static("deal_assigned");
pub(crate) const DEAL_STAGE_CHANGED_NOTIFICATION: NotificationKind =
    NotificationKind::from_static("deal_stage_changed");

pub(crate) struct DealNotificationTexts;

impl NotificationTexts for DealNotificationTexts {
    fn headline(&self, view: &NotificationView, actor: &str, locale: MailLocale) -> Option<String> {
        let en = locale == MailLocale::En;
        Some(match view.kind {
            DEAL_ASSIGNED_NOTIFICATION if en => format!("{actor} assigned you a deal"),
            DEAL_ASSIGNED_NOTIFICATION => format!("{actor} atribuiu uma oportunidade a você"),
            DEAL_STAGE_CHANGED_NOTIFICATION if en => {
                format!("{actor} changed the stage of one of your deals")
            }
            DEAL_STAGE_CHANGED_NOTIFICATION => {
                format!("{actor} mudou a etapa de uma oportunidade sua")
            }
            _ => return None,
        })
    }
}
