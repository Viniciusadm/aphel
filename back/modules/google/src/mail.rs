use std::collections::HashSet;

use aphel_sdk::sso::oauth::http_client;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use email::oauth::connect::connect_path;
use email::oauth::http::{json, text};
use email::oauth::mime::{decode_base64url, encode_base64url, parse_remote};
use email::oauth::tokens::{with_access, with_account_access};
use email::oauth::{MAIL_API_FAILED, MAIL_NOT_FOUND, MailOAuth, fetch_concurrently};

use aphel_kernel::domain::access::AccessScope;
use aphel_kernel::errors::AppError;
use aphel_sdk::email::mailbox::{
    ConnectedMailbox, FetchedEmail, MailProviderError, MailboxAccount, MailboxConnectRequest,
    MailboxConnectionKind, MailboxCredentials, MailboxProvider, ProviderEmail, SentEmail,
    SyncBatch, SyncRequest,
};
use aphel_sdk::sso::errors::error_code;
use email::oauth::{provider_ctx, provider_error, provider_state};
use serde_json::{Value, json};

use crate::KEY;
use crate::provider::GoogleProvider;

pub(crate) const QUALIFIED_KEY: &str = "google.gmail";
pub(crate) const READ_SCOPE: &str = "https://www.googleapis.com/auth/gmail.readonly";
pub(crate) const SEND_SCOPE: &str = "https://www.googleapis.com/auth/gmail.send";
const GMAIL_URL: &str = "https://gmail.googleapis.com/gmail/v1/users/me";
const MAX_PAGE: usize = 500;
const PHASE_INITIAL: &str = "initial";
const PHASE_HISTORY: &str = "history";
const SKIPPED_LABELS: [&str; 2] = ["DRAFT", "SPAM"];

impl MailOAuth for GoogleProvider {
    fn mail_provider_key(&self) -> &'static str {
        QUALIFIED_KEY
    }

    fn mail_scopes(&self) -> &'static [&'static str] {
        &["openid", "email", READ_SCOPE, SEND_SCOPE]
    }

    fn mail_consent_params(&self) -> Vec<(&'static str, String)> {
        vec![
            ("access_type", "offline".to_owned()),
            ("prompt", "consent".to_owned()),
        ]
    }
}

#[derive(Debug, Clone, Default)]
struct Cursor {
    phase: Option<String>,
    history_id: Option<String>,
    page_token: Option<String>,
}

impl Cursor {
    fn parse(value: Option<&Value>) -> Self {
        let Some(value) = value else {
            return Self::default();
        };
        let field = |key: &str| text(value, key).map(str::to_owned);
        Self {
            phase: field("phase"),
            history_id: field("history_id"),
            page_token: field("page_token"),
        }
    }

    fn to_value(&self) -> Value {
        json!({
            "phase": self.phase,
            "history_id": self.history_id,
            "page_token": self.page_token,
        })
    }
}

struct Listing {
    ids: Vec<String>,
    cursor: Cursor,
    has_more: bool,
}

fn page_size(limit: usize) -> String {
    limit.clamp(1, MAX_PAGE).to_string()
}

async fn profile(token: &str) -> Result<Value, AppError> {
    json(
        http_client()?
            .get(format!("{GMAIL_URL}/profile"))
            .bearer_auth(token),
    )
    .await
}

fn profile_address(profile: &Value) -> Result<String, AppError> {
    text(profile, "emailAddress")
        .map(str::to_lowercase)
        .ok_or_else(|| {
            AppError::validation_code(MAIL_API_FAILED, "Gmail answered without an address")
        })
}

fn history_id(value: &Value) -> Option<String> {
    match value.get("historyId")? {
        Value::String(text) => Some(text.clone()),
        Value::Number(number) => Some(number.to_string()),
        _ => None,
    }
}

async fn initial_listing(
    token: &str,
    since: DateTime<Utc>,
    limit: usize,
    cursor: Cursor,
) -> Result<Listing, AppError> {
    let start_history = match cursor.history_id.clone() {
        Some(history) => Some(history),
        None => history_id(&profile(token).await?),
    };
    let mut query: Vec<(&str, String)> = vec![
        ("q", format!("after:{}", since.timestamp())),
        ("maxResults", page_size(limit)),
    ];
    if let Some(page_token) = &cursor.page_token {
        query.push(("pageToken", page_token.clone()));
    }
    let page = json(
        http_client()?
            .get(format!("{GMAIL_URL}/messages"))
            .bearer_auth(token)
            .query(&query),
    )
    .await?;
    let ids = page
        .get("messages")
        .and_then(Value::as_array)
        .map(|messages| {
            messages
                .iter()
                .filter_map(|message| text(message, "id").map(str::to_owned))
                .collect()
        })
        .unwrap_or_default();
    let next = text(&page, "nextPageToken").map(str::to_owned);
    let has_more = next.is_some();
    Ok(Listing {
        ids,
        cursor: Cursor {
            phase: Some(
                if has_more {
                    PHASE_INITIAL
                } else {
                    PHASE_HISTORY
                }
                .to_owned(),
            ),
            history_id: start_history,
            page_token: next,
        },
        has_more,
    })
}

async fn history_listing(
    token: &str,
    limit: usize,
    cursor: &Cursor,
    start: &str,
) -> Result<Listing, AppError> {
    let mut query: Vec<(&str, String)> = vec![
        ("startHistoryId", start.to_owned()),
        ("historyTypes", "messageAdded".to_owned()),
        ("maxResults", page_size(limit)),
    ];
    if let Some(page_token) = &cursor.page_token {
        query.push(("pageToken", page_token.clone()));
    }
    let page = json(
        http_client()?
            .get(format!("{GMAIL_URL}/history"))
            .bearer_auth(token)
            .query(&query),
    )
    .await?;
    let mut seen = HashSet::new();
    let ids = page
        .get("history")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|entry| entry.get("messagesAdded").and_then(Value::as_array))
        .flatten()
        .filter_map(|added| added.get("message"))
        .filter(|message| !has_skipped_label(message))
        .filter_map(|message| text(message, "id").map(str::to_owned))
        .filter(|id| seen.insert(id.clone()))
        .collect();
    let next = text(&page, "nextPageToken").map(str::to_owned);
    let has_more = next.is_some();
    let history_id = if has_more {
        Some(start.to_owned())
    } else {
        history_id(&page).or_else(|| Some(start.to_owned()))
    };
    Ok(Listing {
        ids,
        cursor: Cursor {
            phase: Some(PHASE_HISTORY.to_owned()),
            history_id,
            page_token: next,
        },
        has_more,
    })
}

fn labels(message: &Value) -> Vec<&str> {
    message
        .get("labelIds")
        .and_then(Value::as_array)
        .map(|labels| labels.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default()
}

fn has_skipped_label(message: &Value) -> bool {
    labels(message)
        .iter()
        .any(|label| SKIPPED_LABELS.contains(label))
}

async fn fetch_message(token: &str, id: String) -> Result<Option<FetchedEmail>, AppError> {
    let message = match json(
        http_client()?
            .get(format!("{GMAIL_URL}/messages/{id}"))
            .bearer_auth(token)
            .query(&[("format", "raw")]),
    )
    .await
    {
        Ok(message) => message,
        Err(error) if error_code(&error) == MAIL_NOT_FOUND => return Ok(None),
        Err(error) => return Err(error),
    };
    if has_skipped_label(&message) {
        return Ok(None);
    }
    let Some(raw) = text(&message, "raw").and_then(decode_base64url) else {
        tracing::warn!("Gmail answered a message without a readable raw body");
        return Ok(None);
    };
    let Some(mut mail) = parse_remote(&raw) else {
        tracing::warn!("Gmail message could not be parsed");
        return Ok(None);
    };
    mail.provider_message_id = Some(id);
    mail.sent_by_account = labels(&message).contains(&"SENT");
    if mail.date.is_none() {
        mail.date = text(&message, "internalDate")
            .and_then(|millis| millis.parse::<i64>().ok())
            .and_then(DateTime::from_timestamp_millis);
    }
    Ok(Some(mail))
}

async fn sync_page(token: &str, request: &SyncRequest) -> Result<SyncBatch, AppError> {
    let cursor = Cursor::parse(request.cursor.as_ref());
    let listing = match (cursor.phase.as_deref(), cursor.history_id.as_deref()) {
        (Some(PHASE_HISTORY), Some(start)) => {
            match history_listing(token, request.limit, &cursor, start).await {
                Err(error) if error_code(&error) == MAIL_NOT_FOUND => {
                    initial_listing(token, request.since, request.limit, Cursor::default()).await?
                }
                other => other?,
            }
        }
        (Some(PHASE_INITIAL), _) => {
            initial_listing(token, request.since, request.limit, cursor).await?
        }
        _ => initial_listing(token, request.since, request.limit, Cursor::default()).await?,
    };
    let messages = fetch_concurrently(listing.ids, |id| fetch_message(token, id)).await?;
    Ok(SyncBatch {
        messages,
        cursor: listing.cursor.to_value(),
        has_more: listing.has_more,
        credentials: None,
    })
}

async fn thread_for(token: &str, message_id: &str) -> Option<String> {
    let page = json(
        http_client()
            .ok()?
            .get(format!("{GMAIL_URL}/messages"))
            .bearer_auth(token)
            .query(&[
                ("q", format!("rfc822msgid:{message_id}")),
                ("maxResults", "1".to_owned()),
                ("includeSpamTrash", "true".to_owned()),
            ]),
    )
    .await
    .ok()?;
    page.get("messages")
        .and_then(Value::as_array)
        .and_then(|messages| messages.first())
        .and_then(|message| text(message, "threadId"))
        .map(str::to_owned)
}

async fn send_raw(token: &str, raw: &str, in_reply_to: Option<&str>) -> Result<Value, AppError> {
    let thread_id = match in_reply_to {
        Some(message_id) => thread_for(token, message_id).await,
        None => None,
    };
    let mut body = json!({ "raw": raw });
    if let Some(thread_id) = thread_id {
        body["threadId"] = Value::String(thread_id);
    }
    json(
        http_client()?
            .post(format!("{GMAIL_URL}/messages/send"))
            .bearer_auth(token)
            .json(&body),
    )
    .await
}

pub(crate) struct GmailProvider;

#[async_trait]
impl MailboxProvider for GmailProvider {
    fn key(&self) -> &str {
        QUALIFIED_KEY
    }

    fn module(&self) -> Option<&str> {
        Some(KEY)
    }

    fn label(&self) -> &str {
        "Gmail"
    }

    fn connection(&self) -> MailboxConnectionKind {
        MailboxConnectionKind::OAuth {
            start_path: connect_path(KEY),
        }
    }

    async fn connect(
        &self,
        request: MailboxConnectRequest,
    ) -> Result<ConnectedMailbox, MailProviderError> {
        let state = provider_state()?;
        let scope = AccessScope::system();
        let ctx = provider_ctx(&state, &scope).await?;
        let (profile, refreshed) = with_access(
            &ctx,
            &GoogleProvider,
            &request.credentials,
            |token| async move { profile(&token).await },
        )
        .await
        .map_err(provider_error)?;
        Ok(ConnectedMailbox {
            address: profile_address(&profile).map_err(provider_error)?,
            display_name: None,
            settings: json!({}),
            credentials: refreshed.unwrap_or(request.credentials),
        })
    }

    async fn test(
        &self,
        account: &MailboxAccount,
    ) -> Result<Option<MailboxCredentials>, MailProviderError> {
        let state = provider_state()?;
        let scope = AccessScope::system();
        let ctx = provider_ctx(&state, &scope).await?;
        let (_, refreshed) =
            with_account_access(&ctx, &GoogleProvider, account, |token| async move {
                profile(&token).await
            })
            .await
            .map_err(provider_error)?;
        Ok(refreshed)
    }

    async fn sync_since(
        &self,
        account: &MailboxAccount,
        request: SyncRequest,
    ) -> Result<SyncBatch, MailProviderError> {
        let state = provider_state()?;
        let scope = AccessScope::system();
        let ctx = provider_ctx(&state, &scope).await?;
        let request = &request;
        let (mut page, refreshed) =
            with_account_access(&ctx, &GoogleProvider, account, |token| async move {
                sync_page(&token, request).await
            })
            .await
            .map_err(provider_error)?;
        page.credentials = refreshed;
        Ok(page)
    }

    async fn send(
        &self,
        account: &MailboxAccount,
        mail: &ProviderEmail,
    ) -> Result<SentEmail, MailProviderError> {
        let state = provider_state()?;
        let scope = AccessScope::system();
        let ctx = provider_ctx(&state, &scope).await?;
        let raw = encode_base64url(&email::oauth::mime::build(mail).map_err(provider_error)?);
        let raw = raw.as_str();
        let in_reply_to = mail.in_reply_to.as_deref();
        let (sent, refreshed) =
            with_account_access(&ctx, &GoogleProvider, account, |token| async move {
                send_raw(&token, raw, in_reply_to).await
            })
            .await
            .map_err(provider_error)?;
        Ok(SentEmail {
            message_id: mail.message_id.clone(),
            provider_message_id: text(&sent, "id").map(str::to_owned),
            credentials: refreshed,
        })
    }
}
