use uuid::Uuid;

use crate::application::access::Authorizer;
use crate::application::fields::FieldSet;
use crate::application::organization_admin::Listing;
use crate::application::query::{
    Filter, FilterExpansion, FilterExpansions, RecordQuery, RecordQueryExecutor, RecordQueryPorts,
    RelatedFieldSets, ResolveContext, filter_expansion_ids, load_related_sets,
    resolve_query_related,
};
use crate::application::security::Clock;
use crate::domain::access::AccessScope;
use crate::errors::AppError;

pub struct RecordQueryEngine<'a, P> {
    ports: &'a P,
}

impl<'a, P> RecordQueryEngine<'a, P>
where
    P: RecordQueryPorts,
{
    pub fn new(ports: &'a P) -> Self {
        Self { ports }
    }

    pub async fn query(
        &self,
        scope: &AccessScope,
        record_type: &str,
        query: RecordQuery,
    ) -> Result<Listing<Uuid>, AppError> {
        let definition = self.ports.registry().require(record_type)?;
        let visibility = scope.record_filter(&definition.read_permission)?;
        let fields = FieldSet::for_scope(self.ports.definitions(), definition, scope).await?;
        let context = ResolveContext {
            now: self.ports.clock().now(),
            timezone: self.ports.executor().timezone().await?,
            user_id: scope.user_id().clone(),
        };
        let expansions = self.expansions(record_type, query.filter.as_ref()).await?;
        let relations = match &query.filter {
            Some(filter) => {
                load_related_sets(
                    self.ports.registry(),
                    self.ports.definitions(),
                    scope,
                    definition,
                    &filter.relation_keys(),
                )
                .await?
            }
            None => RelatedFieldSets::default(),
        };
        let plan = resolve_query_related(
            &fields,
            &context,
            visibility,
            query,
            FilterExpansion::Expand(&expansions),
            &relations,
        )?;
        self.ports.executor().execute(scope, &plan).await
    }

    pub async fn expansions(
        &self,
        record_type: &str,
        filter: Option<&Filter>,
    ) -> Result<FilterExpansions, AppError> {
        let mut expansions = FilterExpansions::new();
        let Some(filter) = filter else {
            return Ok(expansions);
        };
        for id in filter_expansion_ids(filter) {
            if let Some(source) = self.ports.executor().filter_source(id).await?
                && source.record_type == record_type
            {
                expansions.insert(id, source.filter);
            }
        }
        Ok(expansions)
    }
}
