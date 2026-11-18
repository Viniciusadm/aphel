use crate::store::{activity_from_row, activity_select};
use crate::timeline::activity_entry;
use aphel_contract_crm::CrmRecordTypes;
use aphel_kernel::{
    application::{fields::RecordTypeRegistry, shared::RecordType},
    domain::access::{AccessScope, RecordScopeFilter},
    errors::AppError,
    infrastructure::query::compiler::push_record_scope,
};
use aphel_sdk::platform::database::Db;
use aphel_sdk::timeline::{TimelineEntry, TimelineRequest, push_cursor};
use chrono::NaiveDateTime;
use sqlx::{MySql, QueryBuilder, Row};
use std::sync::Arc;

pub(crate) struct SourceStore {
    pub(crate) pool: Db,
    pub(crate) registry: Arc<RecordTypeRegistry>,
}
impl SourceStore {
    pub(crate) async fn load(
        &self,
        scope: &AccessScope,
        request: &TimelineRequest,
        visibility: &RecordScopeFilter,
    ) -> Result<Vec<TimelineEntry>, AppError> {
        let subject = &request.subject;
        let mut builder = QueryBuilder::<MySql>::new(activity_select("r"));
        builder
            .push("WHERE ")
            .push("r.deleted_at IS NULL AND ((r.record_type = ")
            .push_bind(subject.record_type.as_str().to_owned())
            .push(" AND r.record_id = ")
            .push_bind(subject.record_id.to_string())
            .push(") OR r.id IN (SELECT l.activity_id FROM task_links l WHERE ")
            .push("l.record_type = ")
            .push_bind(subject.record_type.as_str().to_owned())
            .push(" AND l.record_id = ")
            .push_bind(subject.record_id.to_string())
            .push("))");
        push_record_scope(
            &mut builder,
            scope,
            self.registry.require(RecordType::ACTIVITY.as_str())?,
            visibility,
        );
        let plan = aphel_sdk::platform::application::query::QueryPlan {
            record_type: self
                .registry
                .require(RecordType::ACTIVITY.as_str())?
                .clone(),
            visibility: visibility.clone(),
            filter: None,
            search: None,
            sort: Vec::new(),
            archived: aphel_sdk::platform::application::query::ArchivedFilter::Include,
            window: aphel_sdk::platform::application::organization_admin::ListWindow {
                offset: 0,
                limit: 1,
            },
        };
        if let Some(constrained) =
            aphel_sdk::platform::records::constrain_query(&self.pool, scope, &plan).await?
        {
            builder.push(" AND r.id IN (SELECT r.id FROM tasks r");
            aphel_sdk::platform::infrastructure::query::compiler::push_where(
                &mut builder,
                scope,
                &constrained,
            )?;
            builder.push(")");
        }
        push_cursor(
            &mut builder,
            "COALESCE(r.completed_at, r.created_at)",
            "r.id",
            "activity:",
            request.cursor.as_ref(),
        );
        builder
            .push(" ORDER BY occurred_at DESC, r.id DESC LIMIT ")
            .push_bind(request.limit);
        let rows = builder.build().fetch_all(&self.pool).await?;
        rows.iter()
            .map(|row| {
                let occurred_at: NaiveDateTime = row.try_get("occurred_at")?;
                Ok(activity_entry(
                    activity_from_row(row)?,
                    occurred_at,
                    subject,
                ))
            })
            .collect()
    }
}
