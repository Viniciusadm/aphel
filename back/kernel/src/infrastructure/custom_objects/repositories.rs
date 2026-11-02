use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::mysql::MySqlRow;
use sqlx::{MySql, MySqlPool, QueryBuilder, Row};
use uuid::Uuid;

use crate::application::custom_objects::{
    CustomObjectRepository, CustomRecordRepository, RecordLink, RecordLinkRepository,
    RelationshipRepository, record_has_relations,
};
use crate::application::fields::{EntityDefinition, RecordTypeRegistry};
use crate::application::organization_admin::{ListWindow, Listing};
use crate::application::shared::RecordRef;
use crate::database::transaction::MySqlTx;
use crate::domain::access::{AccessScope, RecordScope, RecordScopeFilter};
use crate::domain::custom_objects::{CustomObject, CustomRecord, OnDelete, RelationshipDefinition};
use crate::errors::AppError;
use crate::infrastructure::custom_objects::rows::{
    OBJECT_COLUMNS, RELATIONSHIP_COLUMNS, object_from_row, relationship_from_row,
};
use crate::infrastructure::fields::mysql_definitions::{seed_system_fields, uuid_column};
use crate::infrastructure::query::compiler::{push_live_record, quoted, reference_set_storage};
use crate::infrastructure::repositories::mapper;
use crate::infrastructure::repositories::mapper::{
    instant, optional_instant, optional_user, push_ids,
};

const RECORD_TYPE_TABLES: [&str; 5] = [
    "custom_field_multi_values",
    "custom_field_values",
    "field_permissions",
    "field_permission_versions",
    "field_definitions",
];

#[derive(Debug, Clone, Copy)]
pub struct RecordTypeScopedTable(pub &'static str);

fn record_type_tables() -> Vec<&'static str> {
    let mut tables: Vec<&'static str> = Vec::new();
    if let Some(app) = crate::modules::try_app() {
        for (_, table) in app.all_contributions::<RecordTypeScopedTable>() {
            if !tables.contains(&table.0) {
                tables.push(table.0);
            }
        }
    }
    if let Some(storage) = reference_set_storage()
        && !tables.contains(&storage.links_table)
    {
        tables.push(storage.links_table);
    }
    for table in RECORD_TYPE_TABLES {
        if !tables.contains(&table) {
            tables.push(table);
        }
    }
    tables
}

#[derive(Clone)]
pub struct MySqlCustomObjectRepository {
    pool: MySqlPool,
}

impl MySqlCustomObjectRepository {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl CustomObjectRepository for MySqlCustomObjectRepository {
    type Tx = MySqlTx;

    async fn list(&self) -> Result<Vec<CustomObject>, AppError> {
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "SELECT {OBJECT_COLUMNS} FROM custom_objects o \
             ORDER BY o.position, o.label_plural, o.id"
        )))
        .fetch_all(&self.pool)
        .await?
        .iter()
        .map(object_from_row)
        .collect()
    }

    async fn find_by_key(&self, key: &str) -> Result<Option<CustomObject>, AppError> {
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "SELECT {OBJECT_COLUMNS} FROM custom_objects o \
             WHERE o.`key` = ?"
        )))
        .bind(key)
        .fetch_optional(&self.pool)
        .await?
        .as_ref()
        .map(object_from_row)
        .transpose()
    }

    async fn record_counts(&self) -> Result<HashMap<Uuid, u64>, AppError> {
        let rows = sqlx::query(
            "SELECT object_id, COUNT(*) AS total FROM custom_records \
             WHERE deleted_at IS NULL GROUP BY object_id",
        )
        .fetch_all(&self.pool)
        .await?;
        rows.iter()
            .map(|row| {
                let total: i64 = row.try_get("total")?;
                Ok((uuid_column(row, "object_id")?, total.max(0) as u64))
            })
            .collect()
    }

    async fn insert(&self, tx: &mut Self::Tx, object: &CustomObject) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO custom_objects (id, `key`, label_singular, label_plural, \
             description, icon, color, title_field_key, owner_enabled, is_active, position, \
             module_key, entity_definition, is_internal, created_by, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(object.id.to_string())
        .bind(&object.key)
        .bind(&object.data.label_singular)
        .bind(&object.data.label_plural)
        .bind(&object.data.description)
        .bind(&object.data.icon)
        .bind(&object.data.color)
        .bind(&object.data.title_field_key)
        .bind(object.data.owner_enabled)
        .bind(object.data.is_active)
        .bind(object.position)
        .bind(&object.module_key)
        .bind(object.entity_definition.as_ref().map(sqlx::types::Json))
        .bind(object.is_internal)
        .bind(
            object
                .created_by
                .as_ref()
                .map(|user| user.as_str().to_owned()),
        )
        .bind(mapper::naive(object.created_at))
        .bind(mapper::naive(object.updated_at))
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    async fn update(&self, tx: &mut Self::Tx, object: &CustomObject) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE custom_objects SET label_singular = ?, label_plural = ?, description = ?, \
             icon = ?, color = ?, title_field_key = ?, owner_enabled = ?, is_active = ?, \
             updated_at = ? WHERE id = ?",
        )
        .bind(&object.data.label_singular)
        .bind(&object.data.label_plural)
        .bind(&object.data.description)
        .bind(&object.data.icon)
        .bind(&object.data.color)
        .bind(&object.data.title_field_key)
        .bind(object.data.owner_enabled)
        .bind(object.data.is_active)
        .bind(mapper::naive(object.updated_at))
        .bind(object.id.to_string())
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    async fn delete(&self, tx: &mut Self::Tx, object: &CustomObject) -> Result<(), AppError> {
        let record_type = object.record_type();
        for table in record_type_tables() {
            sqlx::query(sqlx::AssertSqlSafe(format!(
                "DELETE FROM {} WHERE record_type = ?",
                quoted(table)
            )))
            .bind(&record_type)
            .execute(&mut **tx)
            .await?;
        }
        sqlx::query(
            "DELETE rp FROM role_permissions rp JOIN roles r ON r.id = rp.role_id \
             WHERE rp.permission LIKE ?",
        )
        .bind(format!("{record_type}.%"))
        .execute(&mut **tx)
        .await?;
        sqlx::query("DELETE FROM custom_objects WHERE id = ?")
            .bind(object.id.to_string())
            .execute(&mut **tx)
            .await?;
        Ok(())
    }

    async fn reorder(&self, tx: &mut Self::Tx, ordered_ids: &[Uuid]) -> Result<(), AppError> {
        for (position, id) in ordered_ids.iter().enumerate() {
            sqlx::query("UPDATE custom_objects SET position = ? WHERE id = ?")
                .bind(position as i32)
                .bind(id.to_string())
                .execute(&mut **tx)
                .await?;
        }
        Ok(())
    }

    async fn refresh_titles(
        &self,
        tx: &mut Self::Tx,
        object: &CustomObject,
        field_id: Uuid,
    ) -> Result<u64, AppError> {
        let result = sqlx::query(
            "UPDATE custom_records r LEFT JOIN custom_field_values v \
             ON v.record_type = ? AND v.record_id = r.id AND v.field_id = ? \
             SET r.title = LEFT(COALESCE(v.value_text, ''), 255) \
             WHERE r.object_id = ?",
        )
        .bind(object.record_type())
        .bind(field_id.to_string())
        .bind(object.id.to_string())
        .execute(&mut **tx)
        .await?;
        Ok(result.rows_affected())
    }

    async fn bump_schema_version(&self, tx: &mut Self::Tx) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE organization_settings SET custom_schema_version = custom_schema_version + 1 \
             WHERE id = 1",
        )
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    async fn grant_default_permissions(
        &self,
        tx: &mut Self::Tx,
        record_type: &str,
        scope: Option<RecordScope>,
    ) -> Result<(), AppError> {
        sqlx::query(
            "INSERT IGNORE INTO role_permissions (role_id, permission, scope) \
             SELECT r.id, CONCAT(?, '.', p.action), COALESCE(?, IF(r.`key` = 'manager', 'team', 'own')) \
             FROM roles r JOIN ( \
               SELECT 'read' AS action UNION ALL SELECT 'create' UNION ALL SELECT 'update' \
               UNION ALL SELECT 'delete' UNION ALL SELECT 'archive' UNION ALL SELECT 'export' \
             ) p \
             WHERE r.is_system = TRUE \
             AND (r.`key` = 'manager' OR (r.`key` = 'member' AND p.action <> 'export'))",
        )
        .bind(record_type)
        .bind(scope.map(|scope| scope.as_str()))
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    async fn seed_system_fields(
        &self,
        tx: &mut Self::Tx,
        definition: &EntityDefinition,
    ) -> Result<(), AppError> {
        let registry = RecordTypeRegistry::empty().register(definition.clone())?;
        seed_system_fields(tx, &registry).await
    }
}

#[derive(Clone)]
pub struct MySqlRelationshipRepository {
    pool: MySqlPool,
}

impl MySqlRelationshipRepository {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl RelationshipRepository for MySqlRelationshipRepository {
    type Tx = MySqlTx;

    async fn list(&self) -> Result<Vec<RelationshipDefinition>, AppError> {
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "SELECT {RELATIONSHIP_COLUMNS} FROM relationship_definitions d \
             ORDER BY d.created_at, d.id"
        )))
        .fetch_all(&self.pool)
        .await?
        .iter()
        .map(relationship_from_row)
        .collect()
    }

    async fn link_counts(&self) -> Result<HashMap<Uuid, u64>, AppError> {
        let rows = sqlx::query(
            "SELECT relationship_id, COUNT(*) AS total FROM record_relations \
             GROUP BY relationship_id",
        )
        .fetch_all(&self.pool)
        .await?;
        rows.iter()
            .map(|row| {
                let total: i64 = row.try_get("total")?;
                Ok((uuid_column(row, "relationship_id")?, total.max(0) as u64))
            })
            .collect()
    }

    async fn insert(
        &self,
        tx: &mut Self::Tx,
        relationship: &RelationshipDefinition,
    ) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO relationship_definitions (id, `key`, inverse_key, \
             from_type, to_type, cardinality, from_label, to_label, is_required, on_delete, \
             created_by, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(relationship.id.to_string())
        .bind(&relationship.key)
        .bind(&relationship.inverse_key)
        .bind(&relationship.from_type)
        .bind(&relationship.to_type)
        .bind(relationship.cardinality.as_str())
        .bind(&relationship.from_label)
        .bind(&relationship.to_label)
        .bind(relationship.required)
        .bind(relationship.on_delete.as_str())
        .bind(
            relationship
                .created_by
                .as_ref()
                .map(|user| user.as_str().to_owned()),
        )
        .bind(mapper::naive(relationship.created_at))
        .bind(mapper::naive(relationship.updated_at))
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    async fn update(
        &self,
        tx: &mut Self::Tx,
        relationship: &RelationshipDefinition,
    ) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE relationship_definitions SET from_label = ?, to_label = ?, is_required = ?, \
             on_delete = ?, updated_at = ? WHERE id = ?",
        )
        .bind(&relationship.from_label)
        .bind(&relationship.to_label)
        .bind(relationship.required)
        .bind(relationship.on_delete.as_str())
        .bind(mapper::naive(relationship.updated_at))
        .bind(relationship.id.to_string())
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    async fn delete(&self, tx: &mut Self::Tx, relationship_id: Uuid) -> Result<u64, AppError> {
        let removed = sqlx::query("DELETE FROM record_relations WHERE relationship_id = ?")
            .bind(relationship_id.to_string())
            .execute(&mut **tx)
            .await?
            .rows_affected();
        sqlx::query("DELETE FROM relationship_definitions WHERE id = ?")
            .bind(relationship_id.to_string())
            .execute(&mut **tx)
            .await?;
        Ok(removed)
    }
}

#[derive(Clone)]
pub struct MySqlRecordLinkRepository {
    pool: MySqlPool,
    registry: Arc<RecordTypeRegistry>,
}

impl MySqlRecordLinkRepository {
    pub fn new(pool: MySqlPool, registry: Arc<RecordTypeRegistry>) -> Self {
        Self { pool, registry }
    }

    fn table_of(&self, record_type: &str) -> Option<String> {
        self.registry
            .require(record_type)
            .ok()
            .map(|definition| definition.table.clone())
    }
}

fn link_columns(inverse: bool) -> (&'static str, &'static str) {
    if inverse {
        ("to_id", "from_id")
    } else {
        ("from_id", "to_id")
    }
}

#[async_trait]
impl RecordLinkRepository for MySqlRecordLinkRepository {
    type Tx = MySqlTx;

    async fn linked_ids(
        &self,
        scope: &AccessScope,
        target: &EntityDefinition,
        visibility: &RecordScopeFilter,
        relationship_id: Uuid,
        inverse: bool,
        record_id: Uuid,
        window: ListWindow,
    ) -> Result<Listing<Uuid>, AppError> {
        let (source, linked) = link_columns(inverse);
        let push_filter = |builder: &mut QueryBuilder<MySql>| {
            builder
                .push(format!(
                    " FROM record_relations l JOIN {} t ON t.id = l.{linked} WHERE l.relationship_id = ",
                    quoted(&target.table)
                ))
                .push_bind(relationship_id.to_string())
                .push(format!(" AND l.{source} = "))
                .push_bind(record_id.to_string())
                .push(" AND ");
            push_live_record(builder, scope, target, visibility, "t");
        };
        let mut count = QueryBuilder::<MySql>::new("SELECT COUNT(*)");
        push_filter(&mut count);
        let total: i64 = count.build_query_scalar().fetch_one(&self.pool).await?;
        let mut page = QueryBuilder::<MySql>::new("SELECT t.id");
        push_filter(&mut page);
        page.push(" ORDER BY l.created_at DESC, t.id DESC LIMIT ")
            .push_bind(window.limit)
            .push(" OFFSET ")
            .push_bind(window.offset);
        let items = page
            .build()
            .fetch_all(&self.pool)
            .await?
            .iter()
            .map(|row| uuid_column(row, "id"))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Listing {
            items,
            total: total.max(0) as u64,
        })
    }

    async fn count_links(
        &self,
        relationship_id: Uuid,
        inverse: bool,
        record_id: Uuid,
    ) -> Result<u64, AppError> {
        let (source, _) = link_columns(inverse);
        let total: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
            "SELECT COUNT(*) FROM record_relations \
             WHERE relationship_id = ? AND {source} = ?"
        )))
        .bind(relationship_id.to_string())
        .bind(record_id.to_string())
        .fetch_one(&self.pool)
        .await?;
        Ok(total.max(0) as u64)
    }

    async fn lock_links(
        &self,
        tx: &mut Self::Tx,
        relationship_id: Uuid,
        inverse: bool,
        record_id: Uuid,
    ) -> Result<u64, AppError> {
        let (source, _) = link_columns(inverse);
        let rows = sqlx::query(sqlx::AssertSqlSafe(format!(
            "SELECT from_id FROM record_relations \
             WHERE relationship_id = ? AND {source} = ? FOR UPDATE"
        )))
        .bind(relationship_id.to_string())
        .bind(record_id.to_string())
        .fetch_all(&mut **tx)
        .await?;
        Ok(rows.len() as u64)
    }

    async fn insert(&self, tx: &mut Self::Tx, link: &RecordLink) -> Result<bool, AppError> {
        let result = sqlx::query(
            "INSERT IGNORE INTO record_relations \
             (relationship_id, from_id, to_id, created_by, created_at) \
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(link.relationship_id.to_string())
        .bind(link.from_id.to_string())
        .bind(link.to_id.to_string())
        .bind(
            link.created_by
                .as_ref()
                .map(|user| user.as_str().to_owned()),
        )
        .bind(mapper::naive(link.created_at))
        .execute(&mut **tx)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn delete(
        &self,
        tx: &mut Self::Tx,
        relationship_id: Uuid,
        from_id: Uuid,
        to_id: Uuid,
    ) -> Result<bool, AppError> {
        let result = sqlx::query(
            "DELETE FROM record_relations WHERE relationship_id = ? \
             AND from_id = ? AND to_id = ?",
        )
        .bind(relationship_id.to_string())
        .bind(from_id.to_string())
        .bind(to_id.to_string())
        .execute(&mut **tx)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn delete_record_links(
        &self,
        tx: &mut Self::Tx,
        record_id: Uuid,
    ) -> Result<u64, AppError> {
        let result = sqlx::query("DELETE FROM record_relations WHERE (from_id = ? OR to_id = ?)")
            .bind(record_id.to_string())
            .bind(record_id.to_string())
            .execute(&mut **tx)
            .await?;
        Ok(result.rows_affected())
    }

    async fn ensure_deletable(
        &self,
        tx: &mut Self::Tx,
        record: &RecordRef,
    ) -> Result<(), AppError> {
        let relationships = sqlx::query(sqlx::AssertSqlSafe(format!(
            "SELECT {RELATIONSHIP_COLUMNS} FROM relationship_definitions d \
             WHERE d.from_type = ? AND d.on_delete = ?"
        )))
        .bind(record.record_type.as_str())
        .bind(OnDelete::Restrict.as_str())
        .fetch_all(&mut **tx)
        .await?
        .iter()
        .map(relationship_from_row)
        .collect::<Result<Vec<_>, _>>()?;
        let mut blocking = Vec::new();
        for relationship in relationships {
            let Some(table) = self.table_of(&relationship.to_type) else {
                continue;
            };
            let total: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
                "SELECT COUNT(*) FROM record_relations l JOIN {} t ON t.id = l.to_id \
                 AND t.deleted_at IS NULL \
                 WHERE l.relationship_id = ? AND l.from_id = ?",
                quoted(&table)
            )))
            .bind(relationship.id.to_string())
            .bind(record.record_id.to_string())
            .fetch_one(&mut **tx)
            .await?;
            if total > 0 {
                blocking.push(relationship.from_label.clone());
            }
        }
        if blocking.is_empty() {
            Ok(())
        } else {
            Err(record_has_relations(&blocking))
        }
    }

    async fn titles(
        &self,
        definition: &EntityDefinition,
        record_ids: &[Uuid],
    ) -> Result<HashMap<Uuid, String>, AppError> {
        let Some(column) = definition.title_column() else {
            return Ok(HashMap::new());
        };
        if record_ids.is_empty() {
            return Ok(HashMap::new());
        }
        let mut builder = QueryBuilder::<MySql>::new(format!(
            "SELECT r.id, r.{} AS title FROM {} r WHERE r.id",
            quoted(column),
            quoted(&definition.table)
        ));
        push_ids(&mut builder, record_ids);
        let rows = builder.build().fetch_all(&self.pool).await?;
        rows.iter()
            .map(|row| {
                let title: Option<String> = row.try_get("title")?;
                Ok((uuid_column(row, "id")?, title.unwrap_or_default()))
            })
            .collect()
    }
}

const RECORD_SELECT: &str = "SELECT r.id, r.object_id, r.owner_id, r.title, r.created_by, \
     r.archived_at, r.created_at, r.updated_at FROM custom_records r \
     WHERE r.deleted_at IS NULL";

fn record_from_row(row: &MySqlRow) -> Result<CustomRecord, AppError> {
    Ok(CustomRecord {
        id: uuid_column(row, "id")?,
        object_id: uuid_column(row, "object_id")?,
        owner_id: optional_user(row, "owner_id")?,
        title: row.try_get("title")?,
        created_by: optional_user(row, "created_by")?,
        archived_at: optional_instant(row, "archived_at")?,
        created_at: instant(row, "created_at")?,
        updated_at: instant(row, "updated_at")?,
    })
}

#[derive(Clone)]
pub struct MySqlCustomRecordRepository {
    pool: MySqlPool,
}

impl MySqlCustomRecordRepository {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    fn select(object_id: Uuid) -> QueryBuilder<MySql> {
        let mut builder = QueryBuilder::new(RECORD_SELECT);
        builder
            .push(" AND r.object_id = ")
            .push_bind(object_id.to_string());
        builder
    }
}

#[async_trait]
impl CustomRecordRepository for MySqlCustomRecordRepository {
    type Tx = MySqlTx;

    async fn find(
        &self,
        object_id: Uuid,
        record_id: Uuid,
    ) -> Result<Option<CustomRecord>, AppError> {
        let mut builder = Self::select(object_id);
        builder
            .push(" AND r.id = ")
            .push_bind(record_id.to_string());
        builder
            .build()
            .fetch_optional(&self.pool)
            .await?
            .as_ref()
            .map(record_from_row)
            .transpose()
    }

    async fn find_many(
        &self,
        object_id: Uuid,
        record_ids: &[Uuid],
    ) -> Result<Vec<CustomRecord>, AppError> {
        if record_ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut builder = Self::select(object_id);
        builder.push(" AND r.id");
        push_ids(&mut builder, record_ids);
        builder
            .build()
            .fetch_all(&self.pool)
            .await?
            .iter()
            .map(record_from_row)
            .collect()
    }

    async fn insert(&self, tx: &mut Self::Tx, record: &CustomRecord) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO custom_records (id, object_id, owner_id, title, \
             created_by, archived_at, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(record.id.to_string())
        .bind(record.object_id.to_string())
        .bind(
            record
                .owner_id
                .as_ref()
                .map(|user| user.as_str().to_owned()),
        )
        .bind(&record.title)
        .bind(
            record
                .created_by
                .as_ref()
                .map(|user| user.as_str().to_owned()),
        )
        .bind(record.archived_at.map(mapper::naive))
        .bind(mapper::naive(record.created_at))
        .bind(mapper::naive(record.updated_at))
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    async fn update(&self, tx: &mut Self::Tx, record: &CustomRecord) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE custom_records SET owner_id = ?, title = ?, updated_at = ? \
             WHERE object_id = ? AND id = ?",
        )
        .bind(
            record
                .owner_id
                .as_ref()
                .map(|user| user.as_str().to_owned()),
        )
        .bind(&record.title)
        .bind(mapper::naive(record.updated_at))
        .bind(record.object_id.to_string())
        .bind(record.id.to_string())
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    async fn set_archived(
        &self,
        tx: &mut Self::Tx,
        record_id: Uuid,
        archived_at: Option<DateTime<Utc>>,
        updated_at: DateTime<Utc>,
    ) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE custom_records SET archived_at = ?, updated_at = ? \
             WHERE id = ?",
        )
        .bind(archived_at.map(mapper::naive))
        .bind(mapper::naive(updated_at))
        .bind(record_id.to_string())
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    async fn soft_delete(
        &self,
        tx: &mut Self::Tx,
        record_id: Uuid,
        deleted_at: DateTime<Utc>,
    ) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE custom_records SET deleted_at = ?, updated_at = ? \
             WHERE id = ? AND deleted_at IS NULL",
        )
        .bind(mapper::naive(deleted_at))
        .bind(mapper::naive(deleted_at))
        .bind(record_id.to_string())
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    async fn lock(&self, tx: &mut Self::Tx, record_id: Uuid) -> Result<(), AppError> {
        sqlx::query("SELECT id FROM custom_records WHERE id = ? FOR UPDATE")
            .bind(record_id.to_string())
            .fetch_optional(&mut **tx)
            .await?;
        Ok(())
    }
}
