use async_trait::async_trait;
use axum::response::Response;
use chrono::{DateTime, NaiveDateTime, Utc};
use email::oauth::connect::PURPOSE_MAIL;
use sso::errors::internal;
use sso::sso::login::{EMAIL_NOT_VERIFIED, failure};
use sso::sso::oauth::http_client;
use sso::sso::settings::{self, LoginSettings};
use sso::sso::{IdTokenClaims, IdentityProvider, ProviderEndpoints, StoredState};
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

use crate::{
    CALENDAR_CONNECTED, CALENDAR_DISCONNECTED, CALENDAR_SYNC, DEFAULT_TENANT, KEY, TENANT,
};

pub(crate) const LOGIN_HOST: &str = "https://login.microsoftonline.com";
pub(crate) const GRAPH_URL: &str = "https://graph.microsoft.com/v1.0";
pub(crate) const CALENDAR_SCOPE: &str = "https://graph.microsoft.com/Calendars.ReadWrite";
const REFRESH_SCOPES: &str =
    "openid email profile offline_access https://graph.microsoft.com/Calendars.ReadWrite";
const PREFER: &str =
    "odata.maxpagesize=100, outlook.timezone=\"UTC\", outlook.body-content-type=\"text\"";
const MAX_PAGES: usize = 50;
const MULTI_TENANT: [&str; 3] = ["common", "organizations", "consumers"];

pub(crate) fn is_valid_tenant(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 100
        && value.chars().all(|character| {
            character.is_ascii_alphanumeric() || character == '-' || character == '.'
        })
}

pub(crate) fn tenant(settings: &Value) -> String {
    settings::text(settings, TENANT)
        .filter(|value| is_valid_tenant(value))
        .unwrap_or_else(|| DEFAULT_TENANT.to_owned())
}

fn is_guid(value: &str) -> bool {
    value.len() == 36
        && value
            .chars()
            .enumerate()
            .all(|(index, character)| match index {
                8 | 13 | 18 | 23 => character == '-',
                _ => character.is_ascii_hexdigit(),
            })
}

pub(crate) struct MicrosoftProvider;

#[async_trait]
impl IdentityProvider for MicrosoftProvider {
    fn key(&self) -> &'static str {
        KEY
    }

    fn label(&self, _settings: &Value) -> String {
        "Microsoft".to_owned()
    }

    async fn endpoints(&self, settings: &Value) -> Result<ProviderEndpoints, AppError> {
        let tenant = tenant(settings);
        Ok(ProviderEndpoints {
            issuer: None,
            authorization_endpoint: format!("{LOGIN_HOST}/{tenant}/oauth2/v2.0/authorize"),
            token_endpoint: format!("{LOGIN_HOST}/{tenant}/oauth2/v2.0/token"),
        })
    }

    fn login_params(&self) -> Vec<(&'static str, String)> {
        vec![("prompt", "select_account".to_owned())]
    }

    fn issuer_allowed(
        &self,
        issuer: &str,
        _endpoints: &ProviderEndpoints,
        settings: &Value,
    ) -> bool {
        let Some(tenant_id) = issuer
            .strip_prefix(LOGIN_HOST)
            .and_then(|rest| rest.strip_prefix('/'))
            .and_then(|rest| rest.strip_suffix("/v2.0"))
        else {
            return false;
        };
        if !is_guid(tenant_id) {
            return false;
        }
        let configured = tenant(settings);
        if is_guid(&configured) {
            configured.eq_ignore_ascii_case(tenant_id)
        } else {
            MULTI_TENANT.contains(&configured.as_str()) || configured.contains('.')
        }
    }

    fn verified_email(&self, claims: &IdTokenClaims, settings: &Value) -> Result<String, AppError> {
        if claims.flag("xms_edov")
            && let Some(email) = claims.text("email")
        {
            return Ok(email.to_owned());
        }
        let login = LoginSettings::from_value(settings);
        claims
            .text("preferred_username")
            .filter(|username| !login.domains.is_empty() && login.matches_email(username))
            .map(str::to_owned)
            .ok_or_else(|| {
                failure(
                    EMAIL_NOT_VERIFIED,
                    "Microsoft did not confirm the e-mail and the account is outside the configured domains",
                )
            })
    }

    async fn complete_purpose(&self, ctx: &Ctx<'_>, state: &StoredState, code: &str) -> Response {
        if state.purpose == PURPOSE_MAIL {
            return email::oauth::connect::complete(ctx, self, state, code).await;
        }
        complete_connection(ctx, self, state, code).await
    }
}

fn graph_time(value: Option<&Value>) -> Option<DateTime<Utc>> {
    let text = value?.get("dateTime")?.as_str()?;
    NaiveDateTime::parse_from_str(text, "%Y-%m-%dT%H:%M:%S%.f")
        .ok()
        .map(|naive| naive.and_utc())
        .or_else(|| parse_instant(Some(&Value::String(text.to_owned()))))
}

fn graph_instant(value: DateTime<Utc>) -> Value {
    json!({ "dateTime": value.format("%Y-%m-%dT%H:%M:%S").to_string(), "timeZone": "UTC" })
}

fn remote_event(item: &Value) -> Option<RemoteEvent> {
    let id = item.get("id").and_then(Value::as_str)?.to_owned();
    let text = |key: &str| item.get(key).and_then(Value::as_str).map(str::to_owned);
    let removed = item.get("@removed").is_some();
    let attendees = item
        .get("attendees")
        .and_then(Value::as_array)
        .map(|attendees| {
            attendees
                .iter()
                .filter(|attendee| attendee.get("type").and_then(Value::as_str) != Some("resource"))
                .filter_map(|attendee| {
                    attendee
                        .get("emailAddress")
                        .and_then(|address| address.get("address"))
                        .and_then(Value::as_str)
                })
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default();
    let description = item
        .get("body")
        .and_then(|body| body.get("content"))
        .and_then(Value::as_str)
        .map(str::to_owned)
        .or_else(|| text("bodyPreview"));
    Some(RemoteEvent {
        id,
        etag: text("changeKey"),
        updated_at: parse_instant(item.get("lastModifiedDateTime")),
        cancelled: removed
            || item
                .get("isCancelled")
                .and_then(Value::as_bool)
                .unwrap_or(false),
        all_day: item
            .get("isAllDay")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        title: text("subject").unwrap_or_default(),
        description,
        start: graph_time(item.get("start")),
        end: graph_time(item.get("end")),
        attendees,
    })
}

fn event_body(event: &LocalEvent) -> Value {
    json!({
        "subject": event.title,
        "body": { "contentType": "text", "content": event.description.clone().unwrap_or_default() },
        "start": graph_instant(event.start),
        "end": graph_instant(event.end),
    })
}

fn event_url(remote_id: &str) -> String {
    format!("{GRAPH_URL}/me/events/{}", encode_segment(remote_id))
}

fn written(value: Value) -> Result<RemoteEvent, AppError> {
    remote_event(&value).ok_or_else(|| internal("Microsoft Graph answered without an event"))
}

fn trusted_link(url: &str) -> bool {
    url.starts_with("https://graph.microsoft.com/")
}

#[async_trait]
impl CalendarApi for MicrosoftProvider {
    fn spec(&self) -> CalendarSpec {
        CalendarSpec {
            key: KEY,
            permission: CALENDAR_SYNC,
            connected_event: CALENDAR_CONNECTED,
            disconnected_event: CALENDAR_DISCONNECTED,
        }
    }

    fn consent_scopes(&self) -> &'static [&'static str] {
        &[
            "openid",
            "email",
            "profile",
            "offline_access",
            CALENDAR_SCOPE,
        ]
    }

    fn consent_params(&self) -> Vec<(&'static str, String)> {
        vec![("prompt", "consent".to_owned())]
    }

    fn refresh_scopes(&self) -> Option<&'static str> {
        Some(REFRESH_SCOPES)
    }

    async fn changes(
        &self,
        access_token: &str,
        cursor: Option<&str>,
        window: SyncWindow,
    ) -> Result<RemoteChanges, AppError> {
        let client = http_client()?;
        let mut changes = RemoteChanges::default();
        let mut next = match cursor.filter(|cursor| trusted_link(cursor)) {
            Some(cursor) => cursor.to_owned(),
            None => {
                let query = form_urlencoded::Serializer::new(String::new())
                    .append_pair(
                        "startDateTime",
                        &window.start.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
                    )
                    .append_pair(
                        "endDateTime",
                        &window.end.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
                    )
                    .finish();
                format!("{GRAPH_URL}/me/calendarView/delta?{query}")
            }
        };
        let incremental = cursor.is_some_and(trusted_link);
        for _ in 0..MAX_PAGES {
            let page = send(
                client
                    .get(&next)
                    .bearer_auth(access_token)
                    .header("Prefer", PREFER),
                incremental,
            )
            .await?;
            changes.events.extend(
                page.get("value")
                    .and_then(Value::as_array)
                    .map(|items| items.iter().filter_map(remote_event).collect::<Vec<_>>())
                    .unwrap_or_default(),
            );
            if let Some(delta) = page.get("@odata.deltaLink").and_then(Value::as_str) {
                if trusted_link(delta) {
                    changes.cursor = Some(delta.to_owned());
                }
                break;
            }
            match page.get("@odata.nextLink").and_then(Value::as_str) {
                Some(link) if trusted_link(link) => next = link.to_owned(),
                _ => break,
            }
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
                .post(format!("{GRAPH_URL}/me/events"))
                .bearer_auth(access_token)
                .header("Prefer", PREFER)
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
                .header("Prefer", PREFER)
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
                .bearer_auth(access_token),
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
        let query = form_urlencoded::Serializer::new(String::new())
            .append_pair(
                "startDateTime",
                &from.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
            )
            .append_pair("endDateTime", &to.format("%Y-%m-%dT%H:%M:%SZ").to_string())
            .append_pair("$select", "start,end,showAs,isCancelled")
            .append_pair("$top", "100")
            .finish();
        let mut next = format!("{GRAPH_URL}/me/calendarView?{query}");
        let mut busy = Vec::new();
        for _ in 0..MAX_PAGES {
            let page = send(
                client
                    .get(&next)
                    .bearer_auth(access_token)
                    .header("Prefer", PREFER),
                false,
            )
            .await?;
            busy.extend(
                page.get("value")
                    .and_then(Value::as_array)
                    .map(|items| items.iter().filter_map(busy_interval).collect::<Vec<_>>())
                    .unwrap_or_default(),
            );
            match page.get("@odata.nextLink").and_then(Value::as_str) {
                Some(link) if trusted_link(link) => next = link.to_owned(),
                _ => break,
            }
        }
        Ok(busy)
    }
}

fn busy_interval(item: &Value) -> Option<BusyInterval> {
    if item
        .get("isCancelled")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return None;
    }
    if matches!(
        item.get("showAs").and_then(Value::as_str),
        Some("free" | "workingElsewhere")
    ) {
        return None;
    }
    let starts_at = graph_time(item.get("start"))?;
    let ends_at = graph_time(item.get("end"))?;
    (ends_at > starts_at).then(|| BusyInterval::new(starts_at, ends_at))
}
