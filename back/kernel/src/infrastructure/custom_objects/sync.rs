use sqlx::MySqlPool;

use crate::application::custom_objects::custom_record_definition;
use crate::application::fields::RecordTypeRegistry;
use crate::errors::AppError;
use crate::infrastructure::custom_objects::load_schema;
use crate::infrastructure::fields::mysql_definitions::seed_system_fields;

pub async fn sync_custom_object_fields(pool: &MySqlPool) -> Result<u64, AppError> {
    let schema = load_schema(pool).await?;
    let mut registry = RecordTypeRegistry::empty();
    for object in &schema.objects {
        registry = registry.register(custom_record_definition(object)?)?;
    }
    let mut transaction = pool.begin().await?;
    seed_system_fields(&mut transaction, &registry).await?;
    transaction.commit().await?;
    Ok(schema.objects.len() as u64)
}
