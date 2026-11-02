use crate::api::state::AppState;
use crate::application::custom_objects::{CustomRecordUseCases, RelationUseCases};
use crate::errors::AppError;
use crate::infrastructure::custom_objects::{MySqlCustomRecordPorts, MySqlRelationPorts};
pub async fn custom_record_use_cases(
    state: &AppState,
) -> Result<CustomRecordUseCases<MySqlCustomRecordPorts>, AppError> {
    Ok(CustomRecordUseCases::new(MySqlCustomRecordPorts::new(
        state.db()?,
        state.registry().await?,
        state.events.clone(),
        state.audit.clone(),
    )))
}

pub async fn relation_use_cases(
    state: &AppState,
) -> Result<RelationUseCases<MySqlRelationPorts>, AppError> {
    Ok(RelationUseCases::new(MySqlRelationPorts::new(
        state.db()?,
        state.registry().await?,
        state.events.clone(),
        state.audit.clone(),
    )))
}
