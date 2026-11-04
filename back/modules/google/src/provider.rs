use aphel_sdk::sso::errors::internal;
use aphel_sdk::sso::login::{EMAIL_NOT_VERIFIED, failure};
use aphel_sdk::sso::oauth::http_client;
use aphel_sdk::sso::{IdTokenClaims, IdentityProvider, ProviderEndpoints, StoredState};
use async_trait::async_trait;
use axum::response::Response;
use chrono::{DateTime, NaiveDate, SecondsFormat, Utc};
use email::oauth::connect::PURPOSE_MAIL;
use tasks::remote_calendar::connect::complete_connection;
use tasks::remote_calendar::engine::parse_instant;
use tasks::remote_calendar::http::{encode_segment, send};
use tasks::remote_calendar::{
    CalendarApi, CalendarSpec, LocalEvent, RemoteChanges, RemoteEvent, SyncWindow,
};

use aphel_kernel::Ctx;
use aphel_kernel::errors::AppError;
use aphel_sdk::calendar::BusyInterval;
use serde_json::{Value, json};

use crate::{CALENDAR_CONNECTED, CALENDAR_DISCONNECTED, CALENDAR_SYNC, KEY};

pub(crate) const AUTHORIZATION_ENDPOINT: &str = "https://accounts.google.com/o/oauth2/v2/auth";
pub(crate) const TOKEN_ENDPOINT: &str = "https://oauth2.googleapis.com/token";
pub(crate) const ISSUERS: [&str; 2] = ["https://accounts.google.com", "accounts.google.com"];
pub(crate) const CALENDAR_SCOPE: &str = "https://www.googleapis.com/auth/calendar.events";
const EVENTS_URL: &str = "https://www.googleapis.com/calendar/v3/calendars/primary/events";
const PAGE_SIZE: &str = "250";
const MAX_PAGES: usize = 20;
const BUSY_FIELDS: &str =
    "items(status,transparency,start,end,attendees(self,responseStatus)),nextPageToken";

pub(crate) struct GoogleProvider;

#[async_trait]
impl IdentityProvider for GoogleProvider {
    fn key(&self) -> &'static str {
        KEY
    }

    fn label(&self, _settings: &Value) -> String {
        "Google".to_owned()
    }

    async fn endpoints(&self, _settings: &Value) -> Result<ProviderEndpoints, AppError> {
        Ok(ProviderEndpoints {
            issuer: Some(ISSUERS[0].to_owned()),
            authorization_endpoint: AUTHORIZATION_ENDPOINT.to_owned(),
            token_endpoint: TOKEN_ENDPOINT.to_owned(),
        })
    }

    fn login_params(&self) -> Vec<(&'static str, String)> {
        vec![("prompt", "select_account".to_owned())]
    }

    fn issuer_allowed(
        &self,
        issuer: &str,
        _endpoints: &ProviderEndpoints,
        _settings: &Value,
    ) -> bool {
        ISSUERS.contains(&issuer)
    }

    fn verified_email(
        &self,
        claims: &IdTokenClaims,
        _settings: &Value,
    ) -> Result<String, AppError> {
        if !claims.flag("email_verified") {
            return Err(failure(
                EMAIL_NOT_VERIFIED,
                "Google did not confirm the e-mail",
            ));
        }
        claims
            .text("email")
            .map(str::to_owned)
            .ok_or_else(|| failure(EMAIL_NOT_VERIFIED, "the id token has no e-mail"))
    }

    async fn complete_purpose(&self, ctx: &Ctx<'_>, state: &StoredState, code: &str) -> Response {
        if state.purpose == PURPOSE_MAIL {
            return email::oauth::connect::complete(ctx, self, state, code).await;
        }
        complete_connection(ctx, self, state, code).await
    }
}

fn rfc3339(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(SecondsFormat::Secs, true)
}

fn event_time(value: Option<&Value>) -> (Option<DateTime<Utc>>, bool) {
    let Some(value) = value else {
        return (None, false);
    };
    if let Some(instant) = parse_instant(value.get("dateTime")) {
        return (Some(instant), false);
    }
    let date = value
        .get("date")
        .and_then(Value::as_str)
        .and_then(|date| NaiveDate::parse_from_str(date, "%Y-%m-%d").ok())
        .and_then(|date| date.and_hms_opt(0, 0, 0))
        .map(|date| date.and_utc());
    (date, date.is_some())
}

fn remote_event(item: &Value) -> Option<RemoteEvent> {
    let id = item.get("id").and_then(Value::as_str)?.to_owned();
    let text = |key: &str| item.get(key).and_then(Value::as_str).map(str::to_owned);
    let (start, all_day) = event_time(item.get("start"));
    let (end, _) = event_time(item.get("end"));
    let attendees = item
        .get("attendees")
        .and_then(Value::as_array)
        .map(|attendees| {
            attendees
                .iter()
                .filter(|attendee| {
                    !attendee
                        .get("self")
                        .and_then(Value::as_bool)
                        .unwrap_or(false)
                        && !attendee
                            .get("resource")
                            .and_then(Value::as_bool)
                            .unwrap_or(false)
                })
                .filter_map(|attendee| attendee.get("email").and_then(Value::as_str))
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default();
    Some(RemoteEvent {
        id,
        etag: text("etag"),
        updated_at: parse_instant(item.get("updated")),
        cancelled: text("status").as_deref() == Some("cancelled"),
        all_day,
        title: text("summary").unwrap_or_default(),
        description: text("description"),
        start,
        end,
        attendees,
    })
}

fn event_body(event: &LocalEvent) -> Value {
    json!({
        "summary": event.title,
        "description": event.description,
        "start": { "dateTime": rfc3339(event.start) },
        "end": { "dateTime": rfc3339(event.end) },
    })
}

fn event_url(remote_id: &str) -> String {
    format!("{EVENTS_URL}/{}", encode_segment(remote_id))
}

fn written(value: Value) -> Result<RemoteEvent, AppError> {
    remote_event(&value).ok_or_else(|| internal("Google Calendar answered without an event"))
}

#[async_trait]
impl CalendarApi for GoogleProvider {
    fn spec(&self) -> CalendarSpec {
        CalendarSpec {
            key: KEY,
            permission: CALENDAR_SYNC,
            connected_event: CALENDAR_CONNECTED,
            disconnected_event: CALENDAR_DISCONNECTED,
        }
    }

    fn consent_scopes(&self) -> &'static [&'static str] {
        &["openid", "email", CALENDAR_SCOPE]
    }

    fn consent_params(&self) -> Vec<(&'static str, String)> {
        vec![
            ("access_type", "offline".to_owned()),
            ("prompt", "consent".to_owned()),
            ("include_granted_scopes", "true".to_owned()),
        ]
    }

    async fn changes(
        &self,
        access_token: &str,
        cursor: Option<&str>,
        window: SyncWindow,
    ) -> Result<RemoteChanges, AppError> {
        let client = http_client()?;
        let mut changes = RemoteChanges::default();
        let mut page_token: Option<String> = None;
        for _ in 0..MAX_PAGES {
            let mut query: Vec<(&str, String)> = vec![
                ("singleEvents", "true".to_owned()),
                ("showDeleted", "true".to_owned()),
                ("maxResults", PAGE_SIZE.to_owned()),
            ];
            match cursor {
                Some(cursor) => query.push(("syncToken", cursor.to_owned())),
                None => {
                    query.push(("timeMin", rfc3339(window.start)));
                    query.push(("timeMax", rfc3339(window.end)));
                }
            }
            if let Some(token) = &page_token {
                query.push(("pageToken", token.clone()));
            }
            let page = send(
                client
                    .get(EVENTS_URL)
                    .bearer_auth(access_token)
                    .query(&query),
                cursor.is_some(),
            )
            .await?;
            changes.events.extend(
                page.get("items")
                    .and_then(Value::as_array)
                    .map(|items| items.iter().filter_map(remote_event).collect::<Vec<_>>())
                    .unwrap_or_default(),
            );
            if let Some(next_sync) = page.get("nextSyncToken").and_then(Value::as_str) {
                changes.cursor = Some(next_sync.to_owned());
            }
            page_token = page
                .get("nextPageToken")
                .and_then(Value::as_str)
                .map(str::to_owned);
            if page_token.is_none() {
                break;
            }
        }
        if page_token.is_some() {
            changes.cursor = None;
        }
        Ok(changes)
    }

    async fn create_event(
        &self,
        access_token: &str,
        event: &LocalEvent,
    ) -> Result<RemoteEvent, AppError> {
        let value = send(
            http_client()?
                .post(EVENTS_URL)
                .bearer_auth(access_token)
                .query(&[("sendUpdates", "none")])
                .json(&event_body(event)),
            false,
        )
        .await?;
        written(value)
    }

    async fn update_event(
        &self,
        access_token: &str,
        remote_id: &str,
        event: &LocalEvent,
    ) -> Result<RemoteEvent, AppError> {
        let value = send(
            http_client()?
                .patch(event_url(remote_id))
                .bearer_auth(access_token)
                .query(&[("sendUpdates", "none")])
                .json(&event_body(event)),
            false,
        )
        .await?;
        written(value)
    }

    async fn delete_event(&self, access_token: &str, remote_id: &str) -> Result<(), AppError> {
        send(
            http_client()?
                .delete(event_url(remote_id))
                .bearer_auth(access_token)
                .query(&[("sendUpdates", "none")]),
            false,
        )
        .await
        .map(|_| ())
    }

    async fn busy(
        &self,
        access_token: &str,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<Vec<BusyInterval>, AppError> {
        let client = http_client()?;
        let mut busy = Vec::new();
        let mut page_token: Option<String> = None;
        for _ in 0..MAX_PAGES {
            let mut query: Vec<(&str, String)> = vec![
                ("singleEvents", "true".to_owned()),
                ("timeMin", rfc3339(from)),
                ("timeMax", rfc3339(to)),
                ("maxResults", PAGE_SIZE.to_owned()),
                ("fields", BUSY_FIELDS.to_owned()),
            ];
            if let Some(token) = &page_token {
                query.push(("pageToken", token.clone()));
            }
            let page = send(
                client
                    .get(EVENTS_URL)
                    .bearer_auth(access_token)
                    .query(&query),
                false,
            )
            .await?;
            busy.extend(
                page.get("items")
                    .and_then(Value::as_array)
                    .map(|items| items.iter().filter_map(busy_interval).collect::<Vec<_>>())
                    .unwrap_or_default(),
            );
            page_token = page
                .get("nextPageToken")
                .and_then(Value::as_str)
                .map(str::to_owned);
            if page_token.is_none() {
                break;
            }
        }
        Ok(busy)
    }
}

fn busy_interval(item: &Value) -> Option<BusyInterval> {
    let text = |key: &str| item.get(key).and_then(Value::as_str);
    if text("status") == Some("cancelled") || text("transparency") == Some("transparent") {
        return None;
    }
    let declined = item
        .get("attendees")
        .and_then(Value::as_array)
        .is_some_and(|attendees| {
            attendees.iter().any(|attendee| {
                attendee.get("self").and_then(Value::as_bool) == Some(true)
                    && attendee.get("responseStatus").and_then(Value::as_str) == Some("declined")
            })
        });
    if declined {
        return None;
    }
    let (Some(starts_at), _) = event_time(item.get("start")) else {
        return None;
    };
    let (Some(ends_at), _) = event_time(item.get("end")) else {
        return None;
    };
    (ends_at > starts_at).then(|| BusyInterval::new(starts_at, ends_at))
}
