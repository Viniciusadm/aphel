use aphel_kernel::domain::UserId;
use aphel_sdk::platform::application::access::Authorizer;
use aphel_sdk::platform::application::files::UserReference;
use aphel_sdk::platform::application::organization_admin::ListWindow;
use aphel_sdk::platform::application::query::{
    ArchivedFilter, Filter, FilterOperator, RecordQuery, RecordQueryEngine, SortSpec,
};
use aphel_sdk::{
    AppError, Ctx,
    tasks::{TaskReportPage, TaskReportQuery, TaskReportSubject, TaskReportSubjects},
};
use serde_json::json;
use sqlx::Row;
use uuid::Uuid;
pub(crate) struct Subjects;
#[async_trait::async_trait]
impl TaskReportSubjects for Subjects {
    async fn page(
        &self,
        ctx: &Ctx<'_>,
        query: TaskReportQuery,
    ) -> Result<TaskReportPage, AppError> {
        let mut filters = vec![Filter::or(vec![
            Filter::and(vec![
                Filter::condition("completed_at", FilterOperator::Gte, json!(query.from)),
                Filter::condition("completed_at", FilterOperator::Lt, json!(query.to)),
            ]),
            Filter::and(vec![
                Filter::condition(
                    "completed_at",
                    FilterOperator::IsEmpty,
                    serde_json::Value::Null,
                ),
                Filter::condition("created_at", FilterOperator::Gte, json!(query.from)),
                Filter::condition("created_at", FilterOperator::Lt, json!(query.to)),
            ]),
        ])];
        if let Some(owner) = query.owner_id {
            filters.push(Filter::condition(
                "owner_id",
                FilterOperator::Eq,
                json!(owner.as_str()),
            ));
        }
        if let Some(after) = query.after {
            filters.push(Filter::condition("id", FilterOperator::Gt, json!(after)));
        }
        let page = RecordQueryEngine::new(&ctx.query())
            .query(
                ctx.scope,
                "tasks.task",
                RecordQuery {
                    filter: Some(Filter::and(filters)),
                    sort: vec![SortSpec::asc("id")],
                    archived: ArchivedFilter::Include,
                    window: ListWindow {
                        offset: 0,
                        limit: 1000,
                    },
                    ..RecordQuery::default()
                },
            )
            .await?;
        let after = page.items.last().copied();
        let more = page.items.len() == 1000;
        if page.items.is_empty() {
            return Ok(TaskReportPage {
                rows: Vec::new(),
                after,
                more,
            });
        }
        let fields = ctx.field_set(ctx.scope, "tasks.task").await?;
        let owner_visible = !fields.is_hidden("owner_id");
        let mut statement = sqlx::QueryBuilder::<sqlx::MySql>::new(
            "SELECT a.id, a.owner_id, u.display_name AS owner_name FROM tasks a LEFT JOIN users u ON u.id = a.owner_id WHERE a.deleted_at IS NULL AND a.id IN (",
        );
        let mut ids = statement.separated(", ");
        for id in &page.items {
            ids.push_bind(id.to_string());
        }
        statement.push(")");
        let definition = ctx.registry().require("tasks.task")?;
        let visibility = ctx.scope.record_filter(&definition.read_permission)?;
        aphel_sdk::platform::infrastructure::query::compiler::push_record_scope_as(
            &mut statement,
            ctx.scope,
            definition,
            &visibility,
            "a",
        );
        let mut subjects = std::collections::HashMap::new();
        for row in statement.build().fetch_all(ctx.pool()).await? {
            let id: String = row.try_get("id")?;
            let id = Uuid::parse_str(&id)
                .map_err(|_| AppError::Internal("invalid stored task identity".into()))?;
            let owner: Option<String> = row.try_get("owner_id")?;
            let user = if owner_visible {
                owner
                    .map(|id| {
                        Ok::<_, AppError>(UserReference {
                            id: UserId::new(id)?,
                            display_name: row
                                .try_get::<Option<String>, _>("owner_name")?
                                .unwrap_or_default(),
                        })
                    })
                    .transpose()?
            } else {
                None
            };
            subjects.insert(id, TaskReportSubject { id, user });
        }
        Ok(TaskReportPage {
            rows: page
                .items
                .into_iter()
                .filter_map(|id| subjects.remove(&id))
                .collect(),
            after,
            more,
        })
    }
}
