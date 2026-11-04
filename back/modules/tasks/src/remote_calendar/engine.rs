use aphel_contract_crm::CrmRecordTypes;
use std::collections::BTreeSet;

use chrono::{DateTime, Duration, SecondsFormat, Utc};
use serde_json::{Value, json};
use uuid::Uuid;

use super::model::{
    AUTHORIZATION_REVOKED, CURSOR_EXPIRED, CalendarApi, DEFAULT_MEETING_TYPE, EVENT_FAILED,
    IMPORT_ALL, IMPORT_SCOPE, LocalEvent, MEETING_TYPE, MEETING_TYPE_NOT_FOUND, MEMBER_UNAVAILABLE,
    NOT_CONNECTED, PERMISSION_REVOKED, REMOTE_NOT_FOUND, RemoteEvent, STATUS_ERROR, STATUS_REVOKED,
    SyncWindow,
};
use super::store::{self, Connection, Link, LinkWrite, SyncOutcome};
use crate::domain::{
    ActivityRecord, CreateActivityCommand, RecordLinkInput, UpdateActivityCommand,
};
use crate::{service as tasks, types as task_types};
use aphel_kernel::Ctx;
use aphel_kernel::application::organization_admin::ListWindow;
use aphel_kernel::application::query::{Filter, FilterOperator, RecordQuery, RecordQueryEngine};
use aphel_kernel::application::shared::{Job, JobQueue, RecordType};
use aphel_kernel::domain::access::AccessScope;
use aphel_kernel::errors::AppError;
use aphel_sdk::sso::oauth::{self, INVALID_GRANT};
use sso::errors::{error_code, internal, is_internal, is_not_found};
use sso::sso::login::credentials;
use sso::sso::settings;
use sso::support;

pub(crate) const PAST_DAYS: i64 = 30;
pub(crate) const FUTURE_DAYS: i64 = 90;
pub(crate) const MAX_EVENTS_PER_RUN: usize = 5000;
pub(crate) const MAX_ATTENDEES: usize = 20;
pub(crate) const DEFAULT_DURATION_MINUTES: i64 = 30;
pub(crate) const MAX_DURATION_MINUTES: i64 = 7 * 24 * 60;
pub(crate) const MAX_TITLE: usize = 255;
pub(crate) const MAX_DESCRIPTION: usize = 5000;
pub(crate) const MAX_REMOTE_ID: usize = 255;
pub(crate) const PUSH_DELAY_SECONDS: i64 = 15;

const CONTACT: &str = "people.person";
const MAX_ACTIVITY_TYPES: u64 = 1000;

pub(crate) fn refresh_token_name(user_id: Uuid) -> String {
    format!("user:{user_id}:refresh_token")
}

pub(crate) fn window(now: DateTime<Utc>) -> SyncWindow {
    SyncWindow {
        start: now - Duration::days(PAST_DAYS),
        end: now + Duration::days(FUTURE_DAYS),
    }
}

fn truncate(value: &str, max: usize) -> String {
    value.chars().take(max).collect()
}

pub fn parse_instant(value: Option<&Value>) -> Option<DateTime<Utc>> {
    value
        .and_then(Value::as_str)
        .and_then(|text| DateTime::parse_from_rfc3339(text).ok())
        .map(|instant| instant.with_timezone(&Utc))
}

fn instant(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(SecondsFormat::Secs, true)
}

pub(crate) fn qualified_kind(key: &str, kind: &str) -> String {
    format!("{key}.{kind}")
}

pub(crate) async fn access_token(
    ctx: &Ctx<'_>,
    api: &dyn CalendarApi,
    scope: &AccessScope,
) -> Result<String, AppError> {
    let key = api.spec().key;
    let name = refresh_token_name(support::scope_user(scope)?);
    let refresh_token = support::secret(ctx, key, &name)
        .await?
        .ok_or_else(|| AppError::validation_code(NOT_CONNECTED, "the calendar is not connected"))?;
    let raw = support::settings(ctx, key).await?;
    let credentials = credentials(ctx, key, &raw).await?;
    let endpoints = api.endpoints(&raw).await?;
    let tokens = oauth::refresh_access_token(
        &endpoints.token_endpoint,
        &credentials,
        &refresh_token,
        api.refresh_scopes(),
    )
    .await
    .map_err(|error| {
        if error_code(&error) == INVALID_GRANT {
            AppError::validation_code(
                AUTHORIZATION_REVOKED,
                "the calendar authorization was revoked or expired; connect the calendar again",
            )
        } else {
            error
        }
    })?;
    if let Some(rotated) = tokens.refresh_token.as_deref()
        && rotated != refresh_token
    {
        let user_ctx = ctx.with_scope(scope);
        let mut tx = user_ctx.begin().await?;
        support::store_secret(&user_ctx, &mut tx, key, &name, rotated).await?;
        tx.commit().await?;
    }
    Ok(tokens.access_token)
}

async fn meeting_type(ctx: &Ctx<'_>, key: &str) -> Result<Uuid, AppError> {
    let meeting_key = configured_meeting_key(ctx, key).await?;
    let listing = task_types::list(
        ctx,
        true,
        ListWindow {
            offset: 0,
            limit: MAX_ACTIVITY_TYPES,
        },
    )
    .await?;
    listing
        .items
        .iter()
        .find(|kind| kind.key == meeting_key && kind.is_active)
        .map(|kind| kind.id)
        .ok_or_else(|| {
            AppError::validation_code(
                MEETING_TYPE_NOT_FOUND,
                format!("the activity type {meeting_key} does not exist or is inactive"),
            )
        })
}

pub(crate) async fn configured_meeting_key(ctx: &Ctx<'_>, key: &str) -> Result<String, AppError> {
    let raw = support::settings(ctx, key).await?;
    Ok(settings::text(&raw, MEETING_TYPE).unwrap_or_else(|| DEFAULT_MEETING_TYPE.to_owned()))
}

async fn import_all(ctx: &Ctx<'_>, key: &str) -> Result<bool, AppError> {
    let raw = support::settings(ctx, key).await?;
    Ok(settings::text(&raw, IMPORT_SCOPE).as_deref() == Some(IMPORT_ALL))
}

async fn contacts_for(
    ctx: &Ctx<'_>,
    attendees: &[String],
    own_email: Option<&str>,
) -> Result<Vec<Uuid>, AppError> {
    let mut found = BTreeSet::new();
    let own = own_email.map(str::to_lowercase);
    let ports = ctx.query();
    let engine = RecordQueryEngine::new(&ports);
    for email in attendees.iter().take(MAX_ATTENDEES) {
        let email = email.trim().to_lowercase();
        if email.is_empty() || Some(&email) == own.as_ref() {
            continue;
        }
        let page = engine
            .query(
                ctx.scope,
                CONTACT,
                RecordQuery {
                    filter: Some(Filter::condition(
                        "email",
                        FilterOperator::Eq,
                        Value::String(email),
                    )),
                    window: ListWindow {
                        offset: 0,
                        limit: 1,
                    },
                    ..RecordQuery::default()
                },
            )
            .await?;
        found.extend(page.items);
    }
    Ok(found.into_iter().collect())
}

fn duration_minutes(start: DateTime<Utc>, end: Option<DateTime<Utc>>) -> i64 {
    end.map(|end| (end - start).num_minutes())
        .unwrap_or(DEFAULT_DURATION_MINUTES)
        .clamp(0, MAX_DURATION_MINUTES)
}

fn remote_title(event: &RemoteEvent) -> String {
    if event.title.trim().is_empty() {
        "(sem título)".to_owned()
    } else {
        truncate(event.title.trim(), MAX_TITLE)
    }
}

fn remote_description(event: &RemoteEvent) -> Option<String> {
    event
        .description
        .as_deref()
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(|text| truncate(text, MAX_DESCRIPTION))
}

fn contact_link(id: &Uuid) -> RecordLinkInput {
    RecordLinkInput {
        record_type: CONTACT.to_owned(),
        record_id: id.to_string(),
    }
}

fn merged_links(activity: &ActivityRecord, people: &[Uuid]) -> Vec<RecordLinkInput> {
    let mut links: Vec<RecordLinkInput> = activity
        .activity
        .links
        .iter()
        .map(|linked| RecordLinkInput {
            record_type: linked.record.record_type.as_str().to_owned(),
            record_id: linked.record.record_id.to_string(),
        })
        .collect();
    for link in people.iter().map(contact_link) {
        if !links.contains(&link) {
            links.push(link);
        }
    }
    links
}

async fn load_activity(
    ctx: &Ctx<'_>,
    activity_id: Uuid,
) -> Result<Option<ActivityRecord>, AppError> {
    match tasks::get(ctx, &activity_id.to_string()).await {
        Ok(activity) => Ok(Some(activity)),
        Err(error) if is_not_found(&error) => Ok(None),
        Err(error) => Err(error),
    }
}

async fn delete_activity(ctx: &Ctx<'_>, activity_id: Uuid) -> Result<(), AppError> {
    match tasks::delete(ctx, &activity_id.to_string(), None).await {
        Ok(()) => Ok(()),
        Err(error) if is_not_found(&error) => Ok(()),
        Err(error) => Err(error),
    }
}

struct SyncContext<'a> {
    ctx: &'a Ctx<'a>,
    api: &'a dyn CalendarApi,
    user_id: Uuid,
    connection: &'a Connection,
    meeting_type_id: Uuid,
    import_all: bool,
    window: SyncWindow,
}

impl SyncContext<'_> {
    fn key(&self) -> &'static str {
        self.api.spec().key
    }

    async fn apply(&self, event: &RemoteEvent) -> Result<bool, AppError> {
        if event.id.is_empty() || event.id.chars().count() > MAX_REMOTE_ID {
            return Ok(false);
        }
        let pool = self.ctx.pool();
        let link = store::link_by_remote(pool, self.key(), self.user_id, &event.id).await?;
        if event.cancelled {
            if let Some(link) = link {
                delete_activity(self.ctx, link.activity_id).await?;
                store::delete_link(pool, self.key(), link.activity_id).await?;
                return Ok(true);
            }
            return Ok(false);
        }
        let Some(start) = event.start.filter(|_| !event.all_day) else {
            return Ok(false);
        };
        match link {
            Some(link) => self.update(event, start, link).await,
            None => self.create(event, start).await,
        }
    }

    async fn create(&self, event: &RemoteEvent, start: DateTime<Utc>) -> Result<bool, AppError> {
        if !self.window.contains(start) {
            return Ok(false);
        }
        let people = contacts_for(
            self.ctx,
            &event.attendees,
            self.connection.account_email.as_deref(),
        )
        .await?;
        if people.is_empty() && !self.import_all {
            return Ok(false);
        }
        let finished = event.end.unwrap_or(start) < Utc::now();
        let created = tasks::create(
            self.ctx,
            CreateActivityCommand {
                title: remote_title(event),
                activity_type_id: self.meeting_type_id.to_string(),
                status: Some(if finished { "done" } else { "open" }.to_owned()),
                due_at: Some(instant(start)),
                owner_id: Some(self.user_id.to_string()),
                links: people.iter().map(contact_link).collect(),
                duration_minutes: Some(duration_minutes(start, event.end)),
                description: remote_description(event),
                ..CreateActivityCommand::default()
            },
        )
        .await?;
        store::save_link(
            self.ctx.pool(),
            self.key(),
            &LinkWrite {
                user_id: self.user_id,
                activity_id: created.activity.id,
                remote_id: &event.id,
                remote_etag: event.etag.as_deref(),
                remote_updated_at: event.updated_at,
                local_updated_at: Some(created.activity.updated_at),
            },
        )
        .await?;
        Ok(true)
    }

    async fn update(
        &self,
        event: &RemoteEvent,
        start: DateTime<Utc>,
        link: Link,
    ) -> Result<bool, AppError> {
        let pool = self.ctx.pool();
        if let (Some(remote), Some(known)) = (event.updated_at, link.remote_updated_at)
            && remote <= known
        {
            return Ok(false);
        }
        let Some(activity) = load_activity(self.ctx, link.activity_id).await? else {
            store::delete_link(pool, self.key(), link.activity_id).await?;
            return Ok(false);
        };
        let local_updated = activity.activity.updated_at;
        let changed_locally = match link.local_updated_at {
            Some(known) => local_updated > known,
            None => true,
        };
        let local_is_newer = event
            .updated_at
            .is_some_and(|remote| local_updated > remote);
        if changed_locally && local_is_newer {
            schedule_push(
                self.ctx,
                self.key(),
                link.activity_id,
                Some(self.user_id),
                false,
            )
            .await?;
            return Ok(false);
        }
        let people = contacts_for(
            self.ctx,
            &event.attendees,
            self.connection.account_email.as_deref(),
        )
        .await?;
        let updated = tasks::update(
            self.ctx,
            &link.activity_id.to_string(),
            UpdateActivityCommand {
                title: Some(remote_title(event)),
                due_at: Some(Some(instant(start))),
                duration_minutes: Some(Some(duration_minutes(start, event.end))),
                description: Some(remote_description(event)),
                links: Some(merged_links(&activity, &people)),
                ..UpdateActivityCommand::default()
            },
        )
        .await?;
        store::save_link(
            pool,
            self.key(),
            &LinkWrite {
                user_id: self.user_id,
                activity_id: link.activity_id,
                remote_id: &event.id,
                remote_etag: event.etag.as_deref(),
                remote_updated_at: event.updated_at,
                local_updated_at: Some(updated.activity.updated_at),
            },
        )
        .await?;
        Ok(true)
    }
}

pub(crate) async fn usable_scope(
    ctx: &Ctx<'_>,
    api: &dyn CalendarApi,
    user_id: Uuid,
) -> Result<Result<AccessScope, (&'static str, &'static str)>, AppError> {
    let spec = api.spec();
    let Some(scope) = support::scope_for_user(ctx, spec.key, user_id).await? else {
        return Ok(Err((
            MEMBER_UNAVAILABLE,
            "the user is no longer an active member",
        )));
    };
    if !support::can(&scope, &spec.permission()) {
        return Ok(Err((
            PERMISSION_REVOKED,
            "the user no longer has permission to sync the calendar",
        )));
    }
    Ok(Ok(scope))
}

pub(crate) fn failure_status(error: &AppError) -> &'static str {
    let code = error_code(error);
    if code == AUTHORIZATION_REVOKED || code == NOT_CONNECTED {
        STATUS_REVOKED
    } else {
        STATUS_ERROR
    }
}

fn public_message(error: &AppError) -> String {
    if is_internal(error) {
        "unexpected failure while syncing the calendar".to_owned()
    } else {
        error.client_message().to_owned()
    }
}

pub(crate) async fn run_sync(
    ctx: &Ctx<'_>,
    api: &dyn CalendarApi,
    user_id: Uuid,
) -> Result<(), AppError> {
    let key = api.spec().key;
    let pool = ctx.pool();
    let Some(connection) = store::find_connection(pool, key, user_id).await? else {
        return Ok(());
    };
    if !connection.syncable() {
        return Ok(());
    }
    let scope = match usable_scope(ctx, api, user_id).await? {
        Ok(scope) => scope,
        Err((code, message)) => {
            return store::mark_failure(pool, key, user_id, STATUS_ERROR, code, message).await;
        }
    };
    let user_ctx = ctx.with_scope(&scope);
    match sync_connection(&user_ctx, api, user_id, &connection).await {
        Ok(outcome) => {
            store::mark_healthy(pool, key, user_id).await?;
            store::finish_sync(pool, key, user_id, &outcome).await
        }
        Err(error) => {
            let code = error_code(&error);
            tracing::warn!(module = key, code, "calendar sync failed");
            store::mark_failure(
                pool,
                key,
                user_id,
                failure_status(&error),
                code,
                &public_message(&error),
            )
            .await
        }
    }
}

async fn sync_connection(
    ctx: &Ctx<'_>,
    api: &dyn CalendarApi,
    user_id: Uuid,
    connection: &Connection,
) -> Result<SyncOutcome, AppError> {
    let key = api.spec().key;
    let token = access_token(ctx, api, ctx.scope).await?;
    let meeting_type_id = meeting_type(ctx, key).await?;
    let window = window(Utc::now());
    let changes = match api
        .changes(&token, connection.sync_cursor.as_deref(), window)
        .await
    {
        Err(error) if error_code(&error) == CURSOR_EXPIRED => {
            store::reset_cursor(ctx.pool(), key, user_id).await?;
            api.changes(&token, None, window).await?
        }
        other => other?,
    };
    let context = SyncContext {
        ctx,
        api,
        user_id,
        connection,
        meeting_type_id,
        import_all: import_all(ctx, key).await?,
        window,
    };
    let mut outcome = SyncOutcome {
        cursor: changes.cursor.clone(),
        ..SyncOutcome::default()
    };
    let mut failures = 0u32;
    for event in changes.events.iter().take(MAX_EVENTS_PER_RUN) {
        match context.apply(event).await {
            Ok(true) => outcome.imported += 1,
            Ok(false) => {}
            Err(error) => {
                failures += 1;
                if outcome.error.is_none() {
                    outcome.error = Some((error_code(&error).to_owned(), public_message(&error)));
                }
            }
        }
    }
    if failures > 0 {
        outcome.error = outcome.error.map(|(code, message)| {
            (
                if code.is_empty() {
                    EVENT_FAILED.to_owned()
                } else {
                    code
                },
                format!("{failures} evento(s) não sincronizado(s): {message}"),
            )
        });
    }
    Ok(outcome)
}

pub(crate) async fn schedule_push(
    ctx: &Ctx<'_>,
    key: &str,
    activity_id: Uuid,
    owner_id: Option<Uuid>,
    deleted: bool,
) -> Result<(), AppError> {
    ctx.jobs()
        .enqueue_now(
            Job::new(
                qualified_kind(key, super::jobs::PUSH_KIND),
                json!({ "activity_id": activity_id, "owner_id": owner_id, "deleted": deleted }),
            )
            .run_at(Utc::now() + Duration::seconds(PUSH_DELAY_SECONDS))
            .with_unique_key(format!("{key}.push:{activity_id}")),
        )
        .await?;
    Ok(())
}

fn local_event(activity: &ActivityRecord) -> Option<LocalEvent> {
    let view = &activity.activity;
    let start = view.due_at?;
    let minutes = view
        .duration_minutes
        .map(i64::from)
        .filter(|minutes| *minutes > 0)
        .unwrap_or(DEFAULT_DURATION_MINUTES)
        .min(MAX_DURATION_MINUTES);
    Some(LocalEvent {
        title: truncate(&view.title, MAX_TITLE),
        description: view
            .description
            .as_deref()
            .map(|text| truncate(text, MAX_DESCRIPTION)),
        start,
        end: start + Duration::minutes(minutes),
    })
}

async fn remove_remote(
    ctx: &Ctx<'_>,
    api: &dyn CalendarApi,
    user_id: Uuid,
    link: &Link,
) -> Result<(), AppError> {
    let key = api.spec().key;
    let token = access_token(ctx, api, ctx.scope).await?;
    match api.delete_event(&token, &link.remote_id).await {
        Ok(()) => {}
        Err(error) if error_code(&error) == REMOTE_NOT_FOUND => {}
        Err(error) => return Err(error),
    }
    store::delete_link(ctx.pool(), key, link.activity_id).await?;
    store::count_export(ctx.pool(), key, user_id).await
}

pub(crate) async fn push(
    ctx: &Ctx<'_>,
    api: &dyn CalendarApi,
    activity_id: Uuid,
    owner_hint: Option<Uuid>,
    deleted: bool,
) -> Result<(), AppError> {
    let key = api.spec().key;
    let pool = ctx.pool();
    let link = store::link_by_activity(pool, key, activity_id).await?;
    let Some(user_id) = link.as_ref().map(|(user, _)| *user).or(owner_hint) else {
        return Ok(());
    };
    let Some(connection) = store::find_connection(pool, key, user_id).await? else {
        return Ok(());
    };
    if !connection.syncable() {
        return Ok(());
    }
    let Ok(scope) = usable_scope(ctx, api, user_id).await? else {
        return Ok(());
    };
    let user_ctx = ctx.with_scope(&scope);
    let link = link.map(|(_, link)| link);
    let activity = if deleted {
        None
    } else {
        load_activity(&user_ctx, activity_id).await?
    };
    let Some(activity) = activity else {
        if let Some(link) = &link {
            remove_remote(&user_ctx, api, user_id, link).await?;
        }
        return Ok(());
    };
    let meeting_key = configured_meeting_key(ctx, key).await?;
    let is_meeting = activity.activity.activity_type.key == meeting_key;
    let user = user_id.to_string();
    let owned = activity
        .activity
        .owner
        .as_ref()
        .is_some_and(|owner| owner.id.as_str() == user);
    if !is_meeting || !owned {
        return Ok(());
    }
    let local_updated = activity.activity.updated_at;
    if let Some(link) = &link
        && let Some(known) = link.local_updated_at
        && local_updated <= known
    {
        return Ok(());
    }
    let Some(event) = local_event(&activity) else {
        return Ok(());
    };
    let token = access_token(&user_ctx, api, &scope).await?;
    let remote = match &link {
        Some(link) => match api.update_event(&token, &link.remote_id, &event).await {
            Err(error) if error_code(&error) == REMOTE_NOT_FOUND => {
                store::delete_link(pool, key, activity_id).await?;
                api.create_event(&token, &event).await?
            }
            other => other?,
        },
        None => api.create_event(&token, &event).await?,
    };
    if remote.id.is_empty() || remote.id.chars().count() > MAX_REMOTE_ID {
        return Err(internal("the calendar answered with an unusable event id"));
    }
    store::save_link(
        pool,
        key,
        &LinkWrite {
            user_id,
            activity_id,
            remote_id: &remote.id,
            remote_etag: remote.etag.as_deref(),
            remote_updated_at: remote.updated_at,
            local_updated_at: Some(local_updated),
        },
    )
    .await?;
    store::count_export(pool, key, user_id).await
}

pub(crate) fn is_activity(record_type: &RecordType) -> bool {
    record_type.as_str() == RecordType::ACTIVITY.as_str()
}
