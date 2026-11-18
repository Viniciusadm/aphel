use std::collections::HashMap;

use sqlx::{MySql, QueryBuilder, Row};
use uuid::Uuid;

use crate::application::fields::{EntityDefinition, SystemField};
use crate::database::Db;
use crate::errors::AppError;
use crate::infrastructure::query::compiler::{push_partition, quoted};
use crate::infrastructure::query::computed::system_expression;

pub async fn record_titles(
    pool: &Db,
    definition: &EntityDefinition,
    field: &SystemField,
    ids: &[Uuid],
) -> Result<HashMap<Uuid, Option<String>>, AppError> {
    let mut titles = HashMap::with_capacity(ids.len());
    if ids.is_empty() {
        return Ok(titles);
    }
    let mut query = QueryBuilder::<MySql>::new(format!(
        "SELECT r.id, {} AS record_label FROM {} r WHERE r.id IN (",
        system_expression(field, "r"),
        quoted(&definition.table)
    ));
    let mut separated = query.separated(", ");
    for id in ids {
        separated.push_bind(id.to_string());
    }
    query.push(")");
    push_partition(&mut query, definition, "r");
    for row in query.build().fetch_all(pool).await? {
        let raw: String = row.try_get("id")?;
        let id = Uuid::parse_str(&raw)
            .map_err(|_| AppError::Internal("invalid stored record identity".into()))?;
        let title =
            super::contributed::decode_record_field(&row, "record_label", field.field_type)?
                .as_str()
                .map(str::to_owned);
        titles.insert(id, title);
    }
    Ok(titles)
}
