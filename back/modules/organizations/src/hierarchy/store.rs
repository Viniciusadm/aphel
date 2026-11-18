use aphel_contract_crm::CrmRecordTypes;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use sqlx::mysql::MySqlRow;
use sqlx::{MySql, MySqlPool, QueryBuilder, Row};
use uuid::Uuid;

use crate::hierarchy::HierarchyEntry;
use aphel_kernel::Ctx;
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::application::fields::RecordTypeRegistry;
use aphel_sdk::platform::application::shared::RecordType;
use aphel_sdk::platform::database::transaction::Tx;
use aphel_sdk::platform::domain::access::{AccessScope, RecordScopeFilter};
use aphel_sdk::platform::infrastructure::fields::mysql_definitions::uuid_column;
use aphel_sdk::platform::infrastructure::query::compiler::{
    MAX_GROUP_RECURSION, push_record_scope,
};
use aphel_sdk::platform::infrastructure::repositories::mapper::{self, optional_uuid, push_ids};

fn lineage_query(company_id: Uuid) -> QueryBuilder<MySql> {
    let mut builder = QueryBuilder::new(
        "WITH RECURSIVE lineage (id, parent_company_id, depth) AS (SELECT c.id, c.parent_company_id, CAST(0 AS SIGNED) \
         FROM organizations c WHERE c.id = ",
    );
    builder
        .push_bind(company_id.to_string())
        .push(
            " AND c.deleted_at IS NULL UNION ALL SELECT c.id, c.parent_company_id, l.depth + 1 \
             FROM organizations c JOIN lineage l ON c.id = l.parent_company_id",
        )
        .push(format!(
            " WHERE c.deleted_at IS NULL AND l.depth < {MAX_GROUP_RECURSION}) \
             SELECT id FROM lineage ORDER BY depth"
        ));
    builder
}

fn subtree_query(company_id: Uuid) -> QueryBuilder<MySql> {
    let mut builder = QueryBuilder::new(
        "WITH RECURSIVE subtree (id, parent_company_id, depth) AS (SELECT c.id, c.parent_company_id, CAST(0 AS SIGNED) \
         FROM organizations c WHERE c.id = ",
    );
    builder
        .push_bind(company_id.to_string())
        .push(
            " AND c.deleted_at IS NULL UNION ALL SELECT c.id, c.parent_company_id, s.depth + 1 \
             FROM organizations c JOIN subtree s ON c.parent_company_id = s.id",
        )
        .push(format!(
            " WHERE c.deleted_at IS NULL AND s.depth < {MAX_GROUP_RECURSION}) \
             SELECT id, parent_company_id, depth FROM subtree ORDER BY depth, id"
        ));
    builder
}

fn ids_from(rows: &[MySqlRow]) -> Result<Vec<Uuid>, AppError> {
    rows.iter().map(|row| uuid_column(row, "id")).collect()
}

fn entries_from(rows: &[MySqlRow]) -> Result<Vec<HierarchyEntry>, AppError> {
    let mut seen = std::collections::HashSet::new();
    let mut entries = Vec::with_capacity(rows.len());
    for row in rows {
        let id = uuid_column(row, "id")?;
        if !seen.insert(id) {
            continue;
        }
        let depth: i64 = row.try_get("depth")?;
        entries.push(HierarchyEntry {
            id,
            parent_id: optional_uuid(row, "parent_company_id")?,
            depth: u32::try_from(depth).unwrap_or(0),
        });
    }
    Ok(entries)
}

#[derive(Clone)]
pub struct HierarchyStore {
    pool: MySqlPool,
    registry: Arc<RecordTypeRegistry>,
}

impl HierarchyStore {
    pub(crate) fn new(pool: MySqlPool, registry: Arc<RecordTypeRegistry>) -> Self {
        Self { pool, registry }
    }

    pub fn of(ctx: &Ctx<'_>) -> Self {
        Self::new(ctx.pool().clone(), ctx.registry().clone())
    }

    pub(crate) async fn subtree_in(
        &self,
        tx: &mut Tx,
        company_id: Uuid,
    ) -> Result<Vec<HierarchyEntry>, AppError> {
        let rows = subtree_query(company_id)
            .build()
            .fetch_all(&mut **tx)
            .await?;
        entries_from(&rows)
    }

    pub(crate) async fn children(
        &self,
        tx: &mut Tx,
        company_id: Uuid,
    ) -> Result<Vec<Uuid>, AppError> {
        let rows = sqlx::query(
            "SELECT id FROM organizations WHERE parent_company_id = ? \
             AND deleted_at IS NULL ORDER BY id FOR UPDATE",
        )
        .bind(company_id.to_string())
        .fetch_all(&mut **tx)
        .await?;
        ids_from(&rows)
    }

    async fn locked_parent(
        tx: &mut Tx,
        company_id: Uuid,
    ) -> Result<Option<Option<Uuid>>, AppError> {
        let row = sqlx::query(
            "SELECT parent_company_id FROM organizations WHERE id = ? AND deleted_at IS NULL FOR UPDATE",
        )
        .bind(company_id.to_string())
        .fetch_optional(&mut **tx)
        .await?;
        row.map(|row| optional_uuid(&row, "parent_company_id"))
            .transpose()
    }

    pub(crate) async fn parent_of(
        &self,
        tx: &mut Tx,
        company_id: Uuid,
    ) -> Result<Option<Option<Uuid>>, AppError> {
        Self::locked_parent(tx, company_id).await
    }

    pub(crate) async fn lock_company(
        &self,
        tx: &mut Tx,
        company_id: Uuid,
    ) -> Result<bool, AppError> {
        Ok(Self::locked_parent(tx, company_id).await?.is_some())
    }

    pub(crate) async fn locked_lineage(
        &self,
        tx: &mut Tx,
        company_id: Uuid,
    ) -> Result<Vec<Uuid>, AppError> {
        let mut lineage = Vec::new();
        let mut cursor = Some(company_id);
        while let Some(current) = cursor {
            if lineage.contains(&current) || lineage.len() > MAX_GROUP_RECURSION as usize {
                break;
            }
            let Some(parent) = Self::locked_parent(tx, current).await? else {
                break;
            };
            lineage.push(current);
            cursor = parent;
        }
        Ok(lineage)
    }

    pub(crate) async fn locked_height(
        &self,
        tx: &mut Tx,
        company_id: Uuid,
    ) -> Result<usize, AppError> {
        let mut seen = std::collections::HashSet::from([company_id]);
        let mut level = vec![company_id];
        let mut height = 0;
        while height <= MAX_GROUP_RECURSION as usize {
            let mut builder = QueryBuilder::new(
                "SELECT id FROM organizations WHERE deleted_at IS NULL AND parent_company_id",
            );
            push_ids(&mut builder, &level);
            builder.push(" FOR SHARE");
            let rows = builder.build().fetch_all(&mut **tx).await?;
            let next: Vec<Uuid> = ids_from(&rows)?
                .into_iter()
                .filter(|id| seen.insert(*id))
                .collect();
            if next.is_empty() {
                break;
            }
            height += 1;
            level = next;
        }
        Ok(height)
    }

    pub(crate) async fn mark_detached(
        &self,
        tx: &mut Tx,
        company_ids: &[Uuid],
        detached_from: Uuid,
    ) -> Result<(), AppError> {
        if company_ids.is_empty() {
            return Ok(());
        }
        let mut builder = QueryBuilder::new("UPDATE organizations SET detached_from_company_id = ");
        builder
            .push_bind(detached_from.to_string())
            .push(" WHERE id");
        push_ids(&mut builder, company_ids);
        builder.build().execute(&mut **tx).await?;
        Ok(())
    }

    pub(crate) async fn detached_children(
        &self,
        tx: &mut Tx,
        detached_from: Uuid,
        current_parent: Option<Uuid>,
    ) -> Result<Vec<Uuid>, AppError> {
        let mut builder = QueryBuilder::new(
            "SELECT id FROM organizations WHERE deleted_at IS NULL AND detached_from_company_id = ",
        );
        builder.push_bind(detached_from.to_string());
        match current_parent {
            Some(parent) => {
                builder
                    .push(" AND parent_company_id = ")
                    .push_bind(parent.to_string());
            }
            None => {
                builder.push(" AND parent_company_id IS NULL");
            }
        }
        builder.push(" ORDER BY id FOR UPDATE");
        let rows = builder.build().fetch_all(&mut **tx).await?;
        ids_from(&rows)
    }

    pub(crate) async fn clear_detached(
        &self,
        tx: &mut Tx,
        detached_from: Uuid,
    ) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE organizations SET detached_from_company_id = NULL WHERE detached_from_company_id = ?",
        )
        .bind(detached_from.to_string())
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub(crate) async fn set_parent(
        &self,
        tx: &mut Tx,
        company_ids: &[Uuid],
        parent_id: Option<Uuid>,
        updated_at: DateTime<Utc>,
    ) -> Result<(), AppError> {
        if company_ids.is_empty() {
            return Ok(());
        }
        let mut builder = QueryBuilder::new("UPDATE organizations SET parent_company_id = ");
        builder
            .push_bind(parent_id.map(|id| id.to_string()))
            .push(", updated_at = ")
            .push_bind(mapper::naive(updated_at))
            .push(" WHERE deleted_at IS NULL AND id");
        push_ids(&mut builder, company_ids);
        builder.build().execute(&mut **tx).await?;
        Ok(())
    }

    pub(crate) async fn lineage(&self, company_id: Uuid) -> Result<Vec<Uuid>, AppError> {
        let rows = lineage_query(company_id)
            .build()
            .fetch_all(&self.pool)
            .await?;
        ids_from(&rows)
    }

    pub(crate) async fn subtree(&self, company_id: Uuid) -> Result<Vec<HierarchyEntry>, AppError> {
        let rows = subtree_query(company_id)
            .build()
            .fetch_all(&self.pool)
            .await?;
        entries_from(&rows)
    }

    pub(crate) async fn visible_ids(
        &self,
        scope: &AccessScope,
        visibility: &RecordScopeFilter,
        company_ids: &[Uuid],
    ) -> Result<Vec<Uuid>, AppError> {
        if company_ids.is_empty() {
            return Ok(Vec::new());
        }
        let definition = self.registry.require(RecordType::COMPANY.as_str())?;
        let mut builder = QueryBuilder::new(
            "SELECT r.id FROM organizations r WHERE r.deleted_at IS NULL AND r.id",
        );
        push_ids(&mut builder, company_ids);
        push_record_scope(&mut builder, scope, definition, visibility);
        let rows = builder.build().fetch_all(&self.pool).await?;
        ids_from(&rows)
    }

    pub(crate) async fn base_currency(&self) -> Result<Option<String>, AppError> {
        Ok(
            sqlx::query_scalar("SELECT currency FROM organization_settings WHERE id = 1")
                .fetch_optional(&self.pool)
                .await?,
        )
    }
}
