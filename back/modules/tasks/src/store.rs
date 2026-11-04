use aphel_contract_crm::CrmRecordTypes;
use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, NaiveDateTime, Utc, Weekday};
use rust_decimal::Decimal;
use sqlx::mysql::MySqlRow;
use sqlx::{MySql, MySqlPool, QueryBuilder, Row};
use uuid::Uuid;

use crate::domain::{
    ACTIVITY_TYPE_KEY_TAKEN, ActivityData, ActivityLocation, ActivityPriority, ActivityStatus,
    ActivityTypeSummary, ActivityTypeView, ActivityView, LinkedDealValue, LinkedRecord,
};
use crate::recurrence::{
    ActivityRecurrenceView, ActivityTemplate, RecurrenceFrequency, RecurrenceRule,
    RecurrenceSeries, parse_weekday_code, weekday_code,
};
use aphel_kernel::Ctx;
use aphel_kernel::application::fields::RecordTypeRegistry;
use aphel_kernel::application::shared::{RecordRef, RecordType};
use aphel_kernel::database::transaction::MySqlTx;
use aphel_kernel::domain::UserId;
use aphel_kernel::domain::access::{AccessScope, RecordScopeFilter};
use aphel_kernel::errors::AppError;
use aphel_kernel::infrastructure::fields::mysql_definitions::uuid_column;
use aphel_kernel::infrastructure::query::compiler::push_record_scope;
use aphel_kernel::infrastructure::repositories::mapper::{self, user_reference};

pub fn activity_select(alias: &str) -> String {
    format!(
        "SELECT {alias}.id, {alias}.title, {alias}.activity_type_id, t.`key` AS type_key, t.name AS type_name, \
         t.icon AS type_icon, t.color AS type_color, {alias}.status, {alias}.priority, {alias}.due_at, \
         {alias}.completed_at, {alias}.owner_id, ou.display_name AS owner_name, {alias}.created_by, \
         cu.display_name AS created_by_name, {alias}.record_type, {alias}.record_id, \
         NULL AS record_title, {alias}.duration_minutes, {alias}.description, {alias}.created_at, \
         {alias}.location_latitude, {alias}.location_longitude, {alias}.location_accuracy_m, \
         {alias}.location_captured_at, \
         {alias}.updated_at, COALESCE({alias}.completed_at, {alias}.created_at) AS occurred_at, \
         {alias}.reminder_minutes_before, {alias}.recurrence_id, {alias}.recurrence_index, \
         rr.frequency AS recurrence_frequency, rr.interval_count AS recurrence_interval, \
         rr.weekdays AS recurrence_weekdays, rr.month_day AS recurrence_month_day, \
         rr.until_date AS recurrence_until, rr.max_count AS recurrence_count, \
         rr.ended_at AS recurrence_ended_at \
         FROM tasks {alias} \
         JOIN task_types t ON t.id = {alias}.activity_type_id \
         LEFT JOIN task_recurrences rr ON rr.id = {alias}.recurrence_id \
         LEFT JOIN users ou ON ou.id = {alias}.owner_id \
         LEFT JOIN users cu ON cu.id = {alias}.created_by ",
    )
}

pub fn linked_record(
    row: &MySqlRow,
    type_column: &str,
    id_column: &str,
    title_column: &str,
) -> Result<Option<LinkedRecord>, AppError> {
    let record_type: Option<String> = row.try_get(type_column)?;
    let record_id: Option<String> = row.try_get(id_column)?;
    let (Some(record_type), Some(record_id)) = (record_type, record_id) else {
        return Ok(None);
    };
    Ok(Some(LinkedRecord {
        record: RecordRef::parse(&record_type, &record_id)
            .map_err(|_| mapper::invalid_column(type_column, &record_type))?,
        title: row.try_get(title_column)?,
    }))
}

fn optional_instant(row: &MySqlRow, column: &str) -> Result<Option<DateTime<Utc>>, AppError> {
    let value: Option<NaiveDateTime> = row.try_get(column)?;
    Ok(value.map(mapper::utc))
}

fn location_from_row(row: &MySqlRow) -> Result<Option<ActivityLocation>, AppError> {
    let latitude: Option<Decimal> = row.try_get("location_latitude")?;
    let longitude: Option<Decimal> = row.try_get("location_longitude")?;
    let captured_at = optional_instant(row, "location_captured_at")?;
    let (Some(latitude), Some(longitude), Some(captured_at)) = (latitude, longitude, captured_at)
    else {
        return Ok(None);
    };
    Ok(Some(ActivityLocation {
        latitude,
        longitude,
        accuracy_m: row.try_get("location_accuracy_m")?,
        captured_at,
    }))
}

pub fn activity_from_row(row: &MySqlRow) -> Result<ActivityView, AppError> {
    let status: String = row.try_get("status")?;
    let priority: String = row.try_get("priority")?;
    let created_at: NaiveDateTime = row.try_get("created_at")?;
    let updated_at: NaiveDateTime = row.try_get("updated_at")?;
    Ok(ActivityView {
        id: uuid_column(row, "id")?,
        title: row.try_get("title")?,
        activity_type: ActivityTypeSummary {
            id: uuid_column(row, "activity_type_id")?,
            key: row.try_get("type_key")?,
            name: row.try_get("type_name")?,
            icon: row.try_get("type_icon")?,
            color: row.try_get("type_color")?,
        },
        status: ActivityStatus::parse(&status)
            .ok_or_else(|| mapper::invalid_column("status", &status))?,
        priority: ActivityPriority::parse(&priority)
            .ok_or_else(|| mapper::invalid_column("priority", &priority))?,
        due_at: optional_instant(row, "due_at")?,
        completed_at: optional_instant(row, "completed_at")?,
        owner: user_reference(row, "owner_id", "owner_name")?,
        created_by: user_reference(row, "created_by", "created_by_name")?,
        record: linked_record(row, "record_type", "record_id", "record_title")?,
        links: Vec::new(),
        duration_minutes: row.try_get("duration_minutes")?,
        description: row.try_get("description")?,
        location: location_from_row(row)?,
        reminder_minutes_before: row.try_get("reminder_minutes_before")?,
        recurrence: recurrence_from_row(row)?,
        created_at: mapper::utc(created_at),
        updated_at: mapper::utc(updated_at),
    })
}

fn weekdays_from_text(value: Option<String>) -> Vec<Weekday> {
    value
        .unwrap_or_default()
        .split(',')
        .filter_map(parse_weekday_code)
        .collect()
}

fn weekdays_text(weekdays: &[Weekday]) -> Option<String> {
    (!weekdays.is_empty()).then(|| {
        weekdays
            .iter()
            .map(|weekday| weekday_code(*weekday))
            .collect::<Vec<_>>()
            .join(",")
    })
}

fn rule_from_row(row: &MySqlRow, prefix: &str) -> Result<Option<RecurrenceRule>, AppError> {
    let frequency: Option<String> = row.try_get(format!("{prefix}frequency").as_str())?;
    let Some(frequency) = frequency else {
        return Ok(None);
    };
    let interval: Option<u32> = row.try_get(format!("{prefix}interval").as_str())?;
    let month_day: Option<u8> = row.try_get(format!("{prefix}month_day").as_str())?;
    Ok(Some(RecurrenceRule {
        frequency: RecurrenceFrequency::parse(&frequency)
            .ok_or_else(|| mapper::invalid_column("frequency", &frequency))?,
        interval: interval.unwrap_or(1).max(1),
        weekdays: weekdays_from_text(row.try_get(format!("{prefix}weekdays").as_str())?),
        month_day: month_day.map(u32::from),
        until: row.try_get(format!("{prefix}until").as_str())?,
        count: row.try_get(format!("{prefix}count").as_str())?,
    }))
}

fn recurrence_from_row(row: &MySqlRow) -> Result<Option<ActivityRecurrenceView>, AppError> {
    let series_id: Option<String> = row.try_get("recurrence_id")?;
    let index: Option<u32> = row.try_get("recurrence_index")?;
    let (Some(series_id), Some(index)) = (series_id, index) else {
        return Ok(None);
    };
    let Some(rule) = rule_from_row(row, "recurrence_")? else {
        return Ok(None);
    };
    let ended_at: Option<NaiveDateTime> = row.try_get("recurrence_ended_at")?;
    Ok(Some(ActivityRecurrenceView {
        series_id: Uuid::parse_str(&series_id)
            .map_err(|_| mapper::invalid_column("recurrence_id", &series_id))?,
        index,
        rule,
        ended: ended_at.is_some(),
    }))
}

fn reminder_at(data: &ActivityData) -> Option<NaiveDateTime> {
    match (data.due_at, data.reminder_minutes_before) {
        (Some(due_at), Some(minutes)) => Some(mapper::naive(
            due_at - chrono::Duration::minutes(i64::from(minutes)),
        )),
        _ => None,
    }
}

#[derive(Clone)]
pub(crate) struct ActivityStore<'a> {
    ctx: Ctx<'a>,
    pool: MySqlPool,
    registry: Arc<RecordTypeRegistry>,
}

impl<'a> ActivityStore<'a> {
    pub(crate) fn of(ctx: &Ctx<'a>) -> Self {
        Self {
            ctx: ctx.clone(),
            pool: ctx.pool().clone(),
            registry: ctx.registry().clone(),
        }
    }

    pub(crate) async fn links_of(
        &self,
        ids: &[Uuid],
    ) -> Result<HashMap<Uuid, Vec<LinkedRecord>>, AppError> {
        let mut links: HashMap<Uuid, Vec<LinkedRecord>> = HashMap::new();
        if ids.is_empty() {
            return Ok(links);
        }
        let mut builder = QueryBuilder::<MySql>::new(
            "SELECT l.activity_id, l.record_type, l.record_id, NULL AS record_title FROM task_links l WHERE ",
        );
        builder.push("l.activity_id IN (");
        let mut separated = builder.separated(", ");
        for id in ids {
            separated.push_bind(id.to_string());
        }
        builder.push(") ORDER BY l.id");
        let rows = builder.build().fetch_all(&self.pool).await?;
        for row in &rows {
            if let Some(link) = linked_record(row, "record_type", "record_id", "record_title")? {
                links
                    .entry(uuid_column(row, "activity_id")?)
                    .or_default()
                    .push(link);
            }
        }
        Ok(links)
    }

    pub(crate) async fn write_links(
        tx: &mut MySqlTx,
        id: Uuid,
        links: &[RecordRef],
        now: DateTime<Utc>,
    ) -> Result<(), AppError> {
        sqlx::query("DELETE FROM task_links WHERE activity_id = ?")
            .bind(id.to_string())
            .execute(&mut **tx)
            .await?;
        for link in links {
            sqlx::query(
                "INSERT INTO task_links (id, activity_id, record_type, record_id, created_at) \
                 VALUES (?, ?, ?, ?, ?)",
            )
            .bind(Uuid::now_v7().to_string())
            .bind(id.to_string())
            .bind(link.record_type.as_str())
            .bind(link.record_id.to_string())
            .bind(mapper::naive(now))
            .execute(&mut **tx)
            .await?;
        }
        Ok(())
    }
}

impl<'a> ActivityStore<'a> {
    pub(crate) async fn insert(
        &self,
        tx: &mut MySqlTx,
        scope: &AccessScope,
        id: Uuid,
        data: &ActivityData,
        now: DateTime<Utc>,
    ) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO tasks (id, owner_id, created_by, activity_type_id, title, description, \
             priority, status, due_at, completed_at, record_type, record_id, duration_minutes, \
             location_latitude, location_longitude, location_accuracy_m, location_captured_at, recurrence_id, \
             recurrence_index, reminder_minutes_before, reminder_at, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(id.to_string())
        .bind(data.owner_id.as_ref().map(UserId::as_str))
        .bind(scope.user_id().as_str())
        .bind(data.activity_type_id.to_string())
        .bind(&data.title)
        .bind(&data.description)
        .bind(data.priority.as_str())
        .bind(data.status.as_str())
        .bind(data.due_at.map(mapper::naive))
        .bind(data.completed_at.map(mapper::naive))
        .bind(data.record.as_ref().map(|record| record.record_type.as_str()))
        .bind(data.record.as_ref().map(|record| record.record_id.to_string()))
        .bind(data.duration_minutes)
        .bind(data.location.map(|location| location.latitude))
        .bind(data.location.map(|location| location.longitude))
        .bind(data.location.and_then(|location| location.accuracy_m))
        .bind(data.location.map(|location| mapper::naive(location.captured_at)))
        .bind(data.occurrence.map(|occurrence| occurrence.series_id.to_string()))
        .bind(data.occurrence.map(|occurrence| occurrence.index))
        .bind(data.reminder_minutes_before)
        .bind(reminder_at(data))
        .bind(mapper::naive(now))
        .bind(mapper::naive(now))
        .execute(&mut **tx)
        .await?;
        Self::write_links(tx, id, &data.links, now).await
    }

    pub(crate) async fn update(
        &self,
        tx: &mut MySqlTx,
        id: Uuid,
        data: &ActivityData,
        now: DateTime<Utc>,
    ) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE tasks SET \
             reminder_sent_at = IF(reminder_at <=> ? AND status = 'open' AND ? = 'open', reminder_sent_at, NULL), \
             owner_id = ?, activity_type_id = ?, title = ?, description = ?, priority = ?, \
             status = ?, due_at = ?, completed_at = ?, record_type = ?, record_id = ?, duration_minutes = ?, \
             location_latitude = ?, location_longitude = ?, location_accuracy_m = ?, location_captured_at = ?, \
             recurrence_id = ?, recurrence_index = ?, reminder_minutes_before = ?, reminder_at = ?, updated_at = ? \
             WHERE id = ? AND deleted_at IS NULL",
        )
        .bind(reminder_at(data))
        .bind(data.status.as_str())
        .bind(data.owner_id.as_ref().map(UserId::as_str))
        .bind(data.activity_type_id.to_string())
        .bind(&data.title)
        .bind(&data.description)
        .bind(data.priority.as_str())
        .bind(data.status.as_str())
        .bind(data.due_at.map(mapper::naive))
        .bind(data.completed_at.map(mapper::naive))
        .bind(data.record.as_ref().map(|record| record.record_type.as_str()))
        .bind(data.record.as_ref().map(|record| record.record_id.to_string()))
        .bind(data.duration_minutes)
        .bind(data.location.map(|location| location.latitude))
        .bind(data.location.map(|location| location.longitude))
        .bind(data.location.and_then(|location| location.accuracy_m))
        .bind(data.location.map(|location| mapper::naive(location.captured_at)))
        .bind(data.occurrence.map(|occurrence| occurrence.series_id.to_string()))
        .bind(data.occurrence.map(|occurrence| occurrence.index))
        .bind(data.reminder_minutes_before)
        .bind(reminder_at(data))
        .bind(mapper::naive(now))
        .bind(id.to_string())
        .execute(&mut **tx)
        .await?;
        Self::write_links(tx, id, &data.links, now).await
    }

    pub(crate) async fn soft_delete(
        &self,
        tx: &mut MySqlTx,
        id: Uuid,
        now: DateTime<Utc>,
    ) -> Result<bool, AppError> {
        let result = sqlx::query(
            "UPDATE tasks SET deleted_at = ?, updated_at = ? \
             WHERE id = ? AND deleted_at IS NULL",
        )
        .bind(mapper::naive(now))
        .bind(mapper::naive(now))
        .bind(id.to_string())
        .execute(&mut **tx)
        .await?;
        Ok(result.rows_affected() == 1)
    }

    pub(crate) async fn untouched_occurrence(
        &self,
        tx: &mut MySqlTx,
        series_id: Uuid,
        index: u32,
    ) -> Result<Option<Uuid>, AppError> {
        let id: Option<String> = sqlx::query_scalar(
            "SELECT id FROM tasks WHERE recurrence_id = ? AND recurrence_index = ? \
             AND deleted_at IS NULL AND status = 'open' AND updated_at = created_at FOR UPDATE",
        )
        .bind(series_id.to_string())
        .bind(index)
        .fetch_optional(&mut **tx)
        .await?;
        id.map(|id| Uuid::parse_str(&id).map_err(|_| mapper::invalid_column("id", &id)))
            .transpose()
    }

    pub(crate) async fn discard_occurrence(
        &self,
        tx: &mut MySqlTx,
        id: Uuid,
        now: DateTime<Utc>,
    ) -> Result<bool, AppError> {
        let result = sqlx::query(
            "UPDATE tasks SET deleted_at = ?, updated_at = ?, recurrence_id = NULL, \
             recurrence_index = NULL WHERE id = ? AND deleted_at IS NULL",
        )
        .bind(mapper::naive(now))
        .bind(mapper::naive(now))
        .bind(id.to_string())
        .execute(&mut **tx)
        .await?;
        Ok(result.rows_affected() == 1)
    }

    pub(crate) async fn find(&self, id: Uuid) -> Result<Option<ActivityView>, AppError> {
        Ok(self.find_many(&[id]).await?.into_iter().next())
    }

    pub(crate) async fn find_many(&self, ids: &[Uuid]) -> Result<Vec<ActivityView>, AppError> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut builder = QueryBuilder::<MySql>::new(activity_select("a"));
        builder
            .push("WHERE ")
            .push("a.deleted_at IS NULL AND a.id IN (");
        let mut separated = builder.separated(", ");
        for id in ids {
            separated.push_bind(id.to_string());
        }
        builder.push(")");
        let rows = builder.build().fetch_all(&self.pool).await?;
        let mut tasks = rows
            .iter()
            .map(activity_from_row)
            .collect::<Result<Vec<_>, _>>()?;
        let found: Vec<Uuid> = tasks.iter().map(|activity| activity.id).collect();
        let mut links = self.links_of(&found).await?;
        for activity in &mut tasks {
            activity.links = links.remove(&activity.id).unwrap_or_default();
        }
        hydrate_link_titles(
            &self.ctx,
            tasks
                .iter_mut()
                .flat_map(|task| task.record.iter_mut().chain(task.links.iter_mut()))
                .collect(),
        )
        .await?;
        Ok(tasks)
    }

    pub(crate) async fn linked_activity_ids(
        &self,
        record: &RecordRef,
        limit: u64,
    ) -> Result<Vec<Uuid>, AppError> {
        let ids: Vec<String> = sqlx::query_scalar(
            "SELECT l.activity_id FROM task_links l \
             WHERE l.record_type = ? AND l.record_id = ? \
             ORDER BY l.activity_id DESC LIMIT ?",
        )
        .bind(record.record_type.as_str())
        .bind(record.record_id.to_string())
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        ids.into_iter()
            .map(|id| Uuid::parse_str(&id).map_err(|_| mapper::invalid_column("activity_id", &id)))
            .collect()
    }

    pub(crate) async fn within_scope(
        &self,
        scope: &AccessScope,
        id: Uuid,
        filter: &RecordScopeFilter,
    ) -> Result<bool, AppError> {
        let definition = self.registry.get(&RecordType::ACTIVITY).ok_or_else(|| {
            AppError::Configuration("the activity record type is not registered".to_owned())
        })?;
        let mut builder = QueryBuilder::<MySql>::new("SELECT EXISTS(SELECT 1 FROM tasks r WHERE ");
        builder
            .push("r.id = ")
            .push_bind(id.to_string())
            .push(" AND r.deleted_at IS NULL");
        push_record_scope(&mut builder, scope, definition, filter);
        builder.push(")");
        let exists: bool = builder.build_query_scalar().fetch_one(&self.pool).await?;
        Ok(exists)
    }

    pub(crate) async fn linked_deal_values(
        &self,
        ids: &[Uuid],
    ) -> Result<HashMap<Uuid, LinkedDealValue>, AppError> {
        let mut values = HashMap::new();
        if ids.is_empty() {
            return Ok(values);
        }
        let mut builder = QueryBuilder::<MySql>::new(
            "SELECT a.id AS activity_id, a.record_id AS deal_id, 0 AS position FROM tasks a WHERE ",
        );
        builder.push("a.record_type = 'deals.deal' AND a.id IN (");
        let mut separated = builder.separated(", ");
        for id in ids {
            separated.push_bind(id.to_string());
        }
        builder.push(") UNION ALL SELECT l.activity_id, l.record_id AS deal_id, 1 AS position FROM task_links l WHERE l.record_type = 'deals.deal' AND l.activity_id IN (");
        let mut separated = builder.separated(", ");
        for id in ids {
            separated.push_bind(id.to_string());
        }
        builder.push(")");
        let rows = builder.build().fetch_all(&self.pool).await?;
        let mut links = Vec::new();
        let mut deal_ids = std::collections::BTreeSet::new();
        for row in &rows {
            let deal_id = uuid_column(row, "deal_id")?;
            deal_ids.insert(deal_id);
            links.push((
                uuid_column(row, "activity_id")?,
                deal_id,
                row.try_get::<i32, _>("position")?,
            ));
        }
        let deal_ids: Vec<_> = deal_ids.into_iter().collect();
        let mut deals = HashMap::new();
        for batch in deal_ids.chunks(100) {
            use aphel_kernel::application::query::{
                ArchivedFilter, Filter, FilterOperator, RecordQuery,
            };
            let listing = aphel_sdk::record_context::list(
                &self.ctx,
                "deals.deal",
                RecordQuery {
                    filter: Some(Filter::condition(
                        "id",
                        FilterOperator::In,
                        serde_json::json!(batch),
                    )),
                    archived: ArchivedFilter::Include,
                    window: aphel_kernel::application::organization_admin::ListWindow {
                        offset: 0,
                        limit: 100,
                    },
                    ..Default::default()
                },
            )
            .await?;
            for item in listing.items {
                #[derive(serde::Deserialize)]
                struct Snapshot {
                    id: Uuid,
                    #[serde(default)]
                    title: String,
                    #[serde(default)]
                    amount: Option<Decimal>,
                    #[serde(default)]
                    currency: String,
                }
                let item: Snapshot = serde_json::from_value(item).map_err(|error| {
                    AppError::Internal(format!("invalid deal snapshot: {error}"))
                })?;
                if batch.contains(&item.id) {
                    deals.insert(
                        item.id,
                        LinkedDealValue {
                            deal_id: item.id,
                            title: item.title,
                            amount: item.amount,
                            currency: item.currency,
                        },
                    );
                }
            }
        }
        links.sort_by(|left, right| {
            left.0
                .cmp(&right.0)
                .then(left.2.cmp(&right.2))
                .then_with(|| {
                    deals
                        .get(&right.1)
                        .and_then(|deal| deal.amount)
                        .cmp(&deals.get(&left.1).and_then(|deal| deal.amount))
                })
                .then(left.1.cmp(&right.1))
        });
        for (activity_id, deal_id, _) in links {
            if let Some(deal) = deals.get(&deal_id) {
                values.entry(activity_id).or_insert_with(|| deal.clone());
            }
        }
        Ok(values)
    }
}

#[derive(Clone)]
pub(crate) struct RecurrenceStore {
    pool: MySqlPool,
}

impl RecurrenceStore {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    pub(crate) fn of(ctx: &Ctx<'_>) -> Self {
        Self::new(ctx.pool().clone())
    }
}

fn series_from_row(row: &MySqlRow) -> Result<RecurrenceSeries, AppError> {
    let rule = rule_from_row(row, "")?
        .ok_or_else(|| AppError::Internal("recurrence without frequency".to_owned()))?;
    let anchor: NaiveDateTime = row.try_get("anchor_at")?;
    let ended_at: Option<NaiveDateTime> = row.try_get("ended_at")?;
    let priority: String = row.try_get("priority")?;
    let owner_id: Option<String> = row.try_get("owner_id")?;
    let record_type: Option<String> = row.try_get("record_type")?;
    let record_id: Option<String> = row.try_get("record_id")?;
    let links: Option<serde_json::Value> = row.try_get("links")?;
    let links = links
        .and_then(|value| value.as_array().cloned())
        .unwrap_or_default()
        .iter()
        .filter_map(|link| {
            RecordRef::parse(
                link.get("record_type")?.as_str()?,
                link.get("record_id")?.as_str()?,
            )
            .ok()
        })
        .collect();
    Ok(RecurrenceSeries {
        id: uuid_column(row, "id")?,
        rule,
        anchor,
        template: ActivityTemplate {
            title: row.try_get("title")?,
            activity_type_id: uuid_column(row, "activity_type_id")?,
            owner_id: owner_id
                .map(|id| mapper::typed_id(id, UserId::new))
                .transpose()?,
            priority: ActivityPriority::parse(&priority)
                .ok_or_else(|| mapper::invalid_column("priority", &priority))?,
            description: row.try_get("description")?,
            duration_minutes: row.try_get("duration_minutes")?,
            reminder_minutes_before: row.try_get("reminder_minutes_before")?,
            record: match (record_type, record_id) {
                (Some(record_type), Some(record_id)) => {
                    RecordRef::parse(&record_type, &record_id).ok()
                }
                _ => None,
            },
            links,
        },
        ended_at: ended_at.map(mapper::utc),
    })
}

impl RecurrenceStore {
    pub(crate) async fn find(&self, id: Uuid) -> Result<Option<RecurrenceSeries>, AppError> {
        let row = sqlx::query(
            "SELECT id, frequency, interval_count AS `interval`, weekdays, month_day, until_date AS `until`, \
             max_count AS `count`, anchor_at, title, activity_type_id, owner_id, priority, description, \
             duration_minutes, reminder_minutes_before, record_type, record_id, links, ended_at \
             FROM task_recurrences WHERE id = ?",
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await?;
        row.as_ref().map(series_from_row).transpose()
    }

    pub(crate) async fn save(
        &self,
        tx: &mut MySqlTx,
        series: &RecurrenceSeries,
        now: DateTime<Utc>,
    ) -> Result<(), AppError> {
        let template = &series.template;
        let links = serde_json::Value::Array(
            template
                .links
                .iter()
                .map(|link| {
                    serde_json::json!({
                        "record_type": link.record_type.as_str(),
                        "record_id": link.record_id.to_string(),
                    })
                })
                .collect(),
        );
        sqlx::query(
            "INSERT INTO task_recurrences (id, frequency, interval_count, weekdays, month_day, \
             until_date, max_count, anchor_at, title, activity_type_id, owner_id, priority, description, \
             duration_minutes, reminder_minutes_before, record_type, record_id, links, ended_at, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) \
             ON DUPLICATE KEY UPDATE frequency = VALUES(frequency), interval_count = VALUES(interval_count), \
             weekdays = VALUES(weekdays), month_day = VALUES(month_day), until_date = VALUES(until_date), \
             max_count = VALUES(max_count), anchor_at = VALUES(anchor_at), title = VALUES(title), \
             activity_type_id = VALUES(activity_type_id), owner_id = VALUES(owner_id), priority = VALUES(priority), \
             description = VALUES(description), duration_minutes = VALUES(duration_minutes), \
             reminder_minutes_before = VALUES(reminder_minutes_before), record_type = VALUES(record_type), \
             record_id = VALUES(record_id), links = VALUES(links), ended_at = VALUES(ended_at), \
             updated_at = VALUES(updated_at)",
        )
        .bind(series.id.to_string())
        .bind(series.rule.frequency.as_str())
        .bind(series.rule.interval)
        .bind(weekdays_text(&series.rule.weekdays))
        .bind(series.rule.month_day.and_then(|day| u8::try_from(day).ok()))
        .bind(series.rule.until)
        .bind(series.rule.count)
        .bind(series.anchor)
        .bind(&template.title)
        .bind(template.activity_type_id.to_string())
        .bind(template.owner_id.as_ref().map(UserId::as_str))
        .bind(template.priority.as_str())
        .bind(&template.description)
        .bind(template.duration_minutes)
        .bind(template.reminder_minutes_before)
        .bind(template.record.as_ref().map(|record| record.record_type.as_str()))
        .bind(template.record.as_ref().map(|record| record.record_id.to_string()))
        .bind(links)
        .bind(series.ended_at.map(mapper::naive))
        .bind(mapper::naive(now))
        .bind(mapper::naive(now))
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub(crate) async fn occurrence_exists(
        &self,
        series_id: Uuid,
        index: u32,
    ) -> Result<bool, AppError> {
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM tasks WHERE recurrence_id = ? \
             AND recurrence_index = ?)",
        )
        .bind(series_id.to_string())
        .bind(index)
        .fetch_one(&self.pool)
        .await?;
        Ok(exists)
    }
}

const ACTIVITY_TYPE_COLUMNS: &str =
    "id, `key`, name, icon, color, is_system, is_active, position, created_at, updated_at";

#[derive(Clone)]
pub struct ActivityTypeStore {
    pool: MySqlPool,
}

impl ActivityTypeStore {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    pub(crate) fn of(ctx: &Ctx<'_>) -> Self {
        Self::new(ctx.pool().clone())
    }
}

fn activity_type_from_row(row: &MySqlRow) -> Result<ActivityTypeView, AppError> {
    let created_at: NaiveDateTime = row.try_get("created_at")?;
    let updated_at: NaiveDateTime = row.try_get("updated_at")?;
    Ok(ActivityTypeView {
        id: uuid_column(row, "id")?,
        key: row.try_get("key")?,
        name: row.try_get("name")?,
        icon: row.try_get("icon")?,
        color: row.try_get("color")?,
        is_system: row.try_get("is_system")?,
        is_active: row.try_get("is_active")?,
        position: row.try_get("position")?,
        created_at: mapper::utc(created_at),
        updated_at: mapper::utc(updated_at),
    })
}

fn key_conflict(error: sqlx::Error) -> AppError {
    match &error {
        sqlx::Error::Database(database) if database.is_unique_violation() => {
            AppError::conflict_code(
                ACTIVITY_TYPE_KEY_TAKEN,
                "another activity type already uses this key",
            )
        }
        _ => error.into(),
    }
}

impl ActivityTypeStore {
    pub async fn list(&self, include_inactive: bool) -> Result<Vec<ActivityTypeView>, AppError> {
        let rows = sqlx::query(sqlx::AssertSqlSafe(format!(
            "SELECT {ACTIVITY_TYPE_COLUMNS} FROM task_types \
             WHERE (is_active OR ?) ORDER BY position, name, id"
        )))
        .bind(include_inactive)
        .fetch_all(&self.pool)
        .await?;
        rows.iter().map(activity_type_from_row).collect()
    }

    pub(crate) async fn find(&self, id: Uuid) -> Result<Option<ActivityTypeView>, AppError> {
        let row = sqlx::query(sqlx::AssertSqlSafe(format!(
            "SELECT {ACTIVITY_TYPE_COLUMNS} FROM task_types WHERE id = ?"
        )))
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await?;
        row.as_ref().map(activity_type_from_row).transpose()
    }

    pub(crate) async fn key_exists(&self, key: &str) -> Result<bool, AppError> {
        let exists: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM task_types WHERE `key` = ?)")
                .bind(key)
                .fetch_one(&self.pool)
                .await?;
        Ok(exists)
    }

    pub(crate) async fn insert(
        &self,
        tx: &mut MySqlTx,
        activity_type: &ActivityTypeView,
    ) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO task_types (id, `key`, name, icon, color, is_system, is_active, position, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(activity_type.id.to_string())
        .bind(&activity_type.key)
        .bind(&activity_type.name)
        .bind(&activity_type.icon)
        .bind(&activity_type.color)
        .bind(activity_type.is_system)
        .bind(activity_type.is_active)
        .bind(activity_type.position)
        .bind(mapper::naive(activity_type.created_at))
        .bind(mapper::naive(activity_type.updated_at))
        .execute(&mut **tx)
        .await
        .map_err(key_conflict)?;
        Ok(())
    }

    pub async fn update(
        &self,
        tx: &mut MySqlTx,
        activity_type: &ActivityTypeView,
    ) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE task_types SET name = ?, icon = ?, color = ?, is_active = ?, updated_at = ? \
             WHERE id = ?",
        )
        .bind(&activity_type.name)
        .bind(&activity_type.icon)
        .bind(&activity_type.color)
        .bind(activity_type.is_active)
        .bind(mapper::naive(activity_type.updated_at))
        .bind(activity_type.id.to_string())
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub(crate) async fn reorder(
        &self,
        tx: &mut MySqlTx,
        ordered_ids: &[Uuid],
    ) -> Result<(), AppError> {
        for (position, id) in ordered_ids.iter().enumerate() {
            sqlx::query("UPDATE task_types SET position = ? WHERE id = ?")
                .bind(position as i32)
                .bind(id.to_string())
                .execute(&mut **tx)
                .await?;
        }
        Ok(())
    }
}

pub(crate) async fn hydrate_link_titles(
    ctx: &Ctx<'_>,
    mut links: Vec<&mut LinkedRecord>,
) -> Result<(), AppError> {
    use aphel_kernel::application::query::RecordQueryPorts;
    let mut groups: HashMap<RecordType, std::collections::BTreeSet<Uuid>> = HashMap::new();
    for link in &links {
        groups
            .entry(link.record.record_type.clone())
            .or_default()
            .insert(link.record.record_id);
    }
    for (record_type, ids) in groups {
        let ids: Vec<_> = ids.into_iter().collect();
        let mut labels = HashMap::new();
        for batch in ids.chunks(1000) {
            labels.extend(
                aphel_sdk::records::visible_record_labels(
                    ctx.pool(),
                    ctx.registry(),
                    ctx.query().definitions(),
                    ctx.scope,
                    &record_type,
                    batch,
                )
                .await?,
            );
        }
        for link in &mut links {
            if link.record.record_type == record_type {
                link.title = labels
                    .get(&link.record.record_id)
                    .and_then(|label| label.title.clone());
            }
        }
    }
    Ok(())
}
