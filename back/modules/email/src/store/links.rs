use chrono::{DateTime, NaiveDateTime, Utc};
use sqlx::{MySql, MySqlPool, QueryBuilder, Row};
use uuid::Uuid;

use crate::domain::LinkOrigin;
use crate::{NewRecordLink, RecordLinkView};
use aphel_kernel::Ctx;
use aphel_kernel::domain::UserId;
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::application::shared::RecordRef;
use aphel_sdk::platform::database::transaction::MySqlTx;
use aphel_sdk::platform::infrastructure::fields::mysql_definitions::uuid_column;
use aphel_sdk::platform::infrastructure::repositories::mapper;

#[derive(Clone)]
pub(crate) struct LinkStore {
    pool: MySqlPool,
}

impl LinkStore {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    pub(crate) fn of(ctx: &Ctx<'_>) -> Self {
        Self::new(ctx.pool().clone())
    }
}

impl LinkStore {
    pub(crate) async fn link(
        &self,
        tx: &mut MySqlTx,
        link: &NewRecordLink,
        now: DateTime<Utc>,
    ) -> Result<bool, AppError> {
        let result = sqlx::query(
            "INSERT INTO email_record_links (id, thread_id, record_type, record_id, origin, created_by, created_at) \
             SELECT ?, t.id, ?, ?, ?, ?, ? FROM email_threads t WHERE t.id = ? \
             ON DUPLICATE KEY UPDATE origin = IF(? = 'manual', 'manual', email_record_links.origin)",
        )
        .bind(Uuid::now_v7().to_string())
        .bind(link.record.record_type.as_str())
        .bind(link.record.record_id.to_string())
        .bind(link.origin.as_str())
        .bind(link.created_by.as_ref().map(|user| user.as_str().to_owned()))
        .bind(mapper::naive(now))
        .bind(link.thread_id.to_string())
        .bind(link.origin.as_str())
        .execute(&mut **tx)
        .await?;
        Ok(result.rows_affected() == 1)
    }

    pub(crate) async fn unlink(
        &self,
        tx: &mut MySqlTx,
        thread_id: Uuid,
        record: &RecordRef,
    ) -> Result<bool, AppError> {
        let result = sqlx::query(
            "DELETE FROM email_record_links WHERE thread_id = ? AND record_type = ? AND record_id = ?",
        )
        .bind(thread_id.to_string())
        .bind(record.record_type.as_str())
        .bind(record.record_id.to_string())
        .execute(&mut **tx)
        .await?;
        Ok(result.rows_affected() == 1)
    }

    pub(crate) async fn links_for_threads(
        &self,
        thread_ids: &[Uuid],
    ) -> Result<Vec<RecordLinkView>, AppError> {
        if thread_ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut builder = QueryBuilder::<MySql>::new(
            "SELECT id, thread_id, record_type, record_id, origin, created_by, created_at \
             FROM email_record_links WHERE thread_id IN (",
        );
        let mut separated = builder.separated(", ");
        for id in thread_ids {
            separated.push_bind(id.to_string());
        }
        separated.push_unseparated(") ORDER BY thread_id, created_at, id");
        let rows = builder.build().fetch_all(&self.pool).await?;
        rows.iter()
            .map(|row| {
                let record_type: String = row.try_get("record_type")?;
                let record_id: String = row.try_get("record_id")?;
                let origin: String = row.try_get("origin")?;
                let created_by: Option<String> = row.try_get("created_by")?;
                let created_at: NaiveDateTime = row.try_get("created_at")?;
                Ok(RecordLinkView {
                    id: uuid_column(row, "id")?,
                    thread_id: uuid_column(row, "thread_id")?,
                    record: RecordRef::parse(&record_type, &record_id)
                        .map_err(|_| mapper::invalid_column("record_type", &record_type))?,
                    origin: LinkOrigin::parse(&origin)
                        .ok_or_else(|| mapper::invalid_column("origin", &origin))?,
                    created_by: created_by
                        .map(|value| mapper::typed_id(value, UserId::new))
                        .transpose()?,
                    created_at: mapper::utc(created_at),
                })
            })
            .collect()
    }
}
