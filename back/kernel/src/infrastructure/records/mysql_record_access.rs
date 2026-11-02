use std::sync::Arc;

use async_trait::async_trait;
use sqlx::{MySql, MySqlPool, QueryBuilder};

use crate::application::access::Authorizer;
use crate::application::fields::{EntityDefinition, RecordTypeRegistry};
use crate::application::files::{RecordAccessChecker, UNSUPPORTED_RECORD_TYPE};
use crate::application::organization_admin::ListWindow;
use crate::application::query::{
    ArchivedFilter, CompareOp, FieldTarget, PlanCondition, PlanNode, PlanValue, Predicate,
    QueryPlan,
};
use crate::application::shared::RecordRef;
use crate::domain::access::{AccessScope, Permission};
use crate::domain::fields::FieldType;
use crate::errors::AppError;
use crate::infrastructure::query::compiler::{push_where, quoted};
use crate::records::constrain_query;

#[derive(Clone)]
pub struct MySqlRecordAccessChecker {
    pool: MySqlPool,
    registry: Arc<RecordTypeRegistry>,
}

impl MySqlRecordAccessChecker {
    pub fn new(pool: MySqlPool, registry: Arc<RecordTypeRegistry>) -> Self {
        Self { pool, registry }
    }

    fn definition(&self, record: &RecordRef) -> Result<&EntityDefinition, AppError> {
        self.registry.get(&record.record_type).ok_or_else(|| {
            AppError::validation_code(
                UNSUPPORTED_RECORD_TYPE,
                format!("records of type {} are not supported", record.record_type),
            )
        })
    }

    async fn visible_with(
        &self,
        scope: &AccessScope,
        definition: &EntityDefinition,
        permission: &Permission,
        record: &RecordRef,
    ) -> Result<bool, AppError> {
        let Ok(visibility) = scope.record_filter(permission) else {
            return Ok(false);
        };
        let plan = QueryPlan {
            record_type: definition.clone(),
            visibility,
            filter: Some(PlanNode::Condition(PlanCondition {
                target: FieldTarget::System {
                    column: "id".to_owned(),
                },
                field_type: FieldType::Record,
                predicate: Predicate::Compare(
                    CompareOp::Eq,
                    PlanValue::Text(record.record_id.to_string()),
                ),
                negated: false,
            })),
            search: None,
            sort: Vec::new(),
            archived: ArchivedFilter::Include,
            window: ListWindow {
                offset: 0,
                limit: 1,
            },
        };
        let constrained = constrain_query(&self.pool, scope, &plan).await?;
        let plan = constrained.as_ref().unwrap_or(&plan);
        let mut builder = QueryBuilder::<MySql>::new(format!(
            "SELECT EXISTS(SELECT 1 FROM {} r",
            quoted(&definition.table)
        ));
        push_where(&mut builder, scope, plan)?;
        builder.push(")");
        let visible: bool = builder.build_query_scalar().fetch_one(&self.pool).await?;
        Ok(visible)
    }
}

#[async_trait]
impl RecordAccessChecker for MySqlRecordAccessChecker {
    async fn can_read(&self, scope: &AccessScope, record: &RecordRef) -> Result<bool, AppError> {
        let definition = self.definition(record)?;
        self.visible_with(scope, definition, &definition.read_permission, record)
            .await
    }

    async fn can_update(&self, scope: &AccessScope, record: &RecordRef) -> Result<bool, AppError> {
        let definition = self.definition(record)?;
        let Some(permission) = &definition.update_permission else {
            return Ok(false);
        };
        self.visible_with(scope, definition, permission, record)
            .await
    }

    async fn can_with(
        &self,
        scope: &AccessScope,
        record: &RecordRef,
        permission: &Permission,
    ) -> Result<bool, AppError> {
        let definition = self.definition(record)?;
        self.visible_with(scope, definition, permission, record)
            .await
    }
}
