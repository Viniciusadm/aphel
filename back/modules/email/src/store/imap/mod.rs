pub(crate) mod client;
pub(crate) mod config;
pub(crate) mod parse;
pub(crate) mod smtp;

use std::collections::BTreeMap;

use async_trait::async_trait;
use futures::TryStreamExt;
use lettre::AsyncTransport;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    ConnectedMailbox, FetchedEmail, IMAP_PROVIDER, MailProviderError, MailboxAccount,
    MailboxConnectRequest, MailboxConnectionKind, MailboxCredentials, MailboxProvider,
    ProviderEmail, SentEmail, SyncBatch, SyncRequest,
};
use client::{
    COMMAND_TIMEOUT, ImapSession, close, find_sent_folder, imap_failure, open_session, timed,
};
use config::{NetworkPolicy, parse_settings, passwords, stored_credentials};

pub const INBOX: &str = "INBOX";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct FolderState {
    pub uid_validity: u32,
    pub last_uid: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ImapCursor {
    #[serde(default)]
    pub folders: BTreeMap<String, FolderState>,
    #[serde(default)]
    pub sent_folder: Option<String>,
}

impl ImapCursor {
    fn from_value(value: Option<&Value>) -> Self {
        value
            .and_then(|value| serde_json::from_value(value.clone()).ok())
            .unwrap_or_default()
    }

    fn to_value(&self) -> Value {
        serde_json::to_value(self).unwrap_or(Value::Null)
    }
}

const SELF_SAVING_SMTP_HOSTS: [&str; 4] = [
    "smtp.gmail.com",
    "smtp.googlemail.com",
    "smtp.office365.com",
    "smtp-mail.outlook.com",
];

fn saves_sent_copies(smtp_host: &str) -> bool {
    SELF_SAVING_SMTP_HOSTS
        .iter()
        .any(|host| smtp_host.trim().eq_ignore_ascii_case(host))
}

pub(crate) struct ImapMailboxProvider {
    policy: NetworkPolicy,
}

impl ImapMailboxProvider {
    pub(crate) fn new(policy: NetworkPolicy) -> Self {
        Self { policy }
    }

    pub(crate) fn from_env() -> Self {
        Self::new(NetworkPolicy::from_env())
    }

    async fn fetch_folder(
        &self,
        session: &mut ImapSession,
        folder: &str,
        sent: bool,
        state: Option<FolderState>,
        request: &SyncRequest,
        limit: usize,
    ) -> Result<(FolderState, Vec<FetchedEmail>, bool), MailProviderError> {
        let selected = timed(session.examine(folder), COMMAND_TIMEOUT, imap_failure).await?;
        let validity = selected.uid_validity.unwrap_or_default();
        let state = state.filter(|state| state.uid_validity == validity);
        let query = match state {
            Some(state) => format!("UID {}:*", state.last_uid.saturating_add(1)),
            None => format!("SINCE {}", request.since.format("%d-%b-%Y")),
        };
        let found = timed(session.uid_search(query), COMMAND_TIMEOUT, imap_failure).await?;
        let last = state.map(|state| state.last_uid).unwrap_or_default();
        let mut uids: Vec<u32> = found.into_iter().filter(|uid| *uid > last).collect();
        uids.sort_unstable();
        let has_more = uids.len() > limit;
        uids.truncate(limit);
        let next_last = match uids.last() {
            Some(uid) => *uid,
            None if state.is_some() => last,
            None => selected.uid_next.unwrap_or(1).saturating_sub(1),
        };
        let mut messages = Vec::with_capacity(uids.len());
        if !uids.is_empty() {
            let sizes = self.fetch(session, &uids, "(UID RFC822.SIZE)").await?;
            let (large, small): (Vec<u32>, Vec<u32>) = uids.iter().partition(|uid| {
                sizes.iter().any(|(id, size, _)| {
                    id == *uid && size.unwrap_or_default() > self.policy.max_message_bytes
                })
            });
            let mut contents = self.fetch(session, &small, "(UID BODY.PEEK[])").await?;
            contents.extend(
                self.fetch(session, &large, "(UID BODY.PEEK[HEADER])")
                    .await?,
            );
            for uid in &uids {
                let Some((_, _, Some(bytes))) = contents.iter().find(|(id, _, _)| id == uid) else {
                    continue;
                };
                let provider_id = format!("{folder}:{validity}:{uid}");
                if let Some(message) =
                    parse::parse_message(bytes, provider_id, sent, large.contains(uid))
                {
                    messages.push(message);
                }
            }
        }
        Ok((
            FolderState {
                uid_validity: validity,
                last_uid: next_last,
            },
            messages,
            has_more,
        ))
    }

    async fn append_to_sent(
        &self,
        settings: &config::ImapSettings,
        password: &str,
        raw: &[u8],
    ) -> Result<(), MailProviderError> {
        let mut session = open_session(settings, password, &self.policy).await?;
        let folder = match &settings.sent_folder {
            Some(folder) => Some(folder.clone()),
            None => find_sent_folder(&mut session).await.unwrap_or_default(),
        };
        let result = match folder {
            Some(folder) => {
                timed(
                    session.append(&folder, Some("(\\Seen)"), None, raw),
                    COMMAND_TIMEOUT,
                    imap_failure,
                )
                .await
            }
            None => Err(MailProviderError::new(
                crate::MailProviderErrorKind::Unsupported,
                "the mailbox has no sent folder",
            )),
        };
        close(session).await;
        result
    }

    async fn fetch(
        &self,
        session: &mut ImapSession,
        uids: &[u32],
        query: &str,
    ) -> Result<Vec<(u32, Option<u32>, Option<Vec<u8>>)>, MailProviderError> {
        if uids.is_empty() {
            return Ok(Vec::new());
        }
        let set = uids
            .iter()
            .map(u32::to_string)
            .collect::<Vec<_>>()
            .join(",");
        let stream = timed(session.uid_fetch(set, query), COMMAND_TIMEOUT, imap_failure).await?;
        let fetches = timed(
            stream.try_collect::<Vec<_>>(),
            COMMAND_TIMEOUT,
            imap_failure,
        )
        .await?;
        Ok(fetches
            .iter()
            .filter_map(|fetch| {
                let uid = fetch.uid?;
                let content = fetch.body().or_else(|| fetch.header()).map(<[u8]>::to_vec);
                Some((uid, fetch.size, content))
            })
            .collect())
    }
}

#[async_trait]
impl MailboxProvider for ImapMailboxProvider {
    fn key(&self) -> &str {
        IMAP_PROVIDER
    }

    fn label(&self) -> &str {
        "IMAP/SMTP"
    }

    fn connection(&self) -> MailboxConnectionKind {
        MailboxConnectionKind::Credentials
    }

    async fn connect(
        &self,
        _ctx: &aphel_kernel::Ctx<'_>,
        request: MailboxConnectRequest,
    ) -> Result<ConnectedMailbox, MailProviderError> {
        let settings = parse_settings(&request.settings)?;
        let secrets = passwords(&request.credentials)?;
        let mut session = open_session(&settings, &secrets.imap, &self.policy).await?;
        let mut settings = settings;
        if settings.sent_folder.is_none() {
            settings.sent_folder = find_sent_folder(&mut session).await.unwrap_or_default();
        }
        timed(session.examine(INBOX), COMMAND_TIMEOUT, imap_failure).await?;
        close(session).await;
        smtp::verify(&settings, &secrets.smtp, &self.policy).await?;
        Ok(ConnectedMailbox {
            address: settings.address.clone(),
            display_name: None,
            settings: settings.to_value(),
            credentials: stored_credentials(&request.credentials),
        })
    }

    async fn test(
        &self,
        _ctx: &aphel_kernel::Ctx<'_>,
        account: &MailboxAccount,
    ) -> Result<Option<MailboxCredentials>, MailProviderError> {
        let settings = parse_settings(&account.settings)?;
        let secrets = passwords(&account.credentials)?;
        let mut session = open_session(&settings, &secrets.imap, &self.policy).await?;
        timed(session.examine(INBOX), COMMAND_TIMEOUT, imap_failure).await?;
        close(session).await;
        smtp::verify(&settings, &secrets.smtp, &self.policy).await?;
        Ok(None)
    }

    async fn sync_since(
        &self,
        _ctx: &aphel_kernel::Ctx<'_>,
        account: &MailboxAccount,
        request: SyncRequest,
    ) -> Result<SyncBatch, MailProviderError> {
        let settings = parse_settings(&account.settings)?;
        let secrets = passwords(&account.credentials)?;
        let mut cursor = ImapCursor::from_value(request.cursor.as_ref());
        let mut session = open_session(&settings, &secrets.imap, &self.policy).await?;
        if cursor.sent_folder.is_none() {
            cursor.sent_folder = match &settings.sent_folder {
                Some(folder) => Some(folder.clone()),
                None => find_sent_folder(&mut session).await.unwrap_or_default(),
            };
        }
        let mut folders = vec![(INBOX.to_owned(), false)];
        if let Some(sent) = cursor
            .sent_folder
            .clone()
            .filter(|folder| !folder.eq_ignore_ascii_case(INBOX))
        {
            folders.push((sent, true));
        }
        let mut messages = Vec::new();
        let mut has_more = false;
        let limit = request.limit.max(1);
        for (folder, sent) in folders {
            let remaining = limit.saturating_sub(messages.len());
            if remaining == 0 {
                has_more = true;
                break;
            }
            let state = cursor.folders.get(&folder).copied();
            let result = self
                .fetch_folder(&mut session, &folder, sent, state, &request, remaining)
                .await;
            let (state, fetched, more) = match result {
                Ok(result) => result,
                Err(error) if sent => {
                    tracing::warn!("could not read the sent folder: {error}");
                    continue;
                }
                Err(error) => {
                    close(session).await;
                    return Err(error);
                }
            };
            cursor.folders.insert(folder, state);
            messages.extend(fetched);
            has_more |= more;
        }
        close(session).await;
        Ok(SyncBatch {
            messages,
            cursor: cursor.to_value(),
            has_more,
            credentials: None,
        })
    }

    async fn send(
        &self,
        _ctx: &aphel_kernel::Ctx<'_>,
        account: &MailboxAccount,
        email: &ProviderEmail,
    ) -> Result<SentEmail, MailProviderError> {
        let settings = parse_settings(&account.settings)?;
        let secrets = passwords(&account.credentials)?;
        let message = smtp::build_message(email)?;
        let raw = message.formatted();
        let transport = smtp::transport(&settings, &secrets.smtp, &self.policy).await?;
        timed(transport.send(message), smtp::SMTP_TIMEOUT * 2, |error| {
            smtp::smtp_failure(&error)
        })
        .await?;
        if !saves_sent_copies(&settings.smtp_host)
            && let Err(error) = self.append_to_sent(&settings, &secrets.imap, &raw).await
        {
            tracing::warn!(
                mailbox_id = %account.mailbox_id,
                "could not store the sent message in the sent folder: {error}"
            );
        }
        Ok(SentEmail {
            message_id: email.message_id.trim().trim_matches(['<', '>']).to_owned(),
            provider_message_id: None,
            credentials: None,
        })
    }
}
