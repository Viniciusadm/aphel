use aphel_sdk::platform::application::fields::{CustomFieldPorts, FieldSet};
use aphel_sdk::{AppError, Ctx, party::PartyAssociationSource, shared::RecordType};
use uuid::Uuid;
pub(crate) struct Associations;
#[async_trait::async_trait]
impl PartyAssociationSource for Associations {
    fn target_type(&self) -> &'static str {
        aphel_sdk::organizations::RECORD_TYPE
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
        let definition = ctx.registry().require(party_type.as_str())?;
        if FieldSet::for_scope(ctx.fields().definitions(), definition, ctx.scope)
            .await?
            .is_hidden("company_id")
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
        crate::store::organizations_of(ctx.pool(), &ids)
            .await?
            .into_iter()
            .map(|id| {
                Uuid::parse_str(&id).map_err(|_| {
                    AppError::Internal("invalid organization association identity".into())
                })
            })
            .collect()
    }
}
