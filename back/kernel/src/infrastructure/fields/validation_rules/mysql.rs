use std::collections::HashMap;

use async_trait::async_trait;
use chrono::{NaiveDate, NaiveDateTime};
use rust_decimal::Decimal;
use sqlx::mysql::MySqlRow;
use sqlx::{MySql, MySqlPool, QueryBuilder, Row};
use uuid::Uuid;

use crate::application::fields::validation_rules::{
    ProposedFieldValues, RuleTiming, ValidationRule, ValidationRuleRepository,
};
use crate::application::shared::RecordRef;
use crate::database::transaction::MySqlTx;
use crate::domain::UserId;
use crate::domain::fields::{FieldDefinition, FieldType, FieldValue};
use crate::errors::AppError;
use crate::infrastructure::repositories::mapper::parse_uuid;
use crate::infrastructure::repositories::mapper::{invalid_column, naive, utc};

macro_rules! rule_select {
    ($condition:literal) => {
        concat!(
            "SELECT id, `key`, record_type, name, expression, message, field_key, applies_on, ",
            "is_active, position, created_by, updated_by, created_at, updated_at ",
            "FROM validation_rules ",
            $condition
        )
    };
}

fn optional_user(row: &MySqlRow, column: &str) -> Result<Option<UserId>, AppError> {
    row.try_get::<Option<String>, _>(column)?
        .map(UserId::new)
        .transpose()
        .map_err(Into::into)
}

fn rule_from_row(row: &MySqlRow) -> Result<ValidationRule, AppError> {
    let applies_on: String = row.try_get("applies_on")?;
    Ok(ValidationRule {
        id: parse_uuid(&row.try_get::<String, _>("id")?, "validation_rules.id")?,
        key: row.try_get("key")?,
        record_type: row.try_get("record_type")?,
        name: row.try_get("name")?,
        expression: row.try_get("expression")?,
        message: row.try_get("message")?,
        field: row.try_get("field_key")?,
        applies_on: RuleTiming::parse(&applies_on)
            .ok_or_else(|| invalid_column("validation_rules.applies_on", &applies_on))?,
        is_active: row.try_get("is_active")?,
        position: row.try_get("position")?,
        created_by: optional_user(row, "created_by")?,
        updated_by: optional_user(row, "updated_by")?,
        created_at: utc(row.try_get::<NaiveDateTime, _>("created_at")?),
        updated_at: utc(row.try_get::<NaiveDateTime, _>("updated_at")?),
    })
}

#[derive(Clone)]
pub struct MySqlValidationRuleRepository {
    pool: MySqlPool,
}

impl MySqlValidationRuleRepository {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ValidationRuleRepository for MySqlValidationRuleRepository {
    type Tx = MySqlTx;

    async fn list(&self, record_type: Option<&str>) -> Result<Vec<ValidationRule>, AppError> {
        let rows = match record_type {
            Some(record_type) => {
                sqlx::query(rule_select!(
                    "WHERE record_type = ? ORDER BY position, created_at, id"
                ))
                .bind(record_type)
                .fetch_all(&self.pool)
                .await?
            }
            None => {
                sqlx::query(rule_select!(
                    "ORDER BY record_type, position, created_at, id"
                ))
                .fetch_all(&self.pool)
                .await?
            }
        };
        rows.iter().map(rule_from_row).collect()
    }

    async fn active_for(&self, record_type: &str) -> Result<Vec<ValidationRule>, AppError> {
        let rows = sqlx::query(rule_select!(
            "WHERE record_type = ? AND is_active = TRUE ORDER BY position, created_at, id"
        ))
        .bind(record_type)
        .fetch_all(&self.pool)
        .await?;
        rows.iter().map(rule_from_row).collect()
    }

    async fn find(&self, id: Uuid) -> Result<Option<ValidationRule>, AppError> {
        let row = sqlx::query(rule_select!("WHERE id = ?"))
            .bind(id.to_string())
            .fetch_optional(&self.pool)
            .await?;
        row.as_ref().map(rule_from_row).transpose()
    }

    async fn find_by_key(&self, key: &str) -> Result<Option<ValidationRule>, AppError> {
        let row = sqlx::query(rule_select!("WHERE `key` = ?"))
            .bind(key)
            .fetch_optional(&self.pool)
            .await?;
        row.as_ref().map(rule_from_row).transpose()
    }

    async fn insert(&self, tx: &mut MySqlTx, rule: &ValidationRule) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO validation_rules (id, `key`, record_type, name, \
             expression, message, field_key, applies_on, is_active, position, created_by, \
             updated_by, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(rule.id.to_string())
        .bind(&rule.key)
        .bind(&rule.record_type)
        .bind(&rule.name)
        .bind(&rule.expression)
        .bind(&rule.message)
        .bind(&rule.field)
        .bind(rule.applies_on.as_str())
        .bind(rule.is_active)
        .bind(rule.position)
        .bind(rule.created_by.as_ref().map(UserId::as_str))
        .bind(rule.updated_by.as_ref().map(UserId::as_str))
        .bind(naive(rule.created_at))
        .bind(naive(rule.updated_at))
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    async fn update(&self, tx: &mut MySqlTx, rule: &ValidationRule) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE validation_rules SET name = ?, expression = ?, message = ?, field_key = ?, \
             applies_on = ?, is_active = ?, position = ?, updated_by = ?, updated_at = ? \
             WHERE id = ?",
        )
        .bind(&rule.name)
        .bind(&rule.expression)
        .bind(&rule.message)
        .bind(&rule.field)
        .bind(rule.applies_on.as_str())
        .bind(rule.is_active)
        .bind(rule.position)
        .bind(rule.updated_by.as_ref().map(UserId::as_str))
        .bind(naive(rule.updated_at))
        .bind(rule.id.to_string())
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    async fn delete(&self, tx: &mut MySqlTx, id: Uuid) -> Result<(), AppError> {
        sqlx::query("DELETE FROM validation_rules WHERE id = ?")
            .bind(id.to_string())
            .execute(&mut **tx)
            .await?;
        Ok(())
    }

    async fn set_positions(
        &self,
        tx: &mut MySqlTx,
        positions: &[(Uuid, i32)],
    ) -> Result<(), AppError> {
        for (id, position) in positions {
            sqlx::query("UPDATE validation_rules SET position = ? WHERE id = ?")
                .bind(*position)
                .bind(id.to_string())
                .execute(&mut **tx)
                .await?;
        }
        Ok(())
    }
}

fn decode(field: &FieldDefinition, row: &MySqlRow) -> Result<Option<FieldValue>, AppError> {
    Ok(match field.field_type.base_type() {
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
            .map(|value| FieldValue::DateTime(utc(value))),
        FieldType::Checkbox => row
            .try_get::<Option<bool>, _>("value_bool")?
            .map(FieldValue::Bool),
        FieldType::Select => row
            .try_get::<Option<String>, _>("value_option_id")?
            .and_then(|id| Uuid::parse_str(&id).ok())
            .map(FieldValue::Option),
        FieldType::MultiSelect | FieldType::User | FieldType::Record => None,
        FieldType::Custom { .. } => unreachable!("native field representation"),
    })
}

#[derive(Clone, Default)]
pub struct MySqlProposedFieldValues;

#[async_trait]
impl ProposedFieldValues for MySqlProposedFieldValues {
    type Tx = MySqlTx;

    async fn load(
        &self,
        tx: &mut MySqlTx,
        record: &RecordRef,
        fields: &[FieldDefinition],
    ) -> Result<HashMap<Uuid, FieldValue>, AppError> {
        if fields.is_empty() {
            return Ok(HashMap::new());
        }
        let mut builder: QueryBuilder<MySql> = QueryBuilder::new(
            "SELECT field_id, value_text, value_long_text, value_number, value_date, \
             value_datetime, value_bool, value_option_id FROM custom_field_values \
             WHERE ",
        );
        builder
            .push("record_type = ")
            .push_bind(record.record_type.as_str().to_owned())
            .push(" AND record_id = ")
            .push_bind(record.record_id.to_string())
            .push(" AND field_id IN (");
        let mut ids = builder.separated(", ");
        for field in fields {
            ids.push_bind(field.id.to_string());
        }
        builder.push(")");
        let rows = builder.build().fetch_all(&mut **tx).await?;
        let mut values = HashMap::new();
        for row in &rows {
            let field_id = parse_uuid(
                &row.try_get::<String, _>("field_id")?,
                "custom_field_values.field_id",
            )?;
            let Some(field) = fields.iter().find(|field| field.id == field_id) else {
                continue;
            };
            if let Some(value) = decode(field, row)? {
                values.insert(field_id, value);
            }
        }
        Ok(values)
    }
}
