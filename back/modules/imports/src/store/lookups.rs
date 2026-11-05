use crate::DuplicateKey;
use aphel_kernel::Ctx;
use aphel_kernel::domain::{UserId, access::AccessScope};
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::application::fields::{
    EntityDefinition, LabelNormalization, RecordTypeRegistry,
};
use aphel_sdk::platform::application::organization_admin::ListWindow;
use aphel_sdk::platform::application::query::{
    ArchivedFilter, Filter, FilterOperator, RecordQuery, RecordQueryEngine, SortSpec,
};
use aphel_sdk::platform::infrastructure::query::MySqlRecordQueryPorts;
use aphel_sdk::platform::records::normalize_domain;
use sqlx::MySqlPool;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub struct RecordLookups {
    pool: MySqlPool,
    registry: Arc<RecordTypeRegistry>,
}
impl RecordLookups {
    pub(crate) fn new(pool: MySqlPool, registry: Arc<RecordTypeRegistry>) -> Self {
        Self { pool, registry }
    }
    pub fn of(ctx: &Ctx<'_>) -> Self {
        Self::new(ctx.pool().clone(), ctx.registry().clone())
    }
    async fn first(
        &self,
        scope: &AccessScope,
        definition: &EntityDefinition,
        filter: Filter,
    ) -> Result<Option<Uuid>, AppError> {
        let ports = MySqlRecordQueryPorts::new(self.pool.clone(), self.registry.clone());
        Ok(RecordQueryEngine::new(&ports)
            .query(
                scope,
                definition.record_type.as_str(),
                RecordQuery {
                    filter: Some(filter),
                    sort: vec![SortSpec::asc("created_at"), SortSpec::asc("id")],
                    search: None,
                    archived: ArchivedFilter::Include,
                    window: ListWindow {
                        offset: 0,
                        limit: 1,
                    },
                },
            )
            .await?
            .items
            .into_iter()
            .next())
    }
    pub async fn find_member_by_email(&self, email: &str) -> Result<Option<UserId>, AppError> {
        let id: Option<String> = sqlx::query_scalar(
            "SELECT id FROM users WHERE status = 'active' AND email = ? LIMIT 1",
        )
        .bind(email.trim().to_lowercase())
        .fetch_optional(&self.pool)
        .await?;
        id.map(|id| {
            UserId::new(id).map_err(|_| AppError::Internal("invalid stored member identity".into()))
        })
        .transpose()
    }
    pub async fn find_record_by_label(
        &self,
        scope: &AccessScope,
        record_type: &str,
        label: &str,
    ) -> Result<Option<Uuid>, AppError> {
        let Ok(definition) = self.registry.require(record_type) else {
            return Ok(None);
        };
        let Some(title) = definition.title_field.as_deref() else {
            return Ok(None);
        };
        let base = || {
            definition
                .system_field_by_key("is_active")
                .map(|_| {
                    Filter::condition("is_active", FilterOperator::Eq, serde_json::json!(true))
                })
                .into_iter()
                .collect::<Vec<_>>()
        };
        let mut conditions = base();
        conditions.push(Filter::condition(
            title,
            FilterOperator::Eq,
            serde_json::json!(label.trim()),
        ));
        if let Some(id) = self
            .first(scope, definition, Filter::and(conditions))
            .await?
        {
            return Ok(Some(id));
        }
        let aliases: Vec<_> = definition
            .capabilities
            .label_aliases
            .iter()
            .filter_map(|alias| {
                let field = definition
                    .system_fields
                    .iter()
                    .find(|field| field.column == alias.column)?;
                let value = match alias.normalization {
                    LabelNormalization::Exact => Some(label.trim().to_owned()),
                    LabelNormalization::Lowercase => Some(label.trim().to_lowercase()),
                    LabelNormalization::Domain => {
                        normalize_domain(label).filter(|_| !label.contains(' '))
                    }
                }?;
                Some(Filter::condition(
                    field.key.as_str(),
                    FilterOperator::Eq,
                    serde_json::json!(value),
                ))
            })
            .collect();
        if aliases.is_empty() {
            return Ok(None);
        }
        let mut conditions = base();
        conditions.push(Filter::or(aliases));
        self.first(scope, definition, Filter::and(conditions)).await
    }
    pub(crate) async fn find_duplicate(
        &self,
        scope: &AccessScope,
        definition: &EntityDefinition,
        key: DuplicateKey,
        value: &str,
    ) -> Result<Option<Uuid>, AppError> {
        let filters: Vec<_> = key
            .columns(definition)
            .into_iter()
            .map(|column| {
                Filter::condition(column.field, FilterOperator::Eq, serde_json::json!(value))
            })
            .collect();
        if filters.is_empty() {
            return Ok(None);
        }
        self.first(scope, definition, Filter::or(filters)).await
    }
}
