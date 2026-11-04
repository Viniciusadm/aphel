use std::collections::HashMap;
use std::sync::Arc;

use serde_json::{Value, json};
use sqlx::{MySql, MySqlPool, QueryBuilder, Row};
use uuid::Uuid;

use aphel_kernel::Ctx;
use aphel_kernel::application::access::Authorizer;
use aphel_kernel::application::fields::{EntityDefinition, FieldSet, RecordTypeRegistry};
use aphel_kernel::application::organization_admin::ListWindow;
use aphel_kernel::application::query::{
    ArchivedFilter, Filter, FilterExpansion, FilterOperator, ID_FIELD, QueryPlan, RecordQuery,
    RecordQueryEngine, RecordQueryExecutor, RecordQueryPorts, ResolveContext, load_related_sets,
    resolve_query_related, validate_view_query_related,
};
use aphel_kernel::application::security::Clock;
use aphel_kernel::domain::access::{AccessScope, Permission, RecordScopeFilter};
use aphel_kernel::errors::{AppError, FieldViolation};
use aphel_kernel::infrastructure::query::compiler::{
    Projection, build_query, push_record_scope, quoted,
};
use aphel_kernel::infrastructure::query::mysql_executor::MySqlRecordQueryPorts;
use aphel_kernel::infrastructure::repositories::mapper::parse_uuid;

const PAGE_SIZE: u64 = 1000;

#[derive(Clone)]
pub(crate) struct ScoringDirectory {
    pool: MySqlPool,
    registry: Arc<RecordTypeRegistry>,
}

impl ScoringDirectory {
    pub(crate) fn new(pool: MySqlPool, registry: Arc<RecordTypeRegistry>) -> Self {
        Self { pool, registry }
    }

    pub(crate) fn of(ctx: &Ctx<'_>) -> Self {
        Self::new(ctx.pool().clone(), ctx.registry().clone())
    }

    fn ports(&self) -> MySqlRecordQueryPorts {
        MySqlRecordQueryPorts::new(self.pool.clone(), self.registry.clone())
    }

    pub(crate) async fn plan(
        &self,
        elevated: &AccessScope,
        definition: &EntityDefinition,
        filter: Filter,
    ) -> Result<Result<QueryPlan, AppError>, AppError> {
        let ports = self.ports();
        let fields = FieldSet::for_scope(ports.definitions(), definition, elevated).await?;
        let context = ResolveContext {
            now: ports.clock().now(),
            timezone: ports.executor().timezone().await?,
            user_id: elevated.user_id().clone(),
        };
        let engine = RecordQueryEngine::new(&ports);
        let segments = engine
            .expansions(definition.record_type.as_str(), Some(&filter))
            .await?;
        let relations = load_related_sets(
            &self.registry,
            ports.definitions(),
            elevated,
            definition,
            &filter.relation_keys(),
        )
        .await?;
        let query = RecordQuery {
            filter: Some(filter),
            archived: ArchivedFilter::Include,
            window: ListWindow {
                offset: 0,
                limit: PAGE_SIZE,
            },
            ..RecordQuery::default()
        };
        Ok(resolve_query_related(
            &fields,
            &context,
            RecordScopeFilter::Organization,
            query,
            FilterExpansion::Expand(&segments),
            &relations,
        ))
    }

    pub(crate) async fn query_ids(
        &self,
        scope: &AccessScope,
        definition: &EntityDefinition,
        filter: Filter,
    ) -> Result<Vec<Uuid>, AppError> {
        let elevated = AccessScope::system().with_enabled_modules(scope.enabled_modules().to_vec());
        let mut plan = match self.plan(&elevated, definition, filter).await? {
            Ok(plan) => plan,
            Err(error) => {
                tracing::warn!(
                    record_type = definition.record_type.as_str(),
                    %error,
                    "scoring profile filter could not be resolved"
                );
                return Ok(Vec::new());
            }
        };
        let mut ids = Vec::new();
        loop {
            let mut builder = build_query(&elevated, &plan, Projection::Ids)?;
            let rows = builder.build().fetch_all(&self.pool).await?;
            let count = rows.len() as u64;
            for row in &rows {
                ids.push(parse_uuid(&row.try_get::<String, _>("id")?, "id")?);
            }
            if count < plan.window.limit {
                break;
            }
            plan.window.offset += plan.window.limit;
        }
        Ok(ids)
    }
}

fn id_filter(ids: &[Uuid]) -> Filter {
    Filter::condition(
        ID_FIELD,
        FilterOperator::In,
        Value::Array(ids.iter().map(|id| json!(id.to_string())).collect()),
    )
}

impl ScoringDirectory {
    pub(crate) async fn filter_violations(
        &self,
        scope: &AccessScope,
        definition: &EntityDefinition,
        filter: &Filter,
    ) -> Result<Vec<FieldViolation>, AppError> {
        let elevated = AccessScope::system().with_enabled_modules(scope.enabled_modules().to_vec());
        let ports = self.ports();
        let fields = FieldSet::for_scope(ports.definitions(), definition, &elevated).await?;
        let context = ResolveContext {
            now: ports.clock().now(),
            timezone: ports.executor().timezone().await?,
            user_id: scope.user_id().clone(),
        };
        let relations = load_related_sets(
            &self.registry,
            ports.definitions(),
            &elevated,
            definition,
            &filter.relation_keys(),
        )
        .await?;
        Ok(validate_view_query_related(
            &fields,
            &context,
            Some(filter),
            &[],
            &relations,
        ))
    }

    pub(crate) async fn matching_ids(
        &self,
        scope: &AccessScope,
        definition: &EntityDefinition,
        filter: &Filter,
        record_ids: Option<&[Uuid]>,
    ) -> Result<Vec<Uuid>, AppError> {
        let filter = match record_ids {
            Some([]) => return Ok(Vec::new()),
            Some(ids) => Filter::and(vec![filter.clone(), id_filter(ids)]),
            None => filter.clone(),
        };
        self.query_ids(scope, definition, filter).await
    }

    pub(crate) async fn existing_ids(
        &self,
        scope: &AccessScope,
        definition: &EntityDefinition,
        record_ids: &[Uuid],
    ) -> Result<Vec<Uuid>, AppError> {
        if record_ids.is_empty() {
            return Ok(Vec::new());
        }
        self.query_ids(scope, definition, id_filter(record_ids))
            .await
    }

    pub(crate) async fn companies_of_contacts(
        &self,
        contact_ids: &[Uuid],
    ) -> Result<HashMap<Uuid, Uuid>, AppError> {
        let mut organizations = HashMap::new();
        if let Some(source) = aphel_sdk::people::primary_organizations() {
            for batch in contact_ids.chunks(1000) {
                organizations.extend(
                    source
                        .by_people(&self.pool, &AccessScope::system(), batch, true)
                        .await?
                        .into_iter()
                        .map(|link| (link.person_id, link.organization_id)),
                );
            }
        }
        Ok(organizations)
    }

    pub(crate) async fn record_visible(
        &self,
        scope: &AccessScope,
        definition: &EntityDefinition,
        record_id: Uuid,
    ) -> Result<bool, AppError> {
        self.record_accessible(scope, definition, &definition.read_permission, record_id)
            .await
    }

    pub(crate) async fn record_accessible(
        &self,
        scope: &AccessScope,
        definition: &EntityDefinition,
        permission: &Permission,
        record_id: Uuid,
    ) -> Result<bool, AppError> {
        let Ok(visibility) = scope.record_filter(permission) else {
            return Ok(false);
        };
        let mut builder = QueryBuilder::<MySql>::new(format!(
            "SELECT r.id FROM {} r WHERE ",
            quoted(&definition.table)
        ));
        builder.push("r.id = ").push_bind(record_id.to_string());
        if let Some(deleted) = &definition.deleted_column {
            builder.push(format!(" AND r.{} IS NULL", quoted(deleted)));
        }
        push_record_scope(&mut builder, scope, definition, &visibility);
        Ok(builder.build().fetch_optional(&self.pool).await?.is_some())
    }
}
