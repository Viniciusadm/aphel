use crate::application::fields::FieldSet;
use crate::domain::access::AccessScope;
use crate::errors::AppError;
use crate::infrastructure::fields::MySqlFieldDefinitionRepository;
use crate::infrastructure::query::MySqlRecordQueryPorts;
use crate::state::AppState;

pub async fn query_ports(state: &AppState) -> Result<MySqlRecordQueryPorts, AppError> {
    Ok(MySqlRecordQueryPorts::new(
        state.db()?,
        state.registry().await?,
    ))
}

pub async fn field_set(
    state: &AppState,
    scope: &AccessScope,
    record_type: &str,
) -> Result<FieldSet, AppError> {
    let registry = state.registry().await?;
    let definition = registry.require(record_type)?;
    FieldSet::for_scope(
        &MySqlFieldDefinitionRepository::new(state.db()?),
        definition,
        scope,
    )
    .await
}
