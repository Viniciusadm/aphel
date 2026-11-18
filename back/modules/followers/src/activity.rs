use std::collections::HashSet;

use aphel_kernel::domain::UserId;
use aphel_kernel::errors::AppError;
use aphel_sdk::dashboard::ActivityScope;
use aphel_sdk::platform::application::shared::RecordRef;
use aphel_sdk::platform::database::Db;
use sqlx::{MySql, QueryBuilder, Row};

pub(crate) struct FollowedRecords;

#[async_trait::async_trait]
impl ActivityScope for FollowedRecords {
    async fn followed(
        &self,
        pool: &Db,
        user: &UserId,
        records: &[RecordRef],
    ) -> Result<HashSet<RecordRef>, AppError> {
        if records.is_empty() {
            return Ok(HashSet::new());
        }
        let mut builder = QueryBuilder::<MySql>::new(
            "SELECT record_type, record_id FROM record_followers WHERE user_id = ",
        );
        builder
            .push_bind(user.as_str().to_owned())
            .push(" AND (record_type, record_id) IN (");
        let mut seen = HashSet::new();
        let mut separated = builder.separated(", ");
        for record in records.iter().filter(|record| seen.insert(*record)) {
            separated.push("(");
            separated.push_bind_unseparated(record.record_type.as_str().to_owned());
            separated.push_unseparated(", ");
            separated.push_bind_unseparated(record.record_id.to_string());
            separated.push_unseparated(")");
        }
        builder.push(")");
        let rows = builder.build().fetch_all(pool).await?;
        rows.iter()
            .map(|row| {
                let record_type: String = row.try_get("record_type")?;
                let record_id: String = row.try_get("record_id")?;
                RecordRef::parse(&record_type, &record_id)
            })
            .collect()
    }
}
