use uuid::Uuid;

use crate::application::access::Authorizer;
use crate::application::fields::FieldSet;
use crate::application::organization_admin::Listing;
use crate::application::query::{
    Filter, RecordQuery, RecordQueryExecutor, RecordQueryPorts, RelatedFieldSets, ResolveContext,
    SegmentFilters, SegmentResolution, load_related_sets, resolve_query_related, segment_ids,
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
        let segments = self.segments(record_type, query.filter.as_ref()).await?;
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
            SegmentResolution::Expand(&segments),
            &relations,
        )?;
        self.ports.executor().execute(scope, &plan).await
    }

    pub async fn segments(
        &self,
        record_type: &str,
        filter: Option<&Filter>,
    ) -> Result<SegmentFilters, AppError> {
        let mut segments = SegmentFilters::new();
        let Some(filter) = filter else {
            return Ok(segments);
        };
        for id in segment_ids(filter) {
            if let Some(source) = self.ports.executor().segment(id).await?
                && source.record_type == record_type
            {
                segments.insert(id, source.filter);
            }
        }
        Ok(segments)
    }
}
