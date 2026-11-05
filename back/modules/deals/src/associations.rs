use aphel_sdk::platform::application::fields::{CustomFieldPorts, FieldSet};
use aphel_sdk::{AppError, Ctx, party::PartyAssociationSource, shared::RecordType};
use sqlx::QueryBuilder;
use uuid::Uuid;
pub(crate) struct Associations;
#[async_trait::async_trait]
impl PartyAssociationSource for Associations {
    fn target_type(&self) -> &'static str {
        aphel_sdk::deals::RECORD_TYPE
    }
    async fn linked_ids(
        &self,
        ctx: &Ctx<'_>,
        party_type: &RecordType,
        ids: &[Uuid],
    ) -> Result<Vec<Uuid>, AppError> {
        if party_type.as_str() != aphel_sdk::people::RECORD_TYPE {
            return Ok(Vec::new());
        }
        let definition = ctx.registry().require(aphel_sdk::deals::RECORD_TYPE)?;
        if ctx.scope.scope_for(&definition.read_permission).is_none()
            || FieldSet::for_scope(ctx.fields().definitions(), definition, ctx.scope)
                .await?
                .is_hidden("contact_id")
        {
            return Ok(Vec::new());
        }
        let ids = aphel_sdk::records::visible_record_ids(
            ctx.pool(),
            ctx.registry(),
            ctx.scope,
            party_type,
            ids,
        )
        .await?;
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut query = QueryBuilder::<sqlx::MySql>::new(
            "SELECT id FROM deals WHERE status = 'open' AND deleted_at IS NULL AND archived_at IS NULL AND (contact_id IN (",
        );
        let mut separated = query.separated(", ");
        for id in &ids {
            separated.push_bind(id.to_string());
        }
        query.push(") OR id IN (SELECT deal_id FROM deal_contacts WHERE contact_id IN (");
        let mut separated = query.separated(", ");
        for id in &ids {
            separated.push_bind(id.to_string());
        }
        query.push(")))");
        let candidates = query
            .build_query_scalar::<String>()
            .fetch_all(ctx.pool())
            .await?
            .into_iter()
            .map(|id| {
                Uuid::parse_str(&id)
                    .map_err(|_| AppError::Internal("invalid deal association identity".into()))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let mut visible = Vec::new();
        let target = RecordType::parse(aphel_sdk::deals::RECORD_TYPE)?;
        for batch in candidates.chunks(1000) {
            visible.extend(
                aphel_sdk::records::visible_record_ids(
                    ctx.pool(),
                    ctx.registry(),
                    ctx.scope,
                    &target,
                    batch,
                )
                .await?,
            );
        }
        Ok(visible)
    }
}
