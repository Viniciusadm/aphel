use aphel_kernel::{
    Ctx,
    application::{
        organization_admin::ListWindow,
        query::{
            ArchivedFilter, CompareOp, FieldTarget, PlanCondition, PlanNode, PlanValue, Predicate,
            QueryPlan,
        },
        shared::RecordRef,
    },
    domain::fields::FieldType,
    errors::AppError,
    infrastructure::query::compiler::push_reference_subquery,
    records::constrain_query,
};
use aphel_sdk::platform::application::access::Authorizer;
use aphel_sdk::timeline::TimelineEventScope;
use sqlx::{MySql, QueryBuilder};
pub(crate) struct RelatedEvents;
#[async_trait::async_trait]
impl TimelineEventScope for RelatedEvents {
    async fn extend(
        &self,
        ctx: &Ctx<'_>,
        builder: &mut QueryBuilder<MySql>,
        subject: &RecordRef,
    ) -> Result<(), AppError> {
        if subject.record_type.as_str() != "organizations.organization" {
            return Ok(());
        }
        let Some(definition) = ctx
            .registry()
            .definitions()
            .find(|definition| definition.record_type.as_str() == "deals.deal")
        else {
            return Ok(());
        };
        let Ok(visibility) = ctx.scope.record_filter(&definition.read_permission) else {
            return Ok(());
        };
        let plan = QueryPlan {
            record_type: definition.clone(),
            visibility,
            filter: Some(PlanNode::Condition(PlanCondition {
                target: FieldTarget::System {
                    column: "company_id".to_owned(),
                },
                field_type: FieldType::Record,
                predicate: Predicate::Compare(
                    CompareOp::Eq,
                    PlanValue::Text(subject.record_id.to_string()),
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
        let constrained = constrain_query(ctx.pool(), ctx.scope, &plan).await?;
        push_reference_subquery(
            builder,
            ctx.scope,
            constrained.as_ref().unwrap_or(&plan),
            "e",
            "record_type",
            "record_id",
        )
    }
}
