use std::collections::HashMap;

use chrono::{NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Utc};
use chrono_tz::Tz;
use serde_json::Map;
use uuid::Uuid;

use crate::{
    ACTIVITY_TYPE_COLUMN, BODY_COLUMN, COMPANY_COLUMN, CONTACT_COLUMN, CONTACT_EMAIL_COLUMN,
    DEAL_COLUMN, DESCRIPTION_COLUMN, DONE_COLUMN, DUE_AT_COLUMN, NormalizedFile, OWNER_COLUMN,
    StepKind, TITLE_COLUMN,
};
use activities::domain::{CreateActivityCommand, RecordLinkInput};
use aphel_kernel::application::query::{RecordQueryExecutor, RecordQueryPorts};
use aphel_kernel::application::shared::RecordType;
use aphel_kernel::context::Ctx;
use aphel_kernel::errors::AppError;
use imports::store::RecordLookups;

pub(crate) const MAX_HISTORY_ROWS: usize = 5_000;
pub(crate) const MAX_REPORTED_ERRORS: usize = 100;
pub(crate) const HISTORY_TOO_MANY_ROWS: &str = "too_many_rows";
const DEAL_TYPE: &str = "deal";
const CONTACT_TYPE: &str = "contact";
const COMPANY_TYPE: &str = "company";
const ACTIVITY_TYPE_LOOKUP: &str = "activity_type";
const DEFAULT_ACTIVITY_TITLE: &str = "Atividade migrada";
const NOON: NaiveTime = match NaiveTime::from_hms_opt(12, 0, 0) {
    Some(time) => time,
    None => NaiveTime::MIN,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct HistoryRowError {
    pub(crate) row: usize,
    pub(crate) message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct HistoryReport {
    pub(crate) total: u64,
    pub(crate) created: u64,
    pub(crate) skipped: u64,
    pub(crate) failed: u64,
    pub(crate) errors: Vec<HistoryRowError>,
}

impl HistoryReport {
    fn fail(&mut self, row: usize, message: impl Into<String>) {
        self.failed += 1;
        if self.errors.len() < MAX_REPORTED_ERRORS {
            self.errors.push(HistoryRowError {
                row,
                message: message.into(),
            });
        }
    }
}

struct Row<'a> {
    cells: HashMap<&'a str, &'a str>,
}

impl<'a> Row<'a> {
    fn new(file: &'a NormalizedFile, cells: &'a [String]) -> Self {
        let mut values = HashMap::new();
        for column in &file.recognized {
            if let Some(value) = cells.get(column.index).map(|value| value.trim())
                && !value.is_empty()
            {
                values.insert(column.target.as_str(), value);
            }
        }
        Self { cells: values }
    }

    fn get(&self, key: &str) -> Option<&'a str> {
        self.cells.get(key).copied()
    }
}

struct Resolver<'c, 'a> {
    ctx: &'c Ctx<'a>,
    lookups: RecordLookups,
    cache: HashMap<(&'static str, String), Option<Uuid>>,
    users: HashMap<String, Option<String>>,
}

impl<'c, 'a> Resolver<'c, 'a> {
    fn new(ctx: &'c Ctx<'a>) -> Self {
        Self {
            ctx,
            lookups: RecordLookups::of(ctx),
            cache: HashMap::new(),
            users: HashMap::new(),
        }
    }

    async fn record(
        &mut self,
        record_type: &'static str,
        label: &str,
    ) -> Result<Option<Uuid>, AppError> {
        let key = (record_type, label.to_lowercase());
        if let Some(found) = self.cache.get(&key) {
            return Ok(*found);
        }
        let found = self
            .lookups
            .find_record_by_label(self.ctx.scope, record_type, label)
            .await?;
        self.cache.insert(key, found);
        Ok(found)
    }

    async fn user(&mut self, email: &str) -> Result<Option<String>, AppError> {
        if let Some(found) = self.users.get(email) {
            return Ok(found.clone());
        }
        let found = self
            .lookups
            .find_member_by_email(email)
            .await?
            .map(|user| user.as_str().to_owned());
        self.users.insert(email.to_owned(), found.clone());
        Ok(found)
    }

    async fn links(&mut self, row: &Row<'_>) -> Result<Vec<(RecordType, Uuid)>, AppError> {
        let mut links = Vec::new();
        if let Some(deal) = row.get(DEAL_COLUMN)
            && let Some(id) = self.record(DEAL_TYPE, deal).await?
        {
            links.push((RecordType::DEAL, id));
        }
        let contact = match row.get(CONTACT_EMAIL_COLUMN) {
            Some(email) => self.record(CONTACT_TYPE, email).await?,
            None => None,
        };
        let contact = match (contact, row.get(CONTACT_COLUMN)) {
            (Some(id), _) => Some(id),
            (None, Some(label)) => self.record(CONTACT_TYPE, label).await?,
            (None, None) => None,
        };
        if let Some(id) = contact {
            links.push((RecordType::CONTACT, id));
        }
        if let Some(company) = row.get(COMPANY_COLUMN)
            && let Some(id) = self.record(COMPANY_TYPE, company).await?
        {
            links.push((RecordType::COMPANY, id));
        }
        Ok(links)
    }
}

fn instant(value: &str, timezone: Tz) -> Option<String> {
    let value = value.trim();
    let local = NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M")
        .ok()
        .or_else(|| {
            NaiveDate::parse_from_str(value, "%Y-%m-%d")
                .ok()
                .map(|date| date.and_time(NOON))
        })?;
    let resolved = timezone.from_local_datetime(&local).earliest()?;
    Some(resolved.with_timezone(&Utc).to_rfc3339())
}

fn error_message(error: &AppError) -> String {
    error.to_string().chars().take(300).collect()
}

pub(crate) async fn run(
    ctx: &Ctx<'_>,
    kind: StepKind,
    file: &NormalizedFile,
    ip: Option<String>,
) -> Result<HistoryReport, AppError> {
    if file.rows.len() > MAX_HISTORY_ROWS {
        return Err(AppError::validation_code(
            HISTORY_TOO_MANY_ROWS,
            format!("activities and notes are migrated in files of up to {MAX_HISTORY_ROWS} rows"),
        ));
    }
    let timezone = ctx.query().executor().timezone().await?;
    let mut resolver = Resolver::new(ctx);
    let mut report = HistoryReport {
        total: file.rows.len() as u64,
        ..HistoryReport::default()
    };
    for (index, cells) in file.rows.iter().enumerate() {
        let row_number = index + 2;
        let row = Row::new(file, cells);
        let links = resolver.links(&row).await?;
        let outcome = match kind {
            StepKind::Activities => {
                create_activity(ctx, &mut resolver, &row, links, timezone, ip.clone()).await
            }
            StepKind::Notes => create_notes(ctx, &row, links, ip.clone()).await,
            StepKind::Import => Ok(false),
        };
        match outcome {
            Ok(true) => report.created += 1,
            Ok(false) => report.skipped += 1,
            Err(error) => report.fail(row_number, error_message(&error)),
        }
    }
    Ok(report)
}

async fn create_activity(
    ctx: &Ctx<'_>,
    resolver: &mut Resolver<'_, '_>,
    row: &Row<'_>,
    links: Vec<(RecordType, Uuid)>,
    timezone: Tz,
    ip: Option<String>,
) -> Result<bool, AppError> {
    let title = row.get(TITLE_COLUMN).or(row.get(ACTIVITY_TYPE_COLUMN));
    if title.is_none() && row.get(DESCRIPTION_COLUMN).is_none() {
        return Ok(false);
    }
    let activity_type_id = match row.get(ACTIVITY_TYPE_COLUMN) {
        Some(label) => resolver
            .record(ACTIVITY_TYPE_LOOKUP, label)
            .await?
            .map(|id| id.to_string())
            .unwrap_or_default(),
        None => String::new(),
    };
    let owner_id = match row.get(OWNER_COLUMN) {
        Some(email) => resolver.user(email).await?,
        None => None,
    };
    let due_at = row
        .get(DUE_AT_COLUMN)
        .and_then(|value| instant(value, timezone));
    let done = row.get(DONE_COLUMN) == Some("done");
    let completed_at = if done {
        Some(due_at.clone().unwrap_or_else(|| ctx.now().to_rfc3339()))
    } else {
        None
    };
    let mut links = links.into_iter();
    let primary = links.next();
    activities::service::create(
        ctx,
        CreateActivityCommand {
            title: title.unwrap_or(DEFAULT_ACTIVITY_TITLE).to_owned(),
            activity_type_id,
            status: Some(if done { "done" } else { "open" }.to_owned()),
            priority: None,
            due_at,
            completed_at,
            owner_id,
            record_type: primary
                .as_ref()
                .map(|(record_type, _)| record_type.as_str().to_owned()),
            record_id: primary.as_ref().map(|(_, id)| id.to_string()),
            links: links
                .map(|(record_type, id)| RecordLinkInput {
                    record_type: record_type.as_str().to_owned(),
                    record_id: id.to_string(),
                })
                .collect(),
            duration_minutes: None,
            description: row.get(DESCRIPTION_COLUMN).map(str::to_owned),
            location: None,
            custom_fields: Map::new(),
            ip,
        },
    )
    .await?;
    Ok(true)
}

async fn create_notes(
    ctx: &Ctx<'_>,
    row: &Row<'_>,
    links: Vec<(RecordType, Uuid)>,
    ip: Option<String>,
) -> Result<bool, AppError> {
    let Some(body) = row.get(BODY_COLUMN) else {
        return Ok(false);
    };
    let Some((record_type, record_id)) = links.into_iter().next() else {
        return Err(AppError::Validation(
            "the note is not linked to a deal, contact or company found in Aphel".to_owned(),
        ));
    };
    notes::service::create(
        ctx,
        notes::domain::CreateNoteCommand {
            record_type: record_type.as_str().to_owned(),
            record_id: record_id.to_string(),
            body: body.to_owned(),
            visibility: None,
            pinned: None,
            ip,
        },
    )
    .await?;
    Ok(true)
}
