use aphel_sdk::configuration::ConfigReferenceKind;
use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex};

use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::types::Json;
use sqlx::{AssertSqlSafe, MySql, MySqlPool, QueryBuilder, Row};
use uuid::Uuid;

use crate::{ColumnInfo, ColumnKind, ConfigKind, DataRow, MemberRow, OrganizationProfile};
use aphel_kernel::Ctx;
use aphel_kernel::domain::UserId;
use aphel_kernel::errors::AppError;
use aphel_sdk::configuration::{configuration_keys_available, stored_reference_keys};
use aphel_sdk::platform::application::fields::{RecordTypeRegistry, is_sql_identifier};
use aphel_sdk::platform::application::organization_admin::ListWindow;
use aphel_sdk::platform::application::organization_profile::OrganizationRegistration;
use aphel_sdk::platform::database::transaction::MySqlTx;
use aphel_sdk::platform::domain::access::RoleKey;
use aphel_sdk::platform::infrastructure::query::compiler::quoted;
use aphel_sdk::platform::infrastructure::repositories::mapper;
use aphel_sdk::platform::infrastructure::repositories::mysql_organization_profile::insert_registration;

pub(crate) type ConfigKeyMap = BTreeMap<ConfigKind, BTreeMap<Uuid, String>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RowSource<'a> {
    Table {
        table: &'a str,
        partition: Option<(&'a str, Uuid)>,
    },
    AttachmentFiles,
    RecordEvents {
        record_types: &'a [String],
    },
}

#[derive(Debug, Clone)]
struct TableShape {
    columns: Vec<ColumnInfo>,
    primary_key: Vec<String>,
}

#[derive(Clone)]
pub(crate) struct OrgDataStore {
    pool: MySqlPool,
    shapes: Arc<Mutex<HashMap<String, TableShape>>>,
}

impl OrgDataStore {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self {
            pool,
            shapes: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub(crate) fn of(ctx: &Ctx<'_>) -> Self {
        Self::new(ctx.pool().clone())
    }

    pub(crate) async fn operator(&self, email: Option<&str>) -> Result<Option<UserId>, AppError> {
        let id: Option<String> = match email {
            Some(email) => {
                sqlx::query_scalar(
                    "SELECT id FROM users WHERE status = 'active' AND LOWER(email) = ?",
                )
                .bind(email.trim().to_lowercase())
                .fetch_optional(&self.pool)
                .await?
            }
            None => {
                sqlx::query_scalar(
                    "SELECT u.id FROM users u JOIN roles r ON r.id = u.role_id \
                     WHERE u.status = 'active' AND r.`key` = 'admin' ORDER BY u.created_at, u.id LIMIT 1",
                )
                .fetch_optional(&self.pool)
                .await?
            }
        };
        id.map(UserId::new).transpose().map_err(Into::into)
    }

    async fn shape(&self, table: &str) -> Result<TableShape, AppError> {
        if !is_sql_identifier(table) {
            return Err(AppError::Internal(format!("invalid table name {table:?}")));
        }
        if let Some(shape) = self
            .shapes
            .lock()
            .ok()
            .and_then(|shapes| shapes.get(table).cloned())
        {
            return Ok(shape);
        }
        let columns = sqlx::query(
            "SELECT column_name AS name, is_nullable AS nullable, data_type AS data_type, extra AS extra \
             FROM information_schema.columns WHERE table_schema = DATABASE() AND table_name = ? \
             ORDER BY ordinal_position",
        )
        .bind(table)
        .fetch_all(&self.pool)
        .await?
        .iter()
        .map(|row| {
            let name: String = row.try_get("name")?;
            let nullable: String = row.try_get("nullable")?;
            let data_type: String = row.try_get("data_type")?;
            let extra: String = row.try_get("extra")?;
            Ok(ColumnInfo {
                name,
                nullable: nullable.eq_ignore_ascii_case("yes"),
                generated: {
                    let extra = extra.to_ascii_lowercase();
                    extra.contains("virtual generated") || extra.contains("stored generated")
                },
                kind: column_kind(&data_type),
            })
        })
        .collect::<Result<Vec<_>, AppError>>()?;
        let primary_key: Vec<String> = sqlx::query_scalar(
            "SELECT column_name FROM information_schema.key_column_usage \
             WHERE table_schema = DATABASE() AND table_name = ? AND constraint_name = 'PRIMARY' \
             ORDER BY ordinal_position",
        )
        .bind(table)
        .fetch_all(&self.pool)
        .await?;
        let shape = TableShape {
            columns,
            primary_key,
        };
        if let Ok(mut shapes) = self.shapes.lock() {
            shapes.insert(table.to_owned(), shape.clone());
        }
        Ok(shape)
    }

    async fn select_rows(
        &self,
        table: &str,
        filter: impl FnOnce(&mut QueryBuilder<MySql>),
        window: ListWindow,
    ) -> Result<Vec<DataRow>, AppError> {
        let shape = self.shape(table).await?;
        if shape.columns.is_empty() {
            return Ok(Vec::new());
        }
        let pairs: Vec<String> = shape
            .columns
            .iter()
            .filter(|column| is_sql_identifier(&column.name))
            .map(|column| format!("'{}', {}", column.name, export_expression(column)))
            .collect();
        let order: Vec<String> = if shape.primary_key.is_empty() {
            shape
                .columns
                .iter()
                .map(|column| format!("r.{}", quoted(&column.name)))
                .collect()
        } else {
            shape
                .primary_key
                .iter()
                .map(|column| format!("r.{}", quoted(column)))
                .collect()
        };
        let mut builder = QueryBuilder::new(format!(
            "SELECT JSON_OBJECT({}) AS data_row FROM {} r WHERE 1 = 1",
            pairs.join(", "),
            quoted(table)
        ));
        filter(&mut builder);
        builder
            .push(format!(" ORDER BY {} LIMIT ", order.join(", ")))
            .push_bind(window.limit)
            .push(" OFFSET ")
            .push_bind(window.offset);
        builder
            .build()
            .fetch_all(&self.pool)
            .await?
            .iter()
            .map(|row| {
                let Json(value): Json<Value> = row.try_get("data_row")?;
                match value {
                    Value::Object(map) => Ok(map),
                    _ => Err(AppError::Internal("unexpected row shape".to_owned())),
                }
            })
            .collect()
    }
}

fn column_kind(data_type: &str) -> ColumnKind {
    match data_type.to_ascii_lowercase().as_str() {
        "decimal" => ColumnKind::Decimal,
        "date" => ColumnKind::Date,
        "datetime" | "timestamp" => ColumnKind::DateTime,
        "json" => ColumnKind::Json,
        "tinyint" | "smallint" | "mediumint" | "int" | "bigint" | "float" | "double" => {
            ColumnKind::Number
        }
        _ => ColumnKind::Text,
    }
}

fn export_expression(column: &ColumnInfo) -> String {
    let name = format!("r.{}", quoted(&column.name));
    match column.kind {
        ColumnKind::DateTime => format!("DATE_FORMAT({name}, '%Y-%m-%dT%H:%i:%s.%fZ')"),
        ColumnKind::Date => format!("DATE_FORMAT({name}, '%Y-%m-%d')"),
        ColumnKind::Decimal => format!("CAST({name} AS CHAR)"),
        ColumnKind::Json | ColumnKind::Number | ColumnKind::Text => name,
    }
}

fn bind_value(column: &ColumnInfo, value: &Value) -> Option<String> {
    match value {
        Value::Null => None,
        other if column.kind == ColumnKind::Json => serde_json::to_string(other).ok(),
        Value::Bool(flag) => Some(if *flag { "1" } else { "0" }.to_owned()),
        Value::Number(number) => Some(number.to_string()),
        Value::String(text) if column.kind == ColumnKind::DateTime => Some(
            text.trim_end_matches('Z')
                .trim_end_matches("+00:00")
                .replacen('T', " ", 1),
        ),
        Value::String(text) => Some(text.clone()),
        other => serde_json::to_string(other).ok(),
    }
}

fn uuid_text(value: String) -> Result<Uuid, AppError> {
    Uuid::parse_str(&value).map_err(|_| mapper::invalid_column("id", &value))
}

impl OrgDataStore {
    pub(crate) async fn profile(&self) -> Result<OrganizationProfile, AppError> {
        let row = sqlx::query(
            "SELECT name, locale, timezone, currency FROM organization_settings WHERE id = 1",
        )
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::NotFound("organization was not found".to_owned()))?;
        Ok(OrganizationProfile {
            name: row.try_get("name")?,
            locale: row.try_get("locale")?,
            timezone: row.try_get("timezone")?,
            currency: row.try_get("currency")?,
        })
    }

    pub(crate) async fn members(&self) -> Result<Vec<MemberRow>, AppError> {
        sqlx::query(
            "SELECT u.id, u.email, u.display_name, r.`key` AS role, u.status, u.password_hash, u.locale \
             FROM users u JOIN roles r ON r.id = u.role_id ORDER BY u.email",
        )
        .fetch_all(&self.pool)
        .await?
        .iter()
        .map(|row| {
            Ok(MemberRow {
                id: row.try_get("id")?,
                email: row.try_get("email")?,
                display_name: row.try_get("display_name")?,
                role: row.try_get("role")?,
                status: row.try_get("status")?,
                password_hash: row.try_get("password_hash")?,
                locale: row.try_get("locale")?,
            })
        })
        .collect()
    }

    pub(crate) async fn rows(
        &self,
        source: RowSource<'_>,
        window: ListWindow,
    ) -> Result<Vec<DataRow>, AppError> {
        match source {
            RowSource::Table { table, partition } => {
                self.select_rows(
                    table,
                    |builder| {
                        if let Some((column, value)) = partition
                            && is_sql_identifier(column)
                        {
                            builder
                                .push(format!(" AND r.{} = ", quoted(column)))
                                .push_bind(value.to_string());
                        }
                    },
                    window,
                )
                .await
            }
            RowSource::AttachmentFiles => {
                self.select_rows(
                    "files",
                    |builder| {
                        builder.push(" AND r.id IN (SELECT a.file_id FROM attachments a)");
                    },
                    window,
                )
                .await
            }
            RowSource::RecordEvents { record_types } => {
                if record_types.is_empty() {
                    return Ok(Vec::new());
                }
                self.select_rows(
                    "domain_events",
                    |builder| {
                        builder.push(" AND r.record_type IN (");
                        let mut separated = builder.separated(", ");
                        for record_type in record_types {
                            separated.push_bind(record_type.clone());
                        }
                        builder.push(")");
                    },
                    window,
                )
                .await
            }
        }
    }

    pub(crate) async fn config_keys(&self) -> Result<ConfigKeyMap, AppError> {
        let mut map: ConfigKeyMap = BTreeMap::new();
        let keyed = configuration_keys_available();
        for (kind, reference) in [
            (ConfigKind::Pipeline, aphel_contract_crm::keys::PIPELINE),
            (
                ConfigKind::PipelineStage,
                aphel_contract_crm::keys::PIPELINE_STAGE,
            ),
            (
                ConfigKind::LostReason,
                aphel_contract_crm::keys::LOST_REASON,
            ),
            (ConfigKind::Tag, "tagging.tag"),
            (ConfigKind::ActivityType, "tasks.activity_type"),
        ] {
            if !keyed {
                map.insert(kind, BTreeMap::new());
                continue;
            }
            map.insert(
                kind,
                stored_reference_keys(&self.pool, ConfigReferenceKind::new(reference))
                    .await?
                    .into_iter()
                    .collect(),
            );
        }
        let rows = sqlx::query("SELECT id, `key` FROM relationship_definitions")
            .fetch_all(&self.pool)
            .await?;
        for row in rows {
            let id: String = row.try_get("id")?;
            let key: String = row.try_get("key")?;
            map.entry(ConfigKind::Relationship)
                .or_default()
                .insert(uuid_text(id)?, key);
        }
        Ok(map)
    }
}

impl OrgDataStore {
    pub(crate) async fn has_business_data(&self, tables: &[String]) -> Result<bool, AppError> {
        for table in tables {
            if !is_sql_identifier(table) || self.shape(table).await?.columns.is_empty() {
                continue;
            }
            let exists: i64 = sqlx::query_scalar(AssertSqlSafe(format!(
                "SELECT EXISTS(SELECT 1 FROM {})",
                quoted(table)
            )))
            .fetch_one(&self.pool)
            .await?;
            if exists != 0 {
                return Ok(true);
            }
        }
        Ok(false)
    }

    pub(crate) async fn table_exists(&self, table: &str) -> Result<bool, AppError> {
        Ok(!self.shape(table).await?.columns.is_empty())
    }

    pub(crate) async fn self_references(&self, table: &str) -> Result<Vec<String>, AppError> {
        Ok(sqlx::query_scalar(
            "SELECT column_name FROM information_schema.key_column_usage \
             WHERE table_schema = DATABASE() AND table_name = ? AND referenced_table_name = table_name \
             ORDER BY ordinal_position",
        )
        .bind(table)
        .fetch_all(&self.pool)
        .await?)
    }

    pub(crate) async fn update_reference(
        &self,
        tx: &mut MySqlTx,
        table: &str,
        column: &str,
        id: &str,
        value: &str,
    ) -> Result<(), AppError> {
        if !is_sql_identifier(table) || !is_sql_identifier(column) {
            return Err(AppError::Internal(format!(
                "invalid reference {table:?}.{column:?}"
            )));
        }
        sqlx::query(AssertSqlSafe(format!(
            "UPDATE {} SET {} = ? WHERE id = ?",
            quoted(table),
            quoted(column)
        )))
        .bind(value)
        .bind(id)
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub(crate) async fn columns(&self, table: &str) -> Result<Vec<ColumnInfo>, AppError> {
        Ok(self.shape(table).await?.columns)
    }

    pub(crate) async fn taken_ids(&self, table: &str, ids: &[Uuid]) -> Result<u64, AppError> {
        if ids.is_empty() {
            return Ok(0);
        }
        let shape = self.shape(table).await?;
        if !shape.columns.iter().any(|column| column.name == "id") {
            return Ok(0);
        }
        let mut builder = QueryBuilder::new(format!(
            "SELECT COUNT(*) FROM {} WHERE id IN (",
            quoted(table)
        ));
        let mut separated = builder.separated(", ");
        for id in ids {
            separated.push_bind(id.to_string());
        }
        builder.push(")");
        let count: i64 = builder.build_query_scalar().fetch_one(&self.pool).await?;
        Ok(count as u64)
    }

    pub(crate) async fn create_installation(
        &self,
        tx: &mut MySqlTx,
        registration: &OrganizationRegistration,
        registry: &RecordTypeRegistry,
    ) -> Result<bool, AppError> {
        let available: bool =
            sqlx::query_scalar("SELECT NOT EXISTS(SELECT 1 FROM organization_settings FOR UPDATE)")
                .fetch_one(&mut **tx)
                .await?;
        if !available {
            return Ok(false);
        }
        insert_registration(tx, registration, registry).await?;
        sqlx::query("UPDATE users SET email_verified_at = joined_at WHERE id = ?")
            .bind(registration.owner.id.as_str())
            .execute(&mut **tx)
            .await?;
        Ok(true)
    }

    pub(crate) async fn insert_member(
        &self,
        tx: &mut MySqlTx,
        member: &MemberRow,
        joined_at: DateTime<Utc>,
    ) -> Result<(), AppError> {
        let status = match member.status.as_str() {
            "active" | "invited" | "deactivated" => member.status.as_str(),
            _ => "deactivated",
        };
        sqlx::query(
            "INSERT INTO users (id, email, password_hash, display_name, status, locale, role_id, joined_at, email_verified_at) \
             SELECT ?, ?, ?, ?, ?, ?, r.id, ?, IF(? = 'invited', NULL, ?) FROM roles r \
             WHERE r.`key` = COALESCE((SELECT k.`key` FROM roles k WHERE k.`key` = ?), ?)",
        )
        .bind(&member.id)
        .bind(member.email.trim().to_lowercase())
        .bind(member.password_hash.as_deref())
        .bind(&member.display_name)
        .bind(status)
        .bind(member.locale.as_deref())
        .bind(mapper::naive(joined_at))
        .bind(status)
        .bind(mapper::naive(joined_at))
        .bind(&member.role)
        .bind(RoleKey::MEMBER)
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub(crate) async fn members_by_email(&self) -> Result<HashMap<String, UserId>, AppError> {
        let rows = sqlx::query("SELECT id, email FROM users")
            .fetch_all(&self.pool)
            .await?;
        rows.iter()
            .map(|row| {
                let email: String = row.try_get("email")?;
                let id: String = row.try_get("id")?;
                Ok((email.to_lowercase(), UserId::new(id)?))
            })
            .collect()
    }

    pub(crate) async fn insert(
        &self,
        tx: &mut MySqlTx,
        table: &str,
        columns: &[ColumnInfo],
        row: &DataRow,
    ) -> Result<(), AppError> {
        self.write(tx, table, columns, row, false).await
    }

    pub(crate) async fn upsert(
        &self,
        tx: &mut MySqlTx,
        table: &str,
        columns: &[ColumnInfo],
        row: &DataRow,
    ) -> Result<(), AppError> {
        self.write(tx, table, columns, row, true).await
    }

    async fn write(
        &self,
        tx: &mut MySqlTx,
        table: &str,
        columns: &[ColumnInfo],
        row: &DataRow,
        upsert: bool,
    ) -> Result<(), AppError> {
        if !is_sql_identifier(table) {
            return Err(AppError::Internal(format!("invalid table name {table:?}")));
        }
        let selected: Vec<(&ColumnInfo, &Value)> = columns
            .iter()
            .filter(|column| !column.generated && is_sql_identifier(&column.name))
            .filter_map(|column| row.get(&column.name).map(|value| (column, value)))
            .collect();
        let names: Vec<String> = selected
            .iter()
            .map(|(column, _)| quoted(&column.name))
            .collect();
        let mut builder = QueryBuilder::new(format!(
            "INSERT INTO {} ({}) VALUES (",
            quoted(table),
            names.join(", ")
        ));
        let mut first = true;
        for (column, value) in selected {
            if !first {
                builder.push(", ");
            }
            first = false;
            let bound = bind_value(column, value);
            if column.kind == ColumnKind::Json && bound.is_some() {
                builder.push("CAST(").push_bind(bound).push(" AS JSON)");
            } else {
                builder.push_bind(bound);
            }
        }
        builder.push(")");
        if upsert {
            let updates: Vec<String> = names
                .iter()
                .map(|name| format!("{name} = VALUES({name})"))
                .collect();
            builder.push(format!(" ON DUPLICATE KEY UPDATE {}", updates.join(", ")));
        }
        builder.build().execute(&mut **tx).await?;
        Ok(())
    }
}
