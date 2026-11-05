use aphel_contract_crm::CrmPermissions;
use aphel_kernel::errors::AppError;
use aphel_sdk::dashboard::DashboardEventScope;
use aphel_sdk::platform::application::query::QueryPlan;
use aphel_sdk::platform::domain::access::{AccessScope, Permission};
use aphel_sdk::platform::infrastructure::query::compiler::push_reference_subquery;
use aphel_sdk::shared::{RecordRef, RecordType};
use async_trait::async_trait;
use sqlx::{MySql, MySqlPool, QueryBuilder, Row};
use std::collections::HashMap;
use uuid::Uuid;

pub(crate) struct NoteDashboardEvents;

#[async_trait]
impl DashboardEventScope for NoteDashboardEvents {
    fn record_type(&self) -> RecordType {
        RecordType::from_static("notes.note")
    }
    async fn restrict(
        &self,
        _pool: &MySqlPool,
        scope: &AccessScope,
        records: &[QueryPlan],
        builder: &mut QueryBuilder<MySql>,
    ) -> Result<(), AppError> {
        if scope.filter_for(&Permission::NOTES_READ).is_none() {
            return Ok(());
        }
        builder.push(" OR (e.record_type = ").push_bind(self.record_type().as_str().to_owned())
            .push(" AND e.event_type = 'note.created' AND e.record_id IN (SELECT n.id FROM notes n WHERE n.deleted_at IS NULL AND (n.visibility = 'team' OR n.author_id = ")
            .push_bind(scope.user_id().as_str().to_owned()).push(") AND (FALSE");
        for plan in records {
            push_reference_subquery(builder, scope, plan, "n", "record_type", "record_id")?;
        }
        builder.push(")))");
        Ok(())
    }
    async fn targets(
        &self,
        pool: &MySqlPool,
        ids: &[Uuid],
    ) -> Result<HashMap<Uuid, RecordRef>, AppError> {
        let mut targets = HashMap::new();
        if ids.is_empty() {
            return Ok(targets);
        }
        let mut builder = QueryBuilder::<MySql>::new(
            "SELECT id, record_type, record_id FROM notes WHERE id IN (",
        );
        let mut values = builder.separated(", ");
        for id in ids {
            values.push_bind(id.to_string());
        }
        builder.push(")");
        for row in builder.build().fetch_all(pool).await? {
            let id: String = row.try_get("id")?;
            let record_type: String = row.try_get("record_type")?;
            let record_id: String = row.try_get("record_id")?;
            targets.insert(
                Uuid::parse_str(&id)
                    .map_err(|_| AppError::Internal("invalid stored note identity".into()))?,
                RecordRef::parse(&record_type, &record_id)?,
            );
        }
        Ok(targets)
    }
}
