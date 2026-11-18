use async_trait::async_trait;
use sqlx::MySqlPool;

use crate::application::custom_objects::{CustomSchema, CustomSchemaSource};
use crate::errors::AppError;
use crate::infrastructure::custom_objects::rows::{
    OBJECT_COLUMNS, RELATIONSHIP_COLUMNS, object_from_row, relationship_from_row,
};

#[derive(Clone)]
pub struct MySqlCustomSchemaSource {
    pool: MySqlPool,
}

impl MySqlCustomSchemaSource {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

pub(crate) async fn load_schema(pool: &MySqlPool) -> Result<CustomSchema, AppError> {
    let objects = sqlx::query(sqlx::AssertSqlSafe(format!(
        "SELECT {OBJECT_COLUMNS} FROM custom_objects o \
         ORDER BY o.position, o.label_plural, o.id"
    )))
    .fetch_all(pool)
    .await?
    .iter()
    .map(object_from_row)
    .collect::<Result<Vec<_>, _>>()?;
    let relationships = sqlx::query(sqlx::AssertSqlSafe(format!(
        "SELECT {RELATIONSHIP_COLUMNS} FROM relationship_definitions d \
         ORDER BY d.created_at, d.id"
    )))
    .fetch_all(pool)
    .await?
    .iter()
    .map(relationship_from_row)
    .collect::<Result<Vec<_>, _>>()?;
    Ok(CustomSchema {
        objects,
        relationships,
    })
}

#[async_trait]
impl CustomSchemaSource for MySqlCustomSchemaSource {
    async fn version(&self) -> Result<u64, AppError> {
        let version: Option<u64> = sqlx::query_scalar(
            "SELECT custom_schema_version FROM organization_settings WHERE id = 1",
        )
        .fetch_optional(&self.pool)
        .await?;
        Ok(version.unwrap_or_default())
    }

    async fn load(&self) -> Result<CustomSchema, AppError> {
        load_schema(&self.pool).await
    }
}

pub(crate) async fn load_objects_in(
    tx: &mut crate::database::transaction::Tx,
) -> Result<Vec<crate::domain::custom_objects::CustomObject>, AppError> {
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "SELECT {OBJECT_COLUMNS} FROM custom_objects o ORDER BY o.position, o.id"
    )))
    .fetch_all(&mut **tx)
    .await?
    .iter()
    .map(object_from_row)
    .collect()
}
