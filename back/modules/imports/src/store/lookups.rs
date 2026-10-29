use std::sync::Arc;

use sqlx::{MySql, MySqlPool, QueryBuilder};
use uuid::Uuid;

use crate::DuplicateKey;
use aphel_kernel::application::access::Authorizer;
use aphel_kernel::application::fields::{
    LabelNormalization, RecordTypeDefinition, RecordTypeRegistry,
};
use aphel_kernel::context::Ctx;
use aphel_kernel::domain::UserId;
use aphel_kernel::domain::access::AccessScope;
use aphel_kernel::domain::custom_objects::is_custom_record_type;
use aphel_kernel::errors::AppError;
use aphel_kernel::infrastructure::query::compiler::push_record_scope;
use aphel_kernel::records::normalize_domain;
use exports::config_label_source;

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

    fn scoped(
        &self,
        builder: &mut QueryBuilder<MySql>,
        scope: &AccessScope,
        record_type: &str,
    ) -> Result<(), AppError> {
        let Ok(definition) = self.registry.require(record_type) else {
            return Ok(());
        };
        let visibility = scope.record_filter(&definition.read_permission)?;
        push_record_scope(builder, scope, definition, &visibility);
        Ok(())
    }

    async fn first_id(&self, mut builder: QueryBuilder<MySql>) -> Result<Option<Uuid>, AppError> {
        let found: Option<String> = builder
            .build_query_scalar()
            .fetch_optional(&self.pool)
            .await?;
        Ok(found.and_then(|id| Uuid::parse_str(&id).ok()))
    }
}

impl RecordLookups {
    fn record_label_query(
        &self,
        scope: &AccessScope,
        definition: &RecordTypeDefinition,
        label: &str,
    ) -> Result<Option<QueryBuilder<MySql>>, AppError> {
        let Some(title) = definition.title_column() else {
            return Ok(None);
        };
        let mut builder = QueryBuilder::new(format!(
            "SELECT r.id FROM {} r WHERE TRUE",
            definition.table
        ));
        if let Some(deleted) = &definition.deleted_column {
            builder.push(format!(" AND r.{deleted} IS NULL"));
        }
        builder
            .push(format!(" AND (r.{title} = "))
            .push_bind(label.to_owned());
        for alias in &definition.capabilities.label_aliases {
            let value = match alias.normalization {
                LabelNormalization::Exact => Some(label.to_owned()),
                LabelNormalization::Lowercase => Some(label.to_lowercase()),
                LabelNormalization::Domain => {
                    normalize_domain(label).filter(|_| !label.contains(' '))
                }
            };
            if let Some(value) = value {
                builder
                    .push(format!(" OR r.{} = ", alias.column))
                    .push_bind(value);
            }
        }
        builder.push(")");
        self.scoped(&mut builder, scope, definition.record_type.as_str())?;
        builder
            .push(format!(" ORDER BY r.{title} = "))
            .push_bind(label.to_owned())
            .push(" DESC, r.created_at, r.id");
        Ok(Some(builder))
    }
}

impl RecordLookups {
    pub async fn find_member_by_email(&self, email: &str) -> Result<Option<UserId>, AppError> {
        let found: Option<String> = sqlx::query_scalar(
            "SELECT u.id FROM users u \
             JOIN users m ON m.id = u.id \
             WHERE m.status = 'active' AND u.status = 'active' \
             AND u.email = ? LIMIT 1",
        )
        .bind(email.trim().to_lowercase())
        .fetch_optional(&self.pool)
        .await?;
        Ok(found.and_then(|id| UserId::new(id).ok()))
    }

    pub async fn find_record_by_label(
        &self,
        scope: &AccessScope,
        record_type: &str,
        label: &str,
    ) -> Result<Option<Uuid>, AppError> {
        let label = label.trim();
        if let Ok(definition) = self.registry.require(record_type) {
            return match self.record_label_query(scope, definition, label)? {
                Some(mut builder) => {
                    builder.push(" LIMIT 1");
                    self.first_id(builder).await
                }
                None => Ok(None),
            };
        }
        if is_custom_record_type(record_type) {
            return Ok(None);
        }
        let mut builder = match record_type {
            "pipeline_stage" => {
                let mut builder = QueryBuilder::new(
                    "SELECT s.id FROM pipeline_stages s JOIN pipelines p ON p.id = s.pipeline_id \
                     WHERE s.is_active = TRUE AND p.is_active = TRUE",
                );
                builder.push(" AND s.name = ");
                builder.push_bind(label.to_owned());
                builder.push(" ORDER BY p.is_default DESC, p.position, s.position");
                builder
            }
            "pipeline" | "lost_reason" | "activity_type" => {
                let Some(source) = config_label_source(record_type) else {
                    return Ok(None);
                };
                let mut builder = QueryBuilder::new(format!(
                    "SELECT id FROM {} WHERE is_active = TRUE",
                    source.table
                ));
                builder.push(" AND name = ");
                builder.push_bind(label.to_owned());
                builder.push(" ORDER BY position, id");
                builder
            }
            _ => return Ok(None),
        };
        builder.push(" LIMIT 1");
        self.first_id(builder).await
    }

    pub(crate) async fn find_duplicate(
        &self,
        scope: &AccessScope,
        definition: &RecordTypeDefinition,
        key: DuplicateKey,
        value: &str,
    ) -> Result<Option<Uuid>, AppError> {
        let columns = key.columns(definition);
        if columns.is_empty() {
            return Ok(None);
        }
        let mut builder = QueryBuilder::new(format!(
            "SELECT r.id FROM {} r WHERE TRUE",
            definition.table
        ));
        if let Some(deleted) = &definition.deleted_column {
            builder.push(format!(" AND r.{deleted} IS NULL"));
        }
        builder.push(" AND (");
        for (index, column) in columns.iter().enumerate() {
            if index > 0 {
                builder.push(" OR ");
            }
            builder
                .push(format!("r.{} = ", column.column))
                .push_bind(value.to_owned());
        }
        builder.push(")");
        let visibility = scope.record_filter(&definition.read_permission)?;
        push_record_scope(&mut builder, scope, definition, &visibility);
        builder.push(" ORDER BY r.created_at, r.id LIMIT 1");
        self.first_id(builder).await
    }
}
