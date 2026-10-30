use async_trait::async_trait;
use sqlx::{MySql, MySqlPool, QueryBuilder};
use uuid::Uuid;

use crate::application::fields::{RecordTypeDefinition, ReferenceChecker};
use crate::database::transaction::MySqlTx;
use crate::domain::UserId;
use crate::domain::access::{AccessScope, RecordScopeFilter};
use crate::errors::AppError;
use crate::infrastructure::query::compiler::{push_record_scope, quoted};

#[derive(Clone)]
pub struct MySqlReferenceChecker {
    pool: MySqlPool,
}

impl MySqlReferenceChecker {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ReferenceChecker for MySqlReferenceChecker {
    async fn existing_users(&self, user_ids: &[UserId]) -> Result<Vec<UserId>, AppError> {
        if user_ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut builder = QueryBuilder::new("SELECT id FROM users WHERE id IN (");
        let mut ids = builder.separated(", ");
        for id in user_ids {
            ids.push_bind(id.as_str().to_owned());
        }
        builder.push(")");
        let found: Vec<String> = builder.build_query_scalar().fetch_all(&self.pool).await?;
        Ok(found
            .into_iter()
            .filter_map(|id| UserId::new(id).ok())
            .collect())
    }

    async fn existing_records(
        &self,
        scope: &AccessScope,
        target: &RecordTypeDefinition,
        visibility: &RecordScopeFilter,
        record_ids: &[Uuid],
    ) -> Result<Vec<Uuid>, AppError> {
        if record_ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut builder = existing_records_query(scope, target, visibility, record_ids);
        let found: Vec<String> = builder.build_query_scalar().fetch_all(&self.pool).await?;
        Ok(parsed(found))
    }
}

fn existing_records_query(
    scope: &AccessScope,
    target: &RecordTypeDefinition,
    visibility: &RecordScopeFilter,
    record_ids: &[Uuid],
) -> QueryBuilder<MySql> {
    let mut builder = QueryBuilder::new(format!(
        "SELECT r.id FROM {} r WHERE r.id IN (",
        quoted(&target.table)
    ));
    let mut ids = builder.separated(", ");
    for id in record_ids {
        ids.push_bind(id.to_string());
    }
    builder.push(")");
    if let Some(column) = &target.deleted_column {
        builder.push(format!(" AND r.{} IS NULL", quoted(column)));
    }
    push_record_scope(&mut builder, scope, target, visibility);
    builder
}

fn parsed(found: Vec<String>) -> Vec<Uuid> {
    found
        .into_iter()
        .filter_map(|id| Uuid::parse_str(&id).ok())
        .collect()
}

pub(crate) async fn existing_records_in(
    tx: &mut MySqlTx,
    scope: &AccessScope,
    target: &RecordTypeDefinition,
    visibility: &RecordScopeFilter,
    record_ids: &[Uuid],
) -> Result<Vec<Uuid>, AppError> {
    if record_ids.is_empty() {
        return Ok(Vec::new());
    }
    let mut builder = existing_records_query(scope, target, visibility, record_ids);
    let found: Vec<String> = builder.build_query_scalar().fetch_all(&mut **tx).await?;
    Ok(parsed(found))
}
