use std::collections::HashMap;

use async_trait::async_trait;
use chrono::{NaiveDate, NaiveDateTime};
use chrono_tz::Tz;
use rust_decimal::Decimal;
use sqlx::mysql::MySqlRow;
use sqlx::{MySql, MySqlPool, QueryBuilder, Row};
use uuid::Uuid;

use crate::application::{
    AggregateColumn, AggregateFunction, AggregateQuery, AggregateSource, FormulaSource,
};
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::application::fields::{EntityDefinition, RelationPath, SystemField};
use aphel_sdk::platform::application::shared::RecordType;
use aphel_sdk::platform::domain::access::{AccessScope, RecordScopeFilter};
use aphel_sdk::platform::domain::fields::{FORMULA_FIELD_TYPE, FieldType, FieldValue};
use aphel_sdk::platform::infrastructure::fields::mysql_definitions::uuid_column;
use aphel_sdk::platform::infrastructure::query::compiler::push_live_record;
use aphel_sdk::platform::infrastructure::query::computed::system_expression;
use aphel_sdk::platform::infrastructure::repositories::mapper;

#[derive(Clone)]
pub struct MySqlFormulaSource {
    pool: MySqlPool,
}

impl MySqlFormulaSource {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

fn parse_timezone(timezone: Option<String>) -> Tz {
    timezone
        .and_then(|timezone| timezone.parse::<Tz>().ok())
        .unwrap_or(chrono_tz::UTC)
}

fn push_ids(builder: &mut QueryBuilder<MySql>, ids: &[Uuid]) {
    builder.push("(");
    let mut separated = builder.separated(", ");
    for id in ids {
        separated.push_bind(id.to_string());
    }
    builder.push(")");
}

fn system_value(
    field: &SystemField,
    row: &MySqlRow,
    alias: &str,
) -> Result<Option<FieldValue>, AppError> {
    Ok(match field.field_type.base_type() {
        FieldType::Number | FieldType::Currency => row
            .try_get::<Option<Decimal>, _>(alias)?
            .map(|number| FieldValue::Number(number.normalize())),
        FieldType::Date => row
            .try_get::<Option<NaiveDate>, _>(alias)?
            .map(FieldValue::Date),
        FieldType::DateTime => row
            .try_get::<Option<NaiveDateTime>, _>(alias)?
            .map(|value| FieldValue::DateTime(mapper::utc(value))),
        FieldType::Checkbox => row.try_get::<Option<bool>, _>(alias)?.map(FieldValue::Bool),
        FieldType::Text
        | FieldType::LongText
        | FieldType::Url
        | FieldType::Email
        | FieldType::Phone
        | FieldType::Select => row
            .try_get::<Option<String>, _>(alias)?
            .map(FieldValue::Text),
        FieldType::MultiSelect | FieldType::User | FieldType::Record => None,
        FieldType::Custom { .. } => unreachable!("native field representation"),
    })
}

fn selected_column(field: &SystemField) -> String {
    let expression = system_expression(field, "r");
    match field.field_type.base_type() {
        FieldType::Number | FieldType::Currency => {
            format!("CAST({expression} AS DECIMAL(30,6))")
        }
        FieldType::Text
        | FieldType::LongText
        | FieldType::Url
        | FieldType::Email
        | FieldType::Phone
        | FieldType::Select => format!("CAST({expression} AS CHAR)"),
        _ => expression,
    }
}

fn aggregate_expression(function: AggregateFunction, value: &str) -> String {
    match function {
        AggregateFunction::Count => "COUNT(*)".to_owned(),
        AggregateFunction::Sum => format!("SUM({value})"),
        AggregateFunction::Avg => format!("AVG({value})"),
        AggregateFunction::Min => format!("MIN({value})"),
        AggregateFunction::Max => format!("MAX({value})"),
    }
}

fn value_expression(column: Option<&AggregateColumn>) -> String {
    match column {
        Some(AggregateColumn::Custom(_)) => "v.value_number".to_owned(),
        Some(AggregateColumn::System(column)) => format!("t.{column}"),
        Some(AggregateColumn::ItemColumn(column)) => format!("t.{column}"),
        None => "1".to_owned(),
    }
}

fn push_value_join(
    builder: &mut QueryBuilder<MySql>,
    target: &EntityDefinition,
    column: Option<&AggregateColumn>,
) {
    if let Some(AggregateColumn::Custom(field_id)) = column {
        builder
            .push(" LEFT JOIN custom_field_values v ON v.record_id = t.id AND v.record_type = ")
            .push_bind(target.record_type.as_str().to_owned())
            .push(" AND v.field_id = ")
            .push_bind(field_id.to_string());
    }
}

#[async_trait]
impl FormulaSource for MySqlFormulaSource {
    async fn timezone(&self) -> Result<Tz, AppError> {
        let timezone: Option<String> =
            sqlx::query_scalar("SELECT timezone FROM organization_settings WHERE id = 1")
                .fetch_optional(&self.pool)
                .await?;
        Ok(parse_timezone(timezone))
    }

    async fn formula_record_types(&self) -> Result<Vec<RecordType>, AppError> {
        let types: Vec<String> = sqlx::query_scalar(
            "SELECT DISTINCT record_type FROM field_definitions \
             WHERE field_type = ? AND is_active ORDER BY record_type",
        )
        .bind(FORMULA_FIELD_TYPE)
        .fetch_all(&self.pool)
        .await?;
        Ok(types
            .into_iter()
            .filter_map(|record_type| RecordType::parse(record_type).ok())
            .collect())
    }

    async fn system_values(
        &self,
        scope: &AccessScope,
        definition: &EntityDefinition,
        fields: &[SystemField],
        record_ids: &[Uuid],
    ) -> Result<HashMap<Uuid, HashMap<String, FieldValue>>, AppError> {
        if record_ids.is_empty() {
            return Ok(HashMap::new());
        }
        let mut builder = QueryBuilder::new("SELECT r.id");
        for (index, field) in fields.iter().enumerate() {
            builder.push(format!(", {} AS c{index}", selected_column(field)));
        }
        builder.push(format!(" FROM {} r WHERE ", definition.table));
        push_live_record(
            &mut builder,
            scope,
            definition,
            &RecordScopeFilter::Organization,
            "r",
        );
        builder.push(" AND r.id IN ");
        push_ids(&mut builder, record_ids);
        let rows = builder.build().fetch_all(&self.pool).await?;
        let mut values = HashMap::with_capacity(rows.len());
        for row in &rows {
            let id = uuid_column(row, "id")?;
            let mut record = HashMap::new();
            for (index, field) in fields.iter().enumerate() {
                if let Some(value) = system_value(field, row, &format!("c{index}"))? {
                    record.insert(field.key.as_str().to_owned(), value);
                }
            }
            values.insert(id, record);
        }
        Ok(values)
    }

    async fn aggregate(
        &self,
        scope: &AccessScope,
        query: &AggregateQuery<'_>,
        record_ids: &[Uuid],
    ) -> Result<HashMap<Uuid, Decimal>, AppError> {
        if record_ids.is_empty() {
            return Ok(HashMap::new());
        }
        let mut builder: QueryBuilder<MySql>;
        match query.source {
            AggregateSource::Items(table) => {
                let value = match query.column {
                    Some(AggregateColumn::ItemColumn(column)) => format!("t.{column}"),
                    _ => table
                        .columns
                        .first()
                        .map_or_else(|| "1".to_owned(), |column| format!("t.{}", column.column)),
                };
                let parent = &table.parent_column;
                builder = QueryBuilder::new(format!(
                    "SELECT t.{parent} AS parent_id, CAST({} AS DECIMAL(30,6)) AS value \
                     FROM {} t WHERE ",
                    aggregate_expression(query.function, &value),
                    table.table
                ));
                builder.push(format!("t.{parent} IN "));
                push_ids(&mut builder, record_ids);
                if let Some((column, value)) = &table.partition {
                    builder
                        .push(format!(" AND t.{column} = "))
                        .push_bind(value.clone());
                }
                builder.push(format!(" GROUP BY t.{parent}"));
            }
            AggregateSource::HasMany { target, column } => {
                builder = QueryBuilder::new(format!("SELECT t.{column} AS parent_id, "));
                let value = value_expression(query.column);
                builder.push(format!(
                    "CAST({} AS DECIMAL(30,6)) AS value FROM {} t",
                    aggregate_expression(query.function, &value),
                    target.table
                ));
                push_value_join(&mut builder, target, query.column);
                builder.push(" WHERE ");
                push_live_record(
                    &mut builder,
                    scope,
                    target,
                    &RecordScopeFilter::Organization,
                    "t",
                );
                builder.push(format!(" AND t.{column} IN "));
                push_ids(&mut builder, record_ids);
                builder.push(format!(" GROUP BY t.{column}"));
            }
            AggregateSource::Relationship {
                target,
                relationship_id,
                inverse,
            } => {
                let (source_column, target_column) = RelationPath::link_columns(*inverse);
                let value = value_expression(query.column);
                builder = QueryBuilder::new(format!(
                    "SELECT l.{source_column} AS parent_id, CAST({} AS DECIMAL(30,6)) AS value \
                     FROM record_relations l JOIN {} t ON t.id = l.{target_column} \
                    ",
                    aggregate_expression(query.function, &value),
                    target.table
                ));
                push_value_join(&mut builder, target, query.column);
                builder
                    .push(" WHERE l.relationship_id = ")
                    .push_bind(relationship_id.to_string())
                    .push(" AND ");
                push_live_record(
                    &mut builder,
                    scope,
                    target,
                    &RecordScopeFilter::Organization,
                    "t",
                );
                builder.push(format!(" AND l.{source_column} IN "));
                push_ids(&mut builder, record_ids);
                builder.push(format!(" GROUP BY l.{source_column}"));
            }
        }
        let rows = builder.build().fetch_all(&self.pool).await?;
        let mut values = HashMap::with_capacity(rows.len());
        for row in &rows {
            let parent: Option<String> = row.try_get("parent_id")?;
            let value: Option<Decimal> = row.try_get("value")?;
            if let (Some(parent), Some(value)) = (parent, value)
                && let Ok(parent) = Uuid::parse_str(&parent)
            {
                values.insert(parent, value.normalize());
            }
        }
        Ok(values)
    }

    async fn parents(
        &self,
        source: &AggregateSource,
        child_ids: &[Uuid],
    ) -> Result<Vec<Uuid>, AppError> {
        if child_ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut builder: QueryBuilder<MySql> = match source {
            AggregateSource::Items(_) => return Ok(Vec::new()),
            AggregateSource::HasMany { target, column } => {
                let mut builder = QueryBuilder::new(format!(
                    "SELECT DISTINCT t.{column} AS parent_id FROM {} t WHERE ",
                    target.table
                ));
                builder.push("t.id IN ");
                push_ids(&mut builder, child_ids);
                builder
            }
            AggregateSource::Relationship {
                relationship_id,
                inverse,
                ..
            } => {
                let (source_column, target_column) = RelationPath::link_columns(*inverse);
                let mut builder = QueryBuilder::new(format!(
                    "SELECT DISTINCT l.{source_column} AS parent_id FROM record_relations l \
                     WHERE "
                ));
                builder
                    .push("l.relationship_id = ")
                    .push_bind(relationship_id.to_string())
                    .push(format!(" AND l.{target_column} IN "));
                push_ids(&mut builder, child_ids);
                builder
            }
        };
        let rows: Vec<Option<String>> = builder.build_query_scalar().fetch_all(&self.pool).await?;
        Ok(rows
            .into_iter()
            .flatten()
            .filter_map(|id| Uuid::parse_str(&id).ok())
            .collect())
    }

    async fn record_ids(
        &self,
        scope: &AccessScope,
        definition: &EntityDefinition,
        after: Option<Uuid>,
        limit: u32,
    ) -> Result<Vec<Uuid>, AppError> {
        let mut builder =
            QueryBuilder::new(format!("SELECT r.id FROM {} r WHERE ", definition.table));
        push_live_record(
            &mut builder,
            scope,
            definition,
            &RecordScopeFilter::Organization,
            "r",
        );
        if let Some(after) = after {
            builder.push(" AND r.id > ").push_bind(after.to_string());
        }
        builder.push(" ORDER BY r.id LIMIT ").push_bind(limit);
        let rows: Vec<String> = builder.build_query_scalar().fetch_all(&self.pool).await?;
        rows.into_iter()
            .map(|id| {
                Uuid::parse_str(&id)
                    .map_err(|_| AppError::Persistence(format!("record id {id:?} is not a UUID")))
            })
            .collect()
    }

    async fn mark_daily_run(&self, local_date: NaiveDate) -> Result<bool, AppError> {
        let updated = sqlx::query(
            "UPDATE formula_daily_runs SET local_date = ?, ran_at = UTC_TIMESTAMP(6) \
             WHERE id = 1 AND local_date <> ?",
        )
        .bind(local_date)
        .bind(local_date)
        .execute(&self.pool)
        .await?;
        if updated.rows_affected() > 0 {
            return Ok(true);
        }
        let inserted = sqlx::query(
            "INSERT IGNORE INTO formula_daily_runs (id, local_date, ran_at) \
             VALUES (1, ?, UTC_TIMESTAMP(6))",
        )
        .bind(local_date)
        .execute(&self.pool)
        .await?;
        Ok(inserted.rows_affected() > 0)
    }
}
