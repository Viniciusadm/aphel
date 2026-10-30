use std::collections::BTreeMap;

use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use rust_decimal::Decimal;
use sqlx::mysql::MySqlRow;
use sqlx::{MySql, MySqlPool, QueryBuilder, Row};
use uuid::Uuid;

use crate::application::fields::{
    CustomFieldValueRepository, RecordTypeDefinition, StoredFieldValue,
};
use crate::application::shared::{RecordRef, RecordType};
use crate::database::transaction::MySqlTx;
use crate::domain::UserId;
use crate::domain::access::{AccessScope, RecordScopeFilter};
use crate::domain::fields::{FieldDefinition, FieldType, FieldValue, ValueColumn};
use crate::errors::AppError;
use crate::infrastructure::fields::mysql_definitions::uuid_column;
use crate::infrastructure::fields::unique_values;
use crate::infrastructure::query::compiler::quoted;
use crate::infrastructure::repositories::mapper;

#[derive(Clone)]
pub struct MySqlCustomFieldValueRepository {
    pool: MySqlPool,
}

impl MySqlCustomFieldValueRepository {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
    pub async fn load_in_tx(
        tx: &mut MySqlTx,
        record_type: &RecordType,
        fields: &[FieldDefinition],
        record_ids: &[Uuid],
    ) -> Result<Vec<StoredFieldValue>, AppError> {
        if fields.is_empty() || record_ids.is_empty() {
            return Ok(Vec::new());
        }
        let push_filters = |builder: &mut QueryBuilder<MySql>| {
            builder
                .push(" WHERE record_type = ")
                .push_bind(record_type.as_str().to_owned())
                .push(" AND record_id IN (");
            let mut ids = builder.separated(", ");
            for id in record_ids {
                ids.push_bind(id.to_string());
            }
            builder.push(") AND field_id IN (");
            let mut ids = builder.separated(", ");
            for field in fields {
                ids.push_bind(field.id.to_string());
            }
            builder.push(")");
        };
        let mut builder = QueryBuilder::new(
            "SELECT record_id, field_id, value_text, value_long_text, value_number, value_date, \
             value_datetime, value_bool, value_option_id, CAST(NULL AS CHAR(36) CHARACTER SET utf8mb4) COLLATE utf8mb4_unicode_ci AS multi_option_id \
             FROM custom_field_values",
        );
        push_filters(&mut builder);
        builder.push(
            " UNION ALL SELECT record_id, field_id, NULL, NULL, NULL, NULL, NULL, NULL, NULL, option_id \
             FROM custom_field_multi_values",
        );
        push_filters(&mut builder);
        let rows = builder.build().fetch_all(&mut **tx).await?;

        let mut values = Vec::new();
        let mut multi: BTreeMap<(Uuid, Uuid), Vec<Uuid>> = BTreeMap::new();
        for row in &rows {
            let record_id = uuid_column(row, "record_id")?;
            let field_id = uuid_column(row, "field_id")?;
            let Some(field) = fields.iter().find(|field| field.id == field_id) else {
                continue;
            };
            if let Some(option) = row.try_get::<Option<String>, _>("multi_option_id")? {
                if field.field_type == FieldType::MultiSelect
                    && let Ok(option) = Uuid::parse_str(&option)
                {
                    multi.entry((record_id, field_id)).or_default().push(option);
                }
                continue;
            }
            if let Some(value) = decode(field, row)? {
                values.push(StoredFieldValue {
                    record_id,
                    field_id,
                    value,
                });
            }
        }
        values.extend(
            multi
                .into_iter()
                .map(|((record_id, field_id), options)| StoredFieldValue {
                    record_id,
                    field_id,
                    value: FieldValue::Options(options),
                }),
        );
        Ok(values)
    }
}

#[derive(Default)]
struct ColumnValues {
    text: Option<String>,
    long_text: Option<String>,
    number: Option<Decimal>,
    date: Option<NaiveDate>,
    datetime: Option<NaiveDateTime>,
    boolean: Option<bool>,
    option: Option<String>,
}

fn columns_of(value: &FieldValue) -> ColumnValues {
    let mut columns = ColumnValues::default();
    match value {
        FieldValue::Text(text) => columns.text = Some(text.clone()),
        FieldValue::LongText(text) => columns.long_text = Some(text.clone()),
        FieldValue::Number(number) => columns.number = Some(*number),
        FieldValue::Date(date) => columns.date = Some(*date),
        FieldValue::DateTime(instant) => columns.datetime = Some(mapper::naive(*instant)),
        FieldValue::Bool(value) => columns.boolean = Some(*value),
        FieldValue::Option(id) => columns.option = Some(id.to_string()),
        FieldValue::User(id) => columns.text = Some(id.as_str().to_owned()),
        FieldValue::Record(id) => columns.text = Some(id.to_string()),
        FieldValue::Options(_) => {}
    }
    columns
}

fn decode(field: &FieldDefinition, row: &MySqlRow) -> Result<Option<FieldValue>, AppError> {
    Ok(match field.field_type {
        FieldType::Text | FieldType::Url | FieldType::Email | FieldType::Phone => row
            .try_get::<Option<String>, _>("value_text")?
            .map(FieldValue::Text),
        FieldType::LongText => row
            .try_get::<Option<String>, _>("value_long_text")?
            .map(FieldValue::LongText),
        FieldType::Number | FieldType::Currency => row
            .try_get::<Option<Decimal>, _>("value_number")?
            .map(|number| FieldValue::Number(number.normalize())),
        FieldType::Date => row
            .try_get::<Option<NaiveDate>, _>("value_date")?
            .map(FieldValue::Date),
        FieldType::DateTime => row
            .try_get::<Option<NaiveDateTime>, _>("value_datetime")?
            .map(|value| FieldValue::DateTime(mapper::utc(value))),
        FieldType::Checkbox => row
            .try_get::<Option<bool>, _>("value_bool")?
            .map(FieldValue::Bool),
        FieldType::Select => row
            .try_get::<Option<String>, _>("value_option_id")?
            .and_then(|id| Uuid::parse_str(&id).ok())
            .map(FieldValue::Option),
        FieldType::MultiSelect => None,
        FieldType::User => row
            .try_get::<Option<String>, _>("value_text")?
            .and_then(|id| UserId::new(id).ok())
            .map(FieldValue::User),
        FieldType::Record => row
            .try_get::<Option<String>, _>("value_text")?
            .and_then(|id| Uuid::parse_str(&id).ok())
            .map(FieldValue::Record),
    })
}

fn push_value_column(
    builder: &mut QueryBuilder<MySql>,
    field: &FieldDefinition,
    value: &FieldValue,
) {
    let columns = columns_of(value);
    match field.field_type.value_column() {
        ValueColumn::Text => builder.push_bind(columns.text),
        ValueColumn::LongText => builder.push_bind(columns.long_text),
        ValueColumn::Number => builder.push_bind(columns.number),
        ValueColumn::Date => builder.push_bind(columns.date),
        ValueColumn::DateTime => builder.push_bind(columns.datetime),
        ValueColumn::Bool => builder.push_bind(columns.boolean),
        ValueColumn::Option | ValueColumn::MultiOption => builder.push_bind(columns.option),
    };
}

#[async_trait]
impl CustomFieldValueRepository for MySqlCustomFieldValueRepository {
    type Tx = MySqlTx;

    async fn record_timestamp(
        &self,
        definition: &RecordTypeDefinition,
        column: &str,
        record_id: Uuid,
    ) -> Result<Option<DateTime<Utc>>, AppError> {
        let value: Option<Option<NaiveDateTime>> = sqlx::query_scalar(sqlx::AssertSqlSafe(
            format!("SELECT `{column}` FROM `{}` WHERE id = ?", definition.table),
        ))
        .bind(record_id.to_string())
        .fetch_optional(&self.pool)
        .await?;
        Ok(value.flatten().map(|naive| naive.and_utc()))
    }

    async fn load(
        &self,
        record_type: &RecordType,
        fields: &[FieldDefinition],
        record_ids: &[Uuid],
    ) -> Result<Vec<StoredFieldValue>, AppError> {
        if fields.is_empty() || record_ids.is_empty() {
            return Ok(Vec::new());
        }
        let push_filters = |builder: &mut QueryBuilder<MySql>| {
            builder
                .push(" WHERE record_type = ")
                .push_bind(record_type.as_str().to_owned())
                .push(" AND record_id IN (");
            let mut ids = builder.separated(", ");
            for id in record_ids {
                ids.push_bind(id.to_string());
            }
            builder.push(") AND field_id IN (");
            let mut ids = builder.separated(", ");
            for field in fields {
                ids.push_bind(field.id.to_string());
            }
            builder.push(")");
        };
        let mut builder = QueryBuilder::new(
            "SELECT record_id, field_id, value_text, value_long_text, value_number, value_date, \
             value_datetime, value_bool, value_option_id, CAST(NULL AS CHAR(36) CHARACTER SET utf8mb4) COLLATE utf8mb4_unicode_ci AS multi_option_id \
             FROM custom_field_values",
        );
        push_filters(&mut builder);
        builder.push(
            " UNION ALL SELECT record_id, field_id, NULL, NULL, NULL, NULL, NULL, NULL, NULL, option_id \
             FROM custom_field_multi_values",
        );
        push_filters(&mut builder);
        let rows = builder.build().fetch_all(&self.pool).await?;

        let mut values = Vec::new();
        let mut multi: BTreeMap<(Uuid, Uuid), Vec<Uuid>> = BTreeMap::new();
        for row in &rows {
            let record_id = uuid_column(row, "record_id")?;
            let field_id = uuid_column(row, "field_id")?;
            let Some(field) = fields.iter().find(|field| field.id == field_id) else {
                continue;
            };
            if let Some(option) = row.try_get::<Option<String>, _>("multi_option_id")? {
                if field.field_type == FieldType::MultiSelect
                    && let Ok(option) = Uuid::parse_str(&option)
                {
                    multi.entry((record_id, field_id)).or_default().push(option);
                }
                continue;
            }
            if let Some(value) = decode(field, row)? {
                values.push(StoredFieldValue {
                    record_id,
                    field_id,
                    value,
                });
            }
        }
        values.extend(
            multi
                .into_iter()
                .map(|((record_id, field_id), options)| StoredFieldValue {
                    record_id,
                    field_id,
                    value: FieldValue::Options(options),
                }),
        );
        Ok(values)
    }

    async fn upsert(
        &self,
        tx: &mut MySqlTx,
        record: &RecordRef,
        field: &FieldDefinition,
        value: &FieldValue,
    ) -> Result<(), AppError> {
        if let FieldValue::Options(options) = value {
            self.delete(tx, record, field.id).await?;
            if options.is_empty() {
                return Ok(());
            }
            let mut builder = QueryBuilder::new(
                "INSERT INTO custom_field_multi_values (record_type, record_id, field_id, option_id) ",
            );
            builder.push_values(options, |mut values, option| {
                values
                    .push_bind(record.record_type.as_str().to_owned())
                    .push_bind(record.record_id.to_string())
                    .push_bind(field.id.to_string())
                    .push_bind(option.to_string());
            });
            builder.build().execute(&mut **tx).await?;
            return Ok(());
        }

        let columns = columns_of(value);
        sqlx::query(
            "INSERT INTO custom_field_values (record_type, record_id, field_id, \
             value_text, value_long_text, value_number, value_date, value_datetime, value_bool, value_option_id) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?) AS new ON DUPLICATE KEY UPDATE \
             value_text = new.value_text, value_long_text = new.value_long_text, \
             value_number = new.value_number, value_date = new.value_date, \
             value_datetime = new.value_datetime, value_bool = new.value_bool, \
             value_option_id = new.value_option_id",
        )
        .bind(record.record_type.as_str())
        .bind(record.record_id.to_string())
        .bind(field.id.to_string())
        .bind(columns.text)
        .bind(columns.long_text)
        .bind(columns.number)
        .bind(columns.date)
        .bind(columns.datetime)
        .bind(columns.boolean)
        .bind(columns.option)
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    async fn delete(
        &self,
        tx: &mut MySqlTx,
        record: &RecordRef,
        field_id: Uuid,
    ) -> Result<(), AppError> {
        for table in ["custom_field_values", "custom_field_multi_values"] {
            sqlx::query(sqlx::AssertSqlSafe(format!(
                "DELETE FROM {table} WHERE record_type = ? AND record_id = ? AND field_id = ?"
            )))
            .bind(record.record_type.as_str())
            .bind(record.record_id.to_string())
            .bind(field_id.to_string())
            .execute(&mut **tx)
            .await?;
        }
        Ok(())
    }

    async fn delete_record(&self, tx: &mut MySqlTx, record: &RecordRef) -> Result<(), AppError> {
        unique_values::release_record(tx, record).await?;
        for table in ["custom_field_values", "custom_field_multi_values"] {
            sqlx::query(sqlx::AssertSqlSafe(format!(
                "DELETE FROM {table} WHERE record_type = ? AND record_id = ?"
            )))
            .bind(record.record_type.as_str())
            .bind(record.record_id.to_string())
            .execute(&mut **tx)
            .await?;
        }
        Ok(())
    }

    async fn is_taken(
        &self,
        tx: &mut MySqlTx,
        definition: &RecordTypeDefinition,
        record: &RecordRef,
        field: &FieldDefinition,
        value: &FieldValue,
    ) -> Result<bool, AppError> {
        let column = field.field_type.value_column();
        if column == ValueColumn::MultiOption {
            return Ok(false);
        }
        sqlx::query("SELECT id FROM field_definitions WHERE id = ? FOR UPDATE")
            .bind(field.id.to_string())
            .fetch_optional(&mut **tx)
            .await?;
        let mut builder =
            QueryBuilder::new("SELECT EXISTS(SELECT 1 FROM custom_field_values v WHERE ");
        push_taken_condition(&mut builder, record, field, value);
        if let Some(deleted) = &definition.deleted_column {
            builder.push(format!(
                " AND NOT EXISTS (SELECT 1 FROM {} r WHERE r.id = v.record_id AND r.{} IS NOT NULL)",
                quoted(&definition.table),
                quoted(deleted)
            ));
        }
        builder.push(")");
        let taken: bool = builder.build_query_scalar().fetch_one(&mut **tx).await?;
        Ok(taken)
    }

    async fn has_duplicates(
        &self,
        definition: &RecordTypeDefinition,
        field: &FieldDefinition,
    ) -> Result<bool, AppError> {
        let column = field.field_type.value_column();
        if column == ValueColumn::MultiOption {
            return Ok(false);
        }
        let column = column.column_name();
        let mut builder = QueryBuilder::new(
            "SELECT EXISTS(SELECT 1 FROM custom_field_values v WHERE v.field_id = ",
        );
        builder
            .push_bind(field.id.to_string())
            .push(format!(" AND v.{column} IS NOT NULL"));
        if let Some(deleted) = &definition.deleted_column {
            builder.push(format!(
                " AND NOT EXISTS (SELECT 1 FROM {} r WHERE r.id = v.record_id AND r.{} IS NOT NULL)",
                quoted(&definition.table),
                quoted(deleted)
            ));
        }
        builder.push(format!(" GROUP BY v.{column} HAVING COUNT(*) > 1)"));
        let duplicated: bool = builder.build_query_scalar().fetch_one(&self.pool).await?;
        Ok(duplicated)
    }

    async fn reserve_unique(
        &self,
        tx: &mut MySqlTx,
        definition: &RecordTypeDefinition,
        record: &RecordRef,
        field: &FieldDefinition,
    ) -> Result<(), AppError> {
        unique_values::reserve(tx, definition, record, field).await
    }

    async fn release_unique(
        &self,
        tx: &mut MySqlTx,
        record: &RecordRef,
        field_id: Uuid,
    ) -> Result<(), AppError> {
        unique_values::release(tx, record, field_id).await
    }

    async fn existing_records_in(
        &self,
        tx: &mut MySqlTx,
        scope: &AccessScope,
        target: &RecordTypeDefinition,
        visibility: &RecordScopeFilter,
        record_ids: &[Uuid],
    ) -> Result<Option<Vec<Uuid>>, AppError> {
        Ok(Some(
            crate::infrastructure::fields::mysql_references::existing_records_in(
                tx, scope, target, visibility, record_ids,
            )
            .await?,
        ))
    }

    async fn rebuild_unique(
        &self,
        tx: &mut MySqlTx,
        definition: &RecordTypeDefinition,
        field: &FieldDefinition,
    ) -> Result<(), AppError> {
        unique_values::rebuild(tx, definition, field).await
    }
}

fn push_taken_condition(
    builder: &mut QueryBuilder<MySql>,
    record: &RecordRef,
    field: &FieldDefinition,
    value: &FieldValue,
) {
    builder
        .push("v.field_id = ")
        .push_bind(field.id.to_string())
        .push(format!(
            " AND v.{} = ",
            field.field_type.value_column().column_name()
        ));
    push_value_column(builder, field, value);
    builder
        .push(" AND NOT (v.record_type = ")
        .push_bind(record.record_type.as_str().to_owned())
        .push(" AND v.record_id = ")
        .push_bind(record.record_id.to_string())
        .push(")");
}
