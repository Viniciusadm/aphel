use aphel_sdk::email::mime::{build, encode_base64, parse_remote};
use aphel_sdk::email::oauth::connect_path;
use aphel_sdk::email::oauth::{
    MAIL_API_FAILED, MAIL_CURSOR_EXPIRED, MAIL_NOT_FOUND, MailOAuth, fetch_concurrently,
};
use aphel_sdk::email::oauth::{bytes, json, text};
use aphel_sdk::email::oauth::{with_access, with_account_access};
use aphel_sdk::sso::oauth::http_client;
use aphel_sdk::tasks::remote_calendar::encode_segment;
use async_trait::async_trait;
use chrono::{DateTime, SecondsFormat, Utc};

use aphel_kernel::errors::AppError;
use aphel_sdk::email::mailbox::{
    ConnectedMailbox, FetchedEmail, MailProviderError, MailboxAccount, MailboxConnectRequest,
    MailboxConnectionKind, MailboxCredentials, MailboxProvider, ProviderEmail, SentEmail,
    SyncBatch, SyncRequest,
};
use aphel_sdk::email::oauth::provider_error;
use aphel_sdk::sso::errors::error_code;
use serde_json::{Map, Value, json};

use crate::KEY;
use crate::provider::{GRAPH_URL, MicrosoftProvider};

pub(crate) const QUALIFIED_KEY: &str = "microsoft.outlook";
pub(crate) const READ_SCOPE: &str = "https://graph.microsoft.com/Mail.Read";
pub(crate) const SEND_SCOPE: &str = "https://graph.microsoft.com/Mail.Send";
pub(crate) const PROFILE_SCOPE: &str = "https://graph.microsoft.com/User.Read";
const REFRESH_SCOPES: &str = "openid email profile offline_access https://graph.microsoft.com/Mail.Read https://graph.microsoft.com/Mail.Send https://graph.microsoft.com/User.Read";
const GRAPH_ORIGIN: &str = "https://graph.microsoft.com/";
const FOLDER_INBOX: &str = "inbox";
const FOLDER_SENT: &str = "sentitems";
const FOLDERS: [&str; 2] = [FOLDER_INBOX, FOLDER_SENT];
const MAX_PAGE: usize = 100;
const MAX_PAGES_PER_FOLDER: usize = 20;

impl MailOAuth for MicrosoftProvider {
    fn mail_provider_key(&self) -> &'static str {
        QUALIFIED_KEY
    }

    fn mail_scopes(&self) -> &'static [&'static str] {
        &[
            "openid",
            "email",
            "profile",
            "offline_access",
            READ_SCOPE,
            SEND_SCOPE,
            PROFILE_SCOPE,
        ]
    }

    fn mail_consent_params(&self) -> Vec<(&'static str, String)> {
        vec![("prompt", "consent".to_owned())]
    }

    fn mail_refresh_scopes(&self) -> Option<&'static str> {
        Some(REFRESH_SCOPES)
    }
}

#[derive(Debug, Clone, Default)]
struct FolderState {
    link: Option<String>,
    delta: bool,
}

impl FolderState {
    fn parse(value: Option<&Value>) -> Self {
        let Some(value) = value else {
            return Self::default();
        };
        Self {
            link: text(value, "link")
                .filter(|link| link.starts_with(GRAPH_ORIGIN))
                .map(str::to_owned),
            delta: value.get("delta").and_then(Value::as_bool).unwrap_or(false),
        }
    }

    fn to_value(&self) -> Value {
        json!({ "link": self.link, "delta": self.delta })
    }
}

struct FolderPage {
    state: FolderState,
    ids: Vec<String>,
}

fn initial_link(folder: &str, since: DateTime<Utc>) -> String {
    let filter = format!(
        "receivedDateTime ge {}",
        since.to_rfc3339_opts(SecondsFormat::Secs, true)
    );
    format!(
        "{GRAPH_URL}/me/mailFolders/{folder}/messages/delta?$select=id&$filter={}",
        encode_segment(&filter)
    )
}

async fn profile(token: &str) -> Result<Value, AppError> {
    json(
        http_client()?
            .get(format!("{GRAPH_URL}/me"))
            .bearer_auth(token)
            .query(&[("$select", "mail,userPrincipalName,displayName")]),
    )
    .await
}

fn profile_address(profile: &Value) -> Result<String, AppError> {
    text(profile, "mail")
        .or_else(|| text(profile, "userPrincipalName"))
        .filter(|address| address.contains('@'))
        .map(str::to_lowercase)
        .ok_or_else(|| {
            AppError::validation_code(
                MAIL_API_FAILED,
                "Microsoft Graph answered without a mailbox address",
            )
        })
}

async fn delta_page(token: &str, link: &str, page_size: usize) -> Result<Value, AppError> {
    json(
        http_client()?
            .get(link)
            .bearer_auth(token)
            .header("Prefer", format!("odata.maxpagesize={page_size}")),
    )
    .await
}

async fn folder_page(
    token: &str,
    folder: &str,
    state: &FolderState,
    since: DateTime<Utc>,
    budget: usize,
) -> Result<FolderPage, AppError> {
    let page_size = budget.clamp(1, MAX_PAGE);
    let mut link = state
        .link
        .clone()
        .unwrap_or_else(|| initial_link(folder, since));
    let mut resumable = state.link.is_some();
    let mut ids = Vec::new();
    let mut delta = false;
    for _ in 0..MAX_PAGES_PER_FOLDER {
        let page = match delta_page(token, &link, page_size).await {
            Err(error) if error_code(&error) == MAIL_CURSOR_EXPIRED && resumable => {
                resumable = false;
                link = initial_link(folder, since);
                continue;
            }
            other => other?,
        };
        ids.extend(
            page.get("value")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter(|item| item.get("@removed").is_none())
                .filter_map(|item| text(item, "id").map(str::to_owned)),
        );
        if let Some(next) =
            text(&page, "@odata.nextLink").filter(|next| next.starts_with(GRAPH_ORIGIN))
        {
            link = next.to_owned();
            if ids.len() >= budget {
                break;
            }
            continue;
        }
        match text(&page, "@odata.deltaLink").filter(|next| next.starts_with(GRAPH_ORIGIN)) {
            Some(delta_link) => {
                link = delta_link.to_owned();
                delta = true;
            }
            None => {
                return Err(AppError::validation_code(
                    MAIL_API_FAILED,
                    "Microsoft Graph answered a delta page without a continuation link",
                ));
            }
        }
        break;
    }
    Ok(FolderPage {
        state: FolderState {
            link: Some(link),
            delta,
        },
        ids,
    })
}

async fn fetch_message(
    token: &str,
    id: String,
    sent: bool,
) -> Result<Option<FetchedEmail>, AppError> {
    let raw = match bytes(
        http_client()?
            .get(format!(
                "{GRAPH_URL}/me/messages/{}/$value",
                encode_segment(&id)
            ))
            .bearer_auth(token),
    )
    .await
    {
        Ok(raw) => raw,
        Err(error) if error_code(&error) == MAIL_NOT_FOUND => return Ok(None),
        Err(error) => return Err(error),
    };
    let Some(mut mail) = parse_remote(&raw) else {
        tracing::warn!("Outlook message could not be parsed");
        return Ok(None);
    };
    mail.provider_message_id = Some(id);
    mail.sent_by_account = sent;
    Ok(Some(mail))
}

async fn sync_page(token: &str, request: &SyncRequest) -> Result<SyncBatch, AppError> {
    let previous = request.cursor.as_ref();
    let mut budget = request.limit.max(1);
    let mut cursor = Map::new();
    let mut has_more = false;
    let mut messages = Vec::new();
    for folder in FOLDERS {
        let state = FolderState::parse(previous.and_then(|cursor| cursor.get(folder)));
        if budget == 0 {
            has_more = true;
            cursor.insert(folder.to_owned(), state.to_value());
            continue;
        }
        let page = folder_page(token, folder, &state, request.since, budget).await?;
        budget = budget.saturating_sub(page.ids.len());
        has_more |= !page.state.delta;
        let sent = folder == FOLDER_SENT;
        messages.extend(fetch_concurrently(page.ids, |id| fetch_message(token, id, sent)).await?);
        cursor.insert(folder.to_owned(), page.state.to_value());
    }
    Ok(SyncBatch {
        messages,
        cursor: Value::Object(cursor),
        has_more,
        credentials: None,
    })
}

async fn send_mime(token: &str, encoded: &str) -> Result<Value, AppError> {
    json(
        http_client()?
            .post(format!("{GRAPH_URL}/me/sendMail"))
            .bearer_auth(token)
            .header(reqwest::header::CONTENT_TYPE, "text/plain")
            .body(encoded.to_owned()),
    )
    .await
}

pub(crate) struct OutlookProvider;

#[async_trait]
impl MailboxProvider for OutlookProvider {
    fn key(&self) -> &str {
        QUALIFIED_KEY
    }

    fn module(&self) -> Option<&str> {
        Some(KEY)
    }

    fn label(&self) -> &str {
        "Outlook (Microsoft 365)"
    }

    fn connection(&self) -> MailboxConnectionKind {
        MailboxConnectionKind::OAuth {
            start_path: connect_path(KEY),
        }
    }

    async fn connect(
        &self,
        ctx: &aphel_sdk::Ctx<'_>,
        request: MailboxConnectRequest,
    ) -> Result<ConnectedMailbox, MailProviderError> {
        let (profile, refreshed) = with_access(
            ctx,
            &MicrosoftProvider,
            &request.credentials,
            |token| async move { profile(&token).await },
        )
        .await
        .map_err(provider_error)?;
        Ok(ConnectedMailbox {
            address: profile_address(&profile).map_err(provider_error)?,
            display_name: text(&profile, "displayName").map(str::to_owned),
            settings: json!({}),
            credentials: refreshed.unwrap_or(request.credentials),
        })
    }

    async fn test(
        &self,
        ctx: &aphel_sdk::Ctx<'_>,
        account: &MailboxAccount,
    ) -> Result<Option<MailboxCredentials>, MailProviderError> {
        let (_, refreshed) =
            with_account_access(ctx, &MicrosoftProvider, account, |token| async move {
                profile(&token).await
            })
            .await
            .map_err(provider_error)?;
        Ok(refreshed)
    }

    async fn sync_since(
        &self,
        ctx: &aphel_sdk::Ctx<'_>,
        account: &MailboxAccount,
        request: SyncRequest,
    ) -> Result<SyncBatch, MailProviderError> {
        let request = &request;
        let (mut page, refreshed) =
            with_account_access(ctx, &MicrosoftProvider, account, |token| async move {
                sync_page(&token, request).await
            })
            .await
            .map_err(provider_error)?;
        page.credentials = refreshed;
        Ok(page)
    }

    async fn send(
        &self,
        ctx: &aphel_sdk::Ctx<'_>,
        account: &MailboxAccount,
        mail: &ProviderEmail,
    ) -> Result<SentEmail, MailProviderError> {
        let encoded = encode_base64(&build(mail).map_err(provider_error)?);
        let encoded = encoded.as_str();
        let (_, refreshed) =
            with_account_access(ctx, &MicrosoftProvider, account, |token| async move {
                send_mime(&token, encoded).await
            })
            .await
            .map_err(provider_error)?;
        Ok(SentEmail {
            message_id: mail.message_id.clone(),
            provider_message_id: None,
            credentials: refreshed,
        })
    }
}
