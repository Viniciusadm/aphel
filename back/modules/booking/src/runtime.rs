use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use serde_json::Value;
use uuid::Uuid;

use crate::email::{
    BookingEmail, BookingEmailContext, BookingEmailKind, Recipient, render_booking_email,
};
use crate::service::booking;
use crate::store::BookingStore;
use crate::token::ManageToken;
use crate::validation::parse_timezone;
use crate::{BOOKING_EMAIL_JOB, BookingStatus};
use aphel_kernel::Ctx;
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::api::state::AppState;
use aphel_sdk::platform::application::mail::{CalendarInvite, EmailMessage, Mailer, SharedMailer};
use aphel_sdk::platform::application::organization_profile::PublicBaseUrl;
use aphel_sdk::platform::application::shared::{JobContext, JobHandler};
use aphel_sdk::platform::domain::access::AccessScope;

use aphel_kernel::modules::{WorkerEnv, WorkerParts};

pub(crate) struct BookingEmailHandler {
    state: AppState,
    mailer: SharedMailer,
    app_base_url: PublicBaseUrl,
}

pub(crate) fn booking_worker(env: &WorkerEnv) -> WorkerParts {
    let state = env.state();
    WorkerParts::default().handler(Arc::new(BookingEmailHandler {
        state,
        mailer: env.mailer.clone(),
        app_base_url: env.public_url.clone(),
    }))
}

fn text(payload: &Value, key: &str) -> Result<String, AppError> {
    payload
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| AppError::Validation(format!("booking e-mail job without `{key}`")))
}

impl BookingEmailHandler {
    async fn render(
        &self,
        booking_id: Uuid,
        kind: BookingEmailKind,
        recipient: Recipient,
    ) -> Result<Option<BookingEmail>, AppError> {
        let scope = AccessScope::system();
        let ctx = Ctx::from_state(&self.state, &scope).await?;
        let repository = BookingStore::of(&ctx);
        let Some(booking) = repository.booking(booking_id).await? else {
            return Ok(None);
        };
        if kind != BookingEmailKind::Canceled && booking.status != BookingStatus::Confirmed {
            return Ok(None);
        }
        let Some(meeting_type) = repository.meeting_type(booking.meeting_type_id).await? else {
            return Ok(None);
        };
        let Some(host) = repository.host(&booking.host_user_id).await? else {
            return Ok(None);
        };
        let organization = repository.organization().await?;
        let host_timezone = match repository.availability(&booking.host_user_id).await? {
            Some(availability) => availability.timezone,
            None => host
                .timezone
                .as_deref()
                .and_then(parse_timezone)
                .unwrap_or(organization.timezone),
        };
        let key = booking::token_key(&ctx).await?;
        let app_base_url = self.app_base_url.get();
        let manage_url = ManageToken { booking_id }.url(&app_base_url, &key)?;
        let booking_url = format!(
            "{}/book/{}",
            app_base_url.trim_end_matches('/'),
            meeting_type.slug
        );
        Ok(Some(render_booking_email(&BookingEmailContext {
            kind,
            recipient,
            booking: &booking,
            meeting_type: &meeting_type,
            host: &host,
            organization_name: &organization.name,
            host_timezone,
            manage_url: &manage_url,
            booking_url: &booking_url,
            now: Utc::now(),
        })))
    }
}

#[async_trait]
impl JobHandler for BookingEmailHandler {
    fn kind(&self) -> &str {
        BOOKING_EMAIL_JOB
    }

    async fn handle(&self, _context: &JobContext, payload: Value) -> Result<(), AppError> {
        let booking_id = Uuid::parse_str(&text(&payload, "booking_id")?)
            .map_err(|_| AppError::Validation("invalid booking id".to_owned()))?;
        let kind = BookingEmailKind::parse(&text(&payload, "kind")?)
            .ok_or_else(|| AppError::Validation("invalid booking e-mail kind".to_owned()))?;
        let recipient = Recipient::parse(&text(&payload, "recipient")?)
            .ok_or_else(|| AppError::Validation("invalid booking e-mail recipient".to_owned()))?;
        let Some(email) = self.render(booking_id, kind, recipient).await? else {
            return Ok(());
        };
        if aphel_sdk::platform::application::mail::delivery_override().is_some() {
            tracing::info!(
                subject = email.subject.as_str(),
                "mail policy: booking e-mail discarded instead of sent"
            );
            return Ok(());
        }
        self.mailer
            .send(EmailMessage {
                to: email.to,
                subject: email.subject,
                html: email.html,
                text: email.text,
                calendar: Some(CalendarInvite {
                    method: email.method.as_str().to_owned(),
                    content: email.calendar,
                }),
            })
            .await
    }
}
