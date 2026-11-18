use aphel_sdk::calendar::BusyInterval;
use chrono::{DateTime, NaiveDateTime, Utc};
use chrono_tz::Tz;
use serde_json::{Map, Value, json};
use sqlx::mysql::MySqlRow;
use sqlx::types::Json;
use sqlx::{MySql, MySqlPool, QueryBuilder, Row};
use uuid::Uuid;

use crate::{
    Availability, Booking, BookingFilter, BookingStatus, HostProfile, LocationKind, MeetingType,
    MeetingTypeFilter, OrganizationProfile, Question, QuestionKind, WeeklyInterval, parse_clock,
};
use aphel_kernel::Ctx;
use aphel_kernel::domain::UserId;
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::application::organization_admin::Listing;
use aphel_sdk::platform::database::transaction::Tx;
use aphel_sdk::platform::domain::access::RecordScopeFilter;
use aphel_sdk::platform::infrastructure::repositories::mapper;

#[derive(Clone)]
pub(crate) struct BookingStore {
    pool: MySqlPool,
}

impl BookingStore {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    pub(crate) fn of(ctx: &Ctx<'_>) -> Self {
        Self::new(ctx.pool().clone())
    }
}

const MEETING_TYPE_COLUMNS: &str = "SELECT mt.id, mt.owner_id, u.display_name AS owner_name, \
mt.slug, mt.name, mt.description, mt.duration_minutes, mt.slot_interval_minutes, mt.location_kind, \
mt.location_value, mt.questions, mt.spam_protection, mt.distribution_rule_id, NULL AS rule_name, \
mt.is_active, mt.created_at, mt.updated_at \
FROM meeting_types mt JOIN users u ON u.id = mt.owner_id ";

const BOOKING_COLUMNS: &str = "SELECT b.id, b.meeting_type_id, mt.name AS meeting_type_name, b.host_user_id, \
u.display_name AS host_name, b.record_type, b.contact_id, b.activity_id, b.status, b.starts_at, b.ends_at, b.guest_name, \
b.guest_email, b.guest_timezone, b.guest_locale, b.answers, b.revision, b.cancel_reason, b.canceled_at, \
b.created_at, b.updated_at \
FROM meeting_bookings b JOIN meeting_types mt ON mt.id = b.meeting_type_id \
JOIN users u ON u.id = b.host_user_id ";

fn uuid_column(row: &MySqlRow, column: &str) -> Result<Uuid, AppError> {
    let value: String = row.try_get(column)?;
    Uuid::parse_str(&value).map_err(|_| mapper::invalid_column(column, &value))
}

fn optional_uuid(row: &MySqlRow, column: &str) -> Result<Option<Uuid>, AppError> {
    let value: Option<String> = row.try_get(column)?;
    value
        .map(|value| Uuid::parse_str(&value).map_err(|_| mapper::invalid_column(column, &value)))
        .transpose()
}

fn minutes(row: &MySqlRow, column: &str) -> Result<u32, AppError> {
    let value: i32 = row.try_get(column)?;
    Ok(u32::try_from(value).unwrap_or(0))
}

fn utc(row: &MySqlRow, column: &str) -> Result<DateTime<Utc>, AppError> {
    let value: NaiveDateTime = row.try_get(column)?;
    Ok(mapper::utc(value))
}

fn weekly_json(intervals: &[WeeklyInterval]) -> Json<Value> {
    Json(Value::Array(
        intervals
            .iter()
            .map(|interval| {
                json!({
                    "weekday": interval.weekday,
                    "start": interval.start_text(),
                    "end": interval.end_text(),
                })
            })
            .collect(),
    ))
}

fn weekly_from(value: &Value) -> Vec<WeeklyInterval> {
    value
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|item| {
                    Some(WeeklyInterval {
                        weekday: u8::try_from(item.get("weekday")?.as_u64()?).ok()?,
                        start_minute: parse_clock(item.get("start")?.as_str()?)?,
                        end_minute: parse_clock(item.get("end")?.as_str()?)?,
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

fn questions_json(questions: &[Question]) -> Json<Value> {
    Json(Value::Array(
        questions
            .iter()
            .map(|question| {
                json!({
                    "key": question.key,
                    "label": question.label,
                    "kind": question.kind.as_str(),
                    "required": question.required,
                    "contact_field": question.contact_field,
                })
            })
            .collect(),
    ))
}

fn questions_from(value: &Value) -> Vec<Question> {
    value
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|item| {
                    Some(Question {
                        key: item.get("key")?.as_str()?.to_owned(),
                        label: item.get("label")?.as_str()?.to_owned(),
                        kind: QuestionKind::parse(item.get("kind")?.as_str()?)?,
                        required: item
                            .get("required")
                            .and_then(Value::as_bool)
                            .unwrap_or(false),
                        contact_field: item
                            .get("contact_field")
                            .and_then(Value::as_str)
                            .map(str::to_owned),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

fn availability_from(row: &MySqlRow) -> Result<Availability, AppError> {
    let timezone: String = row.try_get("timezone")?;
    let Json(weekly): Json<Value> = row.try_get("weekly_hours")?;
    let max_per_day: Option<i32> = row.try_get("max_per_day")?;
    Ok(Availability {
        user_id: mapper::typed_id(row.try_get("user_id")?, UserId::new)?,
        timezone: timezone
            .parse::<Tz>()
            .map_err(|_| mapper::invalid_column("booking_availability.timezone", &timezone))?,
        weekly: weekly_from(&weekly),
        min_notice_minutes: minutes(row, "min_notice_minutes")?,
        max_per_day: max_per_day.and_then(|value| u32::try_from(value).ok()),
        buffer_before_minutes: minutes(row, "buffer_before_minutes")?,
        buffer_after_minutes: minutes(row, "buffer_after_minutes")?,
        horizon_days: minutes(row, "horizon_days")?,
        configured: true,
        updated_at: Some(utc(row, "updated_at")?),
    })
}

fn meeting_type_from(row: &MySqlRow) -> Result<MeetingType, AppError> {
    let location: String = row.try_get("location_kind")?;
    let Json(questions): Json<Value> = row.try_get("questions")?;
    Ok(MeetingType {
        id: uuid_column(row, "id")?,
        owner_id: mapper::typed_id(row.try_get("owner_id")?, UserId::new)?,
        owner_name: row.try_get("owner_name")?,
        slug: row.try_get("slug")?,
        name: row.try_get("name")?,
        description: row.try_get("description")?,
        duration_minutes: minutes(row, "duration_minutes")?,
        slot_interval_minutes: minutes(row, "slot_interval_minutes")?,
        location_kind: LocationKind::parse(&location)
            .ok_or_else(|| mapper::invalid_column("meeting_types.location_kind", &location))?,
        location_value: row.try_get("location_value")?,
        questions: questions_from(&questions),
        spam_protection: row.try_get("spam_protection")?,
        distribution_rule_id: optional_uuid(row, "distribution_rule_id")?,
        distribution_rule_name: row.try_get("rule_name")?,
        is_active: row.try_get("is_active")?,
        created_at: utc(row, "created_at")?,
        updated_at: utc(row, "updated_at")?,
    })
}

fn booking_from(row: &MySqlRow) -> Result<Booking, AppError> {
    let status: String = row.try_get("status")?;
    let Json(answers): Json<Value> = row.try_get("answers")?;
    let revision: i32 = row.try_get("revision")?;
    let canceled_at: Option<NaiveDateTime> = row.try_get("canceled_at")?;
    Ok(Booking {
        id: uuid_column(row, "id")?,
        meeting_type_id: uuid_column(row, "meeting_type_id")?,
        meeting_type_name: row.try_get("meeting_type_name")?,
        host_user_id: mapper::typed_id(row.try_get("host_user_id")?, UserId::new)?,
        host_name: row.try_get("host_name")?,
        record_type: row.try_get("record_type")?,
        contact_id: optional_uuid(row, "contact_id")?,
        activity_id: optional_uuid(row, "activity_id")?,
        status: BookingStatus::parse(&status)
            .ok_or_else(|| mapper::invalid_column("meeting_bookings.status", &status))?,
        starts_at: utc(row, "starts_at")?,
        ends_at: utc(row, "ends_at")?,
        guest_name: row.try_get("guest_name")?,
        guest_email: row.try_get("guest_email")?,
        guest_timezone: row.try_get("guest_timezone")?,
        guest_locale: row.try_get("guest_locale")?,
        answers: match answers {
            Value::Object(map) => map,
            _ => Map::new(),
        },
        revision: u32::try_from(revision).unwrap_or(0),
        cancel_reason: row.try_get("cancel_reason")?,
        canceled_at: canceled_at.map(mapper::utc),
        created_at: utc(row, "created_at")?,
        updated_at: utc(row, "updated_at")?,
    })
}

fn push_owner_filter(builder: &mut QueryBuilder<MySql>, column: &str, filter: &RecordScopeFilter) {
    match filter {
        RecordScopeFilter::Organization => {}
        RecordScopeFilter::Owner { user_id, .. } => {
            builder
                .push(format!(" AND {column} = "))
                .push_bind(user_id.as_str().to_owned());
        }
        RecordScopeFilter::Teams {
            user_id, team_ids, ..
        } => {
            builder
                .push(format!(" AND ({column} = "))
                .push_bind(user_id.as_str().to_owned());
            if !team_ids.is_empty() {
                builder
                    .push(format!(
                        " OR {column} IN (SELECT tm.user_id FROM team_memberships tm WHERE tm.team_id IN ("
                    ));
                let mut separated = builder.separated(", ");
                for team in team_ids {
                    separated.push_bind(team.as_str().to_owned());
                }
                builder.push("))");
            }
            builder.push(")");
        }
    }
}

fn intervals(rows: Vec<MySqlRow>, from: DateTime<Utc>) -> Result<Vec<BusyInterval>, AppError> {
    let mut busy = Vec::new();
    for row in rows {
        let starts_at = utc(&row, "starts_at")?;
        let ends_at = utc(&row, "ends_at")?;
        if ends_at > from {
            busy.push(BusyInterval::new(starts_at, ends_at));
        }
    }
    Ok(busy)
}

fn booked_query(
    user_id: &UserId,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
    exclude: Option<Uuid>,
    lock: bool,
) -> QueryBuilder<MySql> {
    let mut builder = QueryBuilder::<MySql>::new(
        "SELECT starts_at, ends_at FROM meeting_bookings WHERE host_user_id = ",
    );
    builder
        .push_bind(user_id.as_str().to_owned())
        .push(" AND status = 'confirmed' AND starts_at < ")
        .push_bind(mapper::naive(to))
        .push(" AND ends_at > ")
        .push_bind(mapper::naive(from));
    if let Some(exclude) = exclude {
        builder.push(" AND id <> ").push_bind(exclude.to_string());
    }
    if lock {
        builder.push(" FOR UPDATE");
    }
    builder
}

impl BookingStore {
    pub(crate) async fn organization(&self) -> Result<OrganizationProfile, AppError> {
        let row = sqlx::query("SELECT name, timezone FROM organization_settings WHERE id = 1")
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| AppError::NotFound("organization was not found".to_owned()))?;
        let timezone: String = row.try_get("timezone")?;
        Ok(OrganizationProfile {
            name: row.try_get("name")?,
            timezone: timezone
                .parse::<Tz>()
                .unwrap_or(chrono_tz::America::Sao_Paulo),
        })
    }

    pub(crate) async fn host(&self, user_id: &UserId) -> Result<Option<HostProfile>, AppError> {
        let row = sqlx::query(
            "SELECT u.id, u.display_name, u.email, u.timezone, u.locale \
             FROM users u WHERE u.id = ? AND u.status = 'active'",
        )
        .bind(user_id.as_str())
        .fetch_optional(&self.pool)
        .await?;
        row.map(|row| {
            Ok(HostProfile {
                user_id: mapper::typed_id(row.try_get("id")?, UserId::new)?,
                name: row.try_get("display_name")?,
                email: row.try_get("email")?,
                timezone: row.try_get("timezone")?,
                locale: row.try_get("locale")?,
            })
        })
        .transpose()
    }

    pub(crate) async fn user_within(
        &self,
        user_id: &UserId,
        filter: &RecordScopeFilter,
    ) -> Result<bool, AppError> {
        let mut builder =
            QueryBuilder::<MySql>::new("SELECT EXISTS(SELECT 1 FROM users m WHERE m.id = ");
        builder.push_bind(user_id.as_str().to_owned());
        push_owner_filter(&mut builder, "m.id", filter);
        builder.push(")");
        let exists: bool = builder.build_query_scalar().fetch_one(&self.pool).await?;
        Ok(exists)
    }

    pub(crate) async fn availability(
        &self,
        user_id: &UserId,
    ) -> Result<Option<Availability>, AppError> {
        sqlx::query(
            "SELECT user_id, timezone, weekly_hours, min_notice_minutes, max_per_day, buffer_before_minutes, \
             buffer_after_minutes, horizon_days, updated_at FROM booking_availability \
             WHERE user_id = ?",
        )
        .bind(user_id.as_str())
        .fetch_optional(&self.pool)
        .await?
        .map(|row| availability_from(&row))
        .transpose()
    }

    pub(crate) async fn save_availability(
        &self,
        tx: &mut Tx,
        availability: &Availability,
        now: DateTime<Utc>,
    ) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO booking_availability (user_id, timezone, weekly_hours, \
             min_notice_minutes, max_per_day, buffer_before_minutes, buffer_after_minutes, horizon_days, \
             created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?) \
             ON DUPLICATE KEY UPDATE timezone = VALUES(timezone), weekly_hours = VALUES(weekly_hours), \
             min_notice_minutes = VALUES(min_notice_minutes), max_per_day = VALUES(max_per_day), \
             buffer_before_minutes = VALUES(buffer_before_minutes), \
             buffer_after_minutes = VALUES(buffer_after_minutes), horizon_days = VALUES(horizon_days), \
             updated_at = VALUES(updated_at)",
        )
        .bind(availability.user_id.as_str())
        .bind(availability.timezone.name())
        .bind(weekly_json(&availability.weekly))
        .bind(availability.min_notice_minutes)
        .bind(availability.max_per_day)
        .bind(availability.buffer_before_minutes)
        .bind(availability.buffer_after_minutes)
        .bind(availability.horizon_days)
        .bind(mapper::naive(now))
        .bind(mapper::naive(now))
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub(crate) async fn insert_availability_if_absent(
        &self,
        availability: &Availability,
        now: DateTime<Utc>,
    ) -> Result<(), AppError> {
        sqlx::query(
            "INSERT IGNORE INTO booking_availability (user_id, timezone, weekly_hours, \
             min_notice_minutes, max_per_day, buffer_before_minutes, buffer_after_minutes, horizon_days, \
             created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(availability.user_id.as_str())
        .bind(availability.timezone.name())
        .bind(weekly_json(&availability.weekly))
        .bind(availability.min_notice_minutes)
        .bind(availability.max_per_day)
        .bind(availability.buffer_before_minutes)
        .bind(availability.buffer_after_minutes)
        .bind(availability.horizon_days)
        .bind(mapper::naive(now))
        .bind(mapper::naive(now))
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub(crate) async fn lock_availability(
        &self,
        tx: &mut Tx,
        user_id: &UserId,
    ) -> Result<bool, AppError> {
        let row =
            sqlx::query("SELECT user_id FROM booking_availability WHERE user_id = ? FOR UPDATE")
                .bind(user_id.as_str())
                .fetch_optional(&mut **tx)
                .await?;
        Ok(row.is_some())
    }

    pub(crate) async fn list_meeting_types(
        &self,
        filter: &MeetingTypeFilter,
    ) -> Result<Listing<MeetingType>, AppError> {
        let mut count =
            QueryBuilder::<MySql>::new("SELECT COUNT(*) FROM meeting_types mt WHERE TRUE");
        push_owner_filter(&mut count, "mt.owner_id", &filter.owners);
        let total: i64 = count.build_query_scalar().fetch_one(&self.pool).await?;
        let mut builder = QueryBuilder::<MySql>::new(MEETING_TYPE_COLUMNS);
        builder.push("WHERE TRUE");
        push_owner_filter(&mut builder, "mt.owner_id", &filter.owners);
        builder
            .push(" ORDER BY mt.name, mt.id LIMIT ")
            .push_bind(filter.window.limit)
            .push(" OFFSET ")
            .push_bind(filter.window.offset);
        let rows = builder.build().fetch_all(&self.pool).await?;
        let mut items = rows
            .iter()
            .map(meeting_type_from)
            .collect::<Result<Vec<_>, _>>()?;
        self.hydrate_rule_names(&mut items).await?;
        Ok(Listing {
            items,
            total: u64::try_from(total).unwrap_or_default(),
        })
    }

    async fn hydrate_rule_names(&self, items: &mut [MeetingType]) -> Result<(), AppError> {
        let ids: Vec<_> = items
            .iter()
            .filter_map(|item| item.distribution_rule_id)
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect();
        let mut names = std::collections::HashMap::new();
        if let Some(source) = aphel_sdk::assignment::host_assignment() {
            for batch in ids.chunks(1000) {
                names.extend(source.rule_names(&self.pool, batch).await?);
            }
        }
        for item in items {
            item.distribution_rule_name = item
                .distribution_rule_id
                .and_then(|id| names.get(&id).cloned());
        }
        Ok(())
    }

    pub(crate) async fn meeting_type(&self, id: Uuid) -> Result<Option<MeetingType>, AppError> {
        let mut item = sqlx::query(sqlx::AssertSqlSafe(format!(
            "{MEETING_TYPE_COLUMNS}WHERE mt.id = ?"
        )))
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await?
        .map(|row| meeting_type_from(&row))
        .transpose()?;
        if let Some(item) = &mut item {
            self.hydrate_rule_names(std::slice::from_mut(item)).await?;
        }
        Ok(item)
    }

    pub(crate) async fn meeting_type_by_slug(
        &self,
        slug: &str,
    ) -> Result<Option<MeetingType>, AppError> {
        let mut item = sqlx::query(sqlx::AssertSqlSafe(format!(
            "{MEETING_TYPE_COLUMNS}WHERE mt.slug = ?"
        )))
        .bind(slug)
        .fetch_optional(&self.pool)
        .await?
        .map(|row| meeting_type_from(&row))
        .transpose()?;
        if let Some(item) = &mut item {
            self.hydrate_rule_names(std::slice::from_mut(item)).await?;
        }
        Ok(item)
    }

    pub(crate) async fn slug_taken(
        &self,
        slug: &str,
        except: Option<Uuid>,
    ) -> Result<bool, AppError> {
        let taken: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM meeting_types WHERE slug = ? AND (? IS NULL OR id <> ?))",
        )
        .bind(slug)
        .bind(except.map(|id| id.to_string()))
        .bind(except.map(|id| id.to_string()))
        .fetch_one(&self.pool)
        .await?;
        Ok(taken)
    }

    pub(crate) async fn insert_meeting_type(
        &self,
        tx: &mut Tx,
        meeting_type: &MeetingType,
    ) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO meeting_types (id, owner_id, slug, name, description, duration_minutes, \
             slot_interval_minutes, location_kind, location_value, questions, spam_protection, \
             distribution_rule_id, is_active, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(meeting_type.id.to_string())
        .bind(meeting_type.owner_id.as_str())
        .bind(&meeting_type.slug)
        .bind(&meeting_type.name)
        .bind(&meeting_type.description)
        .bind(meeting_type.duration_minutes)
        .bind(meeting_type.slot_interval_minutes)
        .bind(meeting_type.location_kind.as_str())
        .bind(&meeting_type.location_value)
        .bind(questions_json(&meeting_type.questions))
        .bind(meeting_type.spam_protection)
        .bind(meeting_type.distribution_rule_id.map(|id| id.to_string()))
        .bind(meeting_type.is_active)
        .bind(mapper::naive(meeting_type.created_at))
        .bind(mapper::naive(meeting_type.updated_at))
        .execute(&mut **tx)
        .await
        .map_err(slug_conflict)?;
        Ok(())
    }

    pub(crate) async fn update_meeting_type(
        &self,
        tx: &mut Tx,
        meeting_type: &MeetingType,
    ) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE meeting_types SET slug = ?, name = ?, description = ?, duration_minutes = ?, \
             slot_interval_minutes = ?, location_kind = ?, location_value = ?, questions = ?, \
             spam_protection = ?, distribution_rule_id = ?, is_active = ?, updated_at = ? \
             WHERE id = ?",
        )
        .bind(&meeting_type.slug)
        .bind(&meeting_type.name)
        .bind(&meeting_type.description)
        .bind(meeting_type.duration_minutes)
        .bind(meeting_type.slot_interval_minutes)
        .bind(meeting_type.location_kind.as_str())
        .bind(&meeting_type.location_value)
        .bind(questions_json(&meeting_type.questions))
        .bind(meeting_type.spam_protection)
        .bind(meeting_type.distribution_rule_id.map(|id| id.to_string()))
        .bind(meeting_type.is_active)
        .bind(mapper::naive(meeting_type.updated_at))
        .bind(meeting_type.id.to_string())
        .execute(&mut **tx)
        .await
        .map_err(slug_conflict)?;
        Ok(())
    }

    pub(crate) async fn delete_meeting_type(&self, tx: &mut Tx, id: Uuid) -> Result<(), AppError> {
        sqlx::query("DELETE FROM meeting_types WHERE id = ?")
            .bind(id.to_string())
            .execute(&mut **tx)
            .await?;
        Ok(())
    }

    pub(crate) async fn booked(
        &self,
        user_id: &UserId,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
        exclude: Option<Uuid>,
    ) -> Result<Vec<BusyInterval>, AppError> {
        let rows = booked_query(user_id, from, to, exclude, false)
            .build()
            .fetch_all(&self.pool)
            .await?;
        intervals(rows, from)
    }

    pub(crate) async fn booked_locked(
        &self,
        tx: &mut Tx,
        user_id: &UserId,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
        exclude: Option<Uuid>,
    ) -> Result<Vec<BusyInterval>, AppError> {
        let rows = booked_query(user_id, from, to, exclude, true)
            .build()
            .fetch_all(&mut **tx)
            .await?;
        intervals(rows, from)
    }

    pub(crate) async fn booking(&self, id: Uuid) -> Result<Option<Booking>, AppError> {
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "{BOOKING_COLUMNS}WHERE b.id = ?"
        )))
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await?
        .map(|row| booking_from(&row))
        .transpose()
    }

    pub(crate) async fn list_bookings(
        &self,
        filter: &BookingFilter,
    ) -> Result<Listing<Booking>, AppError> {
        fn conditions(builder: &mut QueryBuilder<MySql>, filter: &BookingFilter) {
            builder.push("WHERE TRUE");
            push_owner_filter(builder, "b.host_user_id", &filter.hosts);
            if let Some(status) = filter.status {
                builder.push(" AND b.status = ").push_bind(status.as_str());
            }
            if let Some(meeting_type_id) = filter.meeting_type_id {
                builder
                    .push(" AND b.meeting_type_id = ")
                    .push_bind(meeting_type_id.to_string());
            }
            if let Some(after) = filter.after {
                builder
                    .push(" AND b.ends_at > ")
                    .push_bind(mapper::naive(after));
            }
            if let Some((from, to)) = filter.between {
                builder
                    .push(" AND b.starts_at < ")
                    .push_bind(mapper::naive(to))
                    .push(" AND b.ends_at > ")
                    .push_bind(mapper::naive(from));
            }
            if filter.without_activity {
                builder.push(" AND b.activity_id IS NULL");
            }
        }
        let mut count = QueryBuilder::<MySql>::new("SELECT COUNT(*) FROM meeting_bookings b ");
        conditions(&mut count, filter);
        let total: i64 = count.build_query_scalar().fetch_one(&self.pool).await?;
        let mut builder = QueryBuilder::<MySql>::new(BOOKING_COLUMNS);
        conditions(&mut builder, filter);
        if filter.after.is_some() {
            builder.push(" ORDER BY b.starts_at ASC, b.id");
        } else {
            builder.push(" ORDER BY b.starts_at DESC, b.id");
        }
        builder
            .push(" LIMIT ")
            .push_bind(filter.window.limit)
            .push(" OFFSET ")
            .push_bind(filter.window.offset);
        let rows = builder.build().fetch_all(&self.pool).await?;
        Ok(Listing {
            items: rows.iter().map(booking_from).collect::<Result<_, _>>()?,
            total: u64::try_from(total).unwrap_or_default(),
        })
    }

    pub(crate) async fn insert_booking(
        &self,
        tx: &mut Tx,
        booking: &Booking,
    ) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO meeting_bookings (id, meeting_type_id, host_user_id, record_type, contact_id, \
             activity_id, status, starts_at, ends_at, guest_name, guest_email, guest_timezone, guest_locale, \
             answers, revision, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(booking.id.to_string())
        .bind(booking.meeting_type_id.to_string())
        .bind(booking.host_user_id.as_str())
        .bind(&booking.record_type)
        .bind(booking.contact_id.map(|id| id.to_string()))
        .bind(booking.activity_id.map(|id| id.to_string()))
        .bind(booking.status.as_str())
        .bind(mapper::naive(booking.starts_at))
        .bind(mapper::naive(booking.ends_at))
        .bind(&booking.guest_name)
        .bind(&booking.guest_email)
        .bind(&booking.guest_timezone)
        .bind(&booking.guest_locale)
        .bind(Json(Value::Object(booking.answers.clone())))
        .bind(booking.revision)
        .bind(mapper::naive(booking.created_at))
        .bind(mapper::naive(booking.updated_at))
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub(crate) async fn move_booking(
        &self,
        tx: &mut Tx,
        booking: &Booking,
        now: DateTime<Utc>,
    ) -> Result<(), AppError> {
        let result = sqlx::query(
            "UPDATE meeting_bookings SET starts_at = ?, ends_at = ?, revision = ?, updated_at = ? \
             WHERE id = ? AND status = 'confirmed' AND revision = ?",
        )
        .bind(mapper::naive(booking.starts_at))
        .bind(mapper::naive(booking.ends_at))
        .bind(booking.revision)
        .bind(mapper::naive(now))
        .bind(booking.id.to_string())
        .bind(booking.revision.saturating_sub(1))
        .execute(&mut **tx)
        .await?;
        changed(result.rows_affected())
    }

    pub(crate) async fn cancel_booking(
        &self,
        tx: &mut Tx,
        booking: &Booking,
        now: DateTime<Utc>,
    ) -> Result<(), AppError> {
        let result = sqlx::query(
            "UPDATE meeting_bookings SET status = 'canceled', cancel_reason = ?, canceled_at = ?, revision = ?, \
             updated_at = ? WHERE id = ? AND status = 'confirmed' AND revision = ?",
        )
        .bind(&booking.cancel_reason)
        .bind(mapper::naive(now))
        .bind(booking.revision)
        .bind(mapper::naive(now))
        .bind(booking.id.to_string())
        .bind(booking.revision.saturating_sub(1))
        .execute(&mut **tx)
        .await?;
        changed(result.rows_affected())
    }
}

fn changed(rows: u64) -> Result<(), AppError> {
    if rows == 0 {
        Err(AppError::conflict_code(
            crate::BOOKING_NOT_CHANGEABLE,
            "this booking was canceled or already happened",
        ))
    } else {
        Ok(())
    }
}

fn slug_conflict(error: sqlx::Error) -> AppError {
    let duplicate = error
        .as_database_error()
        .is_some_and(|database| database.is_unique_violation());
    if duplicate {
        AppError::conflict_code(
            crate::MEETING_TYPE_SLUG_TAKEN,
            "another booking link already uses this address",
        )
    } else {
        AppError::from(error)
    }
}
