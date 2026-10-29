use aphel_kernel::application::mail::MailLocale;
use aphel_kernel::application::notifications::NotificationView;
use aphel_kernel::application::notifications::texts::{NotificationTexts, data_text};
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

use crate::domain::{
    APPROVAL_APPROVED, APPROVAL_CANCELLED, APPROVAL_OVERDUE, APPROVAL_REJECTED, APPROVAL_REQUESTED,
    APPROVAL_STEP_ADVANCED,
};

const EVENT_PREFIX: &str = "approval.";

pub(crate) struct ApprovalNotifications;

fn notification_kind(event_type: &str) -> Option<NotificationKind> {
    match event_type {
        APPROVAL_REQUESTED | APPROVAL_STEP_ADVANCED => Some(APPROVAL_REQUESTED_NOTIFICATION),
        APPROVAL_OVERDUE => Some(APPROVAL_OVERDUE_NOTIFICATION),
        APPROVAL_APPROVED | APPROVAL_REJECTED | APPROVAL_CANCELLED => {
            Some(APPROVAL_DECIDED_NOTIFICATION)
        }
        _ => None,
    }
}

#[async_trait]
impl NotificationSource for ApprovalNotifications {
    fn accepts(&self, event_type: &str) -> bool {
        notification_kind(event_type).is_some()
    }

    async fn intents(
        &self,
        _context: &NotificationSourceContext<'_>,
        event: &DomainEvent,
    ) -> Result<Vec<NotificationIntent>, AppError> {
        let Some(kind) = notification_kind(&event.event_type) else {
            return Ok(Vec::new());
        };
        let payload = &event.payload;
        let recipients: Vec<UserId> = if kind == APPROVAL_DECIDED_NOTIFICATION {
            payload_user(payload.get("requested_by"))
                .into_iter()
                .collect()
        } else {
            payload
                .get("approvers")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(|value| payload_user(Some(value)))
                .collect()
        };
        let status = event
            .event_type
            .strip_prefix(EVENT_PREFIX)
            .unwrap_or_default()
            .to_owned();
        let link = payload_text(payload, "request_id").map(|id| format!("/approvals/{id}"));
        Ok(recipients
            .into_iter()
            .map(|recipient| {
                NotificationIntent::new(
                    recipient,
                    kind,
                    payload_text(payload, "policy_name").unwrap_or_default(),
                    event_dedupe(event, kind),
                )
                .with_body(
                    payload_text(payload, "resolution_comment")
                        .or_else(|| payload_text(payload, "comment")),
                )
                .with_record(event.record.clone())
                .with_link(link.clone())
                .with_data(json!({
                    "request_id": payload.get("request_id"),
                    "policy_key": payload.get("policy_key"),
                    "record_type": payload.get("record_type"),
                    "record_id": payload.get("record_id"),
                    "moment": payload.get("moment"),
                    "status": status,
                }))
            })
            .collect())
    }
}

pub(crate) const APPROVAL_REQUESTED_NOTIFICATION: NotificationKind =
    NotificationKind::from_static("approval_requested");
pub(crate) const APPROVAL_DECIDED_NOTIFICATION: NotificationKind =
    NotificationKind::from_static("approval_decided");
pub(crate) const APPROVAL_OVERDUE_NOTIFICATION: NotificationKind =
    NotificationKind::from_static("approval_overdue");

pub(crate) struct ApprovalNotificationTexts;

impl NotificationTexts for ApprovalNotificationTexts {
    fn headline(&self, view: &NotificationView, actor: &str, locale: MailLocale) -> Option<String> {
        let en = locale == MailLocale::En;
        Some(match view.kind {
            APPROVAL_REQUESTED_NOTIFICATION if en => format!("{actor} asked for your approval"),
            APPROVAL_REQUESTED_NOTIFICATION => format!("{actor} pediu sua aprovação"),
            APPROVAL_DECIDED_NOTIFICATION => match data_text(view, "status") {
                Some("approved") if en => format!("{actor} approved your request"),
                Some("approved") => format!("{actor} aprovou sua solicitação"),
                Some("rejected") if en => format!("{actor} rejected your request"),
                Some("rejected") => format!("{actor} rejeitou sua solicitação"),
                _ if en => format!("{actor} cancelled an approval request"),
                _ => format!("{actor} cancelou uma solicitação de aprovação"),
            },
            APPROVAL_OVERDUE_NOTIFICATION => locale
                .pick(
                    "Uma aprovação aguardando você passou do prazo",
                    "An approval waiting for you is overdue",
                )
                .to_owned(),
            _ => return None,
        })
    }

    fn default_path(&self, view: &NotificationView) -> Option<&'static str> {
        matches!(
            view.kind,
            APPROVAL_REQUESTED_NOTIFICATION
                | APPROVAL_DECIDED_NOTIFICATION
                | APPROVAL_OVERDUE_NOTIFICATION
        )
        .then_some("/approvals")
    }
}
