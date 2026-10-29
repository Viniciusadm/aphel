use std::sync::Arc;

use chrono::{DateTime, Utc};
use sqlx::mysql::MySqlRow;
use sqlx::{MySql, MySqlPool, QueryBuilder, Row};
use uuid::Uuid;

use crate::domain::TrashItem;
use aphel_kernel::Ctx;
use aphel_kernel::application::fields::RecordTypeDefinition;
use aphel_kernel::application::organization_admin::{ListWindow, Listing};
use aphel_kernel::application::shared::RecordType;
use aphel_kernel::database::transaction::MySqlTx;
use aphel_kernel::domain::access::{AccessScope, RecordScopeFilter};
use aphel_kernel::errors::AppError;
use aphel_kernel::infrastructure::fields::mysql_definitions::uuid_column;
use aphel_kernel::infrastructure::query::compiler::{
    aliased, push_partition, push_record_scope_as,
};
use aphel_kernel::infrastructure::repositories::mapper;
use aphel_kernel::infrastructure::repositories::mapper::{instant, owner_reference};

pub trait TrashExclusion: Send + Sync {
    fn push_hidden(
        &self,
        builder: &mut QueryBuilder<MySql>,
        definition: &RecordTypeDefinition,
        alias: &str,
    );
}

const NULL_TEXT: &str = "CAST(NULL AS CHAR CHARACTER SET utf8mb4) COLLATE utf8mb4_unicode_ci";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TrashHidden {
    Excluded,
    Included,
}

#[derive(Debug, Clone)]
pub struct TrashSource {
    pub(crate) definition: RecordTypeDefinition,
    pub(crate) subtitle_columns: Vec<String>,
    pub(crate) visibility: RecordScopeFilter,
}

impl TrashSource {
    pub(crate) fn new(definition: RecordTypeDefinition) -> Self {
        Self {
            definition,
            subtitle_columns: Vec::new(),
            visibility: RecordScopeFilter::Organization,
        }
    }

    pub(crate) fn with_visibility(mut self, visibility: RecordScopeFilter) -> Self {
        self.visibility = visibility;
        self
    }

    pub(crate) fn with_subtitles(mut self, columns: Vec<String>) -> Self {
        self.subtitle_columns = columns;
        self
    }
}

fn text(expression: String) -> String {
    format!("CAST({expression} AS CHAR CHARACTER SET utf8mb4) COLLATE utf8mb4_unicode_ci")
}

fn deleted_column(definition: &RecordTypeDefinition) -> String {
    aliased(
        "r",
        definition.deleted_column.as_deref().unwrap_or("deleted_at"),
    )
}

fn title_sql(definition: &RecordTypeDefinition) -> String {
    text(aliased("r", definition.title_column().unwrap_or("id")))
}

fn subtitle_sql(columns: &[String]) -> String {
    match columns {
        [] => NULL_TEXT.to_owned(),
        columns => format!(
            "COALESCE({})",
            columns
                .iter()
                .map(|column| text(aliased("r", column)))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

fn escape_like(term: &str) -> String {
    let mut escaped = String::with_capacity(term.len() + 2);
    escaped.push('%');
    for character in term.chars() {
        if matches!(character, '%' | '_' | '\\') {
            escaped.push('\\');
        }
        escaped.push(character);
    }
    escaped.push('%');
    escaped
}

fn push_select(
    builder: &mut QueryBuilder<MySql>,
    scope: &AccessScope,
    source: &TrashSource,
    search: Option<&str>,
    exclusions: &[Arc<dyn TrashExclusion>],
) {
    let definition = &source.definition;
    let kind = definition.record_type.as_str();
    let deleted = deleted_column(definition);
    let (owner, owner_join) = match definition.owner_column.as_deref() {
        Some(column) => {
            let owner = aliased("r", column);
            (
                owner.clone(),
                format!("LEFT JOIN users u ON u.id = {owner}"),
            )
        }
        None => (
            "CAST(NULL AS CHAR)".to_owned(),
            "LEFT JOIN users u ON FALSE".to_owned(),
        ),
    };
    builder
        .push(format!(
            "SELECT _utf8mb4'{kind}' COLLATE utf8mb4_unicode_ci AS record_type, r.id, {title} AS title, {subtitle} AS subtitle, \
             {owner} AS owner_id, u.display_name AS owner_display_name, {deleted} AS deleted_at \
             FROM {table} r {owner_join} \
             WHERE {deleted} IS NOT NULL",
            title = title_sql(definition),
            subtitle = subtitle_sql(&source.subtitle_columns),
            table = definition.table,
        ));
    push_record_scope_as(builder, scope, definition, &source.visibility, "r");
    for exclusion in exclusions {
        exclusion.push_hidden(builder, definition, "r");
    }
    if let Some(term) = search {
        builder
            .push(format!(" AND {} LIKE ", title_sql(definition)))
            .push_bind(escape_like(term));
    }
}

fn item_from_row(row: &MySqlRow) -> Result<Option<TrashItem>, AppError> {
    let record_type: String = row.try_get("record_type")?;
    let Ok(record_type) = RecordType::parse(&record_type) else {
        return Ok(None);
    };
    Ok(Some(TrashItem {
        record_type,
        id: uuid_column(row, "id")?,
        title: row
            .try_get::<Option<String>, _>("title")?
            .unwrap_or_default(),
        subtitle: row.try_get("subtitle")?,
        owner: owner_reference(row)?,
        deleted_at: instant(row, "deleted_at")?,
    }))
}

#[derive(Clone)]
pub struct TrashStore {
    pool: MySqlPool,
    exclusions: Vec<Arc<dyn TrashExclusion>>,
}

impl TrashStore {
    pub fn of(ctx: &Ctx<'_>) -> Self {
        Self {
            pool: ctx.pool().clone(),
            exclusions: ctx.app().contributions::<dyn TrashExclusion>(),
        }
    }

    pub(crate) async fn list(
        &self,
        scope: &AccessScope,
        sources: &[TrashSource],
        search: Option<&str>,
        window: ListWindow,
    ) -> Result<Listing<TrashItem>, AppError> {
        if sources.is_empty() {
            return Ok(Listing {
                items: Vec::new(),
                total: 0,
            });
        }
        let push_union = |builder: &mut QueryBuilder<MySql>| {
            for (index, source) in sources.iter().enumerate() {
                if index > 0 {
                    builder.push(" UNION ALL ");
                }
                builder.push("(");
                push_select(builder, scope, source, search, &self.exclusions);
                builder.push(")");
            }
        };

        let mut count = QueryBuilder::new("SELECT COUNT(*) FROM (");
        push_union(&mut count);
        count.push(") trash");
        let total: i64 = count.build_query_scalar().fetch_one(&self.pool).await?;

        let mut builder = QueryBuilder::new("SELECT * FROM (");
        push_union(&mut builder);
        builder
            .push(") trash ORDER BY deleted_at DESC, id DESC LIMIT ")
            .push_bind(window.limit as i64)
            .push(" OFFSET ")
            .push_bind(window.offset as i64);
        let rows = builder.build().fetch_all(&self.pool).await?;
        let mut items = Vec::with_capacity(rows.len());
        for row in &rows {
            if let Some(item) = item_from_row(row)? {
                items.push(item);
            }
        }
        Ok(Listing {
            items,
            total: total.max(0) as u64,
        })
    }

    pub(crate) async fn find(
        &self,
        scope: &AccessScope,
        source: &TrashSource,
        record_id: Uuid,
        hidden: TrashHidden,
    ) -> Result<Option<TrashItem>, AppError> {
        let exclusions: &[Arc<dyn TrashExclusion>] = match hidden {
            TrashHidden::Excluded => &self.exclusions,
            TrashHidden::Included => &[],
        };
        let mut builder = QueryBuilder::new("");
        push_select(&mut builder, scope, source, None, exclusions);
        builder
            .push(" AND r.id = ")
            .push_bind(record_id.to_string());
        let row = builder.build().fetch_optional(&self.pool).await?;
        match row {
            Some(row) => item_from_row(&row),
            None => Ok(None),
        }
    }

    pub(crate) async fn restore(
        &self,
        tx: &mut MySqlTx,
        definition: &RecordTypeDefinition,
        record_id: Uuid,
    ) -> Result<bool, AppError> {
        let deleted = deleted_column(definition);
        let mut builder = QueryBuilder::<MySql>::new(format!(
            "UPDATE {} r SET {deleted} = NULL WHERE r.id = ",
            definition.table
        ));
        builder
            .push_bind(record_id.to_string())
            .push(format!(" AND {deleted} IS NOT NULL"));
        push_partition(&mut builder, definition, "r");
        let result = builder.build().execute(&mut **tx).await?;
        Ok(result.rows_affected() > 0)
    }

    pub async fn purge(
        &self,
        tx: &mut MySqlTx,
        definition: &RecordTypeDefinition,
        record_id: Uuid,
    ) -> Result<bool, AppError> {
        let deleted = deleted_column(definition);
        let mut builder = QueryBuilder::<MySql>::new(format!(
            "DELETE r FROM {} r WHERE r.id = ",
            definition.table
        ));
        builder
            .push_bind(record_id.to_string())
            .push(format!(" AND {deleted} IS NOT NULL"));
        push_partition(&mut builder, definition, "r");
        let result = builder.build().execute(&mut **tx).await?;
        Ok(result.rows_affected() > 0)
    }

    pub(crate) async fn retention_days(&self) -> Result<Option<u32>, AppError> {
        let days: Option<u32> =
            sqlx::query_scalar("SELECT retention_days FROM trash_settings WHERE id = 1")
                .fetch_optional(&self.pool)
                .await?;
        Ok(days)
    }

    pub(crate) async fn save_retention(
        &self,
        tx: &mut MySqlTx,
        retention_days: u32,
    ) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO trash_settings (id, retention_days) VALUES (1, ?) \
             ON DUPLICATE KEY UPDATE retention_days = VALUES(retention_days)",
        )
        .bind(retention_days)
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub(crate) async fn expired(
        &self,
        definition: &RecordTypeDefinition,
        deleted_before: DateTime<Utc>,
        limit: u32,
        skip: &[Uuid],
    ) -> Result<Vec<Uuid>, AppError> {
        let deleted = deleted_column(definition);
        let mut builder = QueryBuilder::<MySql>::new(format!(
            "SELECT r.id FROM {} r WHERE {deleted} IS NOT NULL AND {deleted} < ",
            definition.table
        ));
        builder.push_bind(mapper::naive(deleted_before));
        push_partition(&mut builder, definition, "r");
        if !skip.is_empty() {
            builder.push(" AND r.id NOT IN (");
            let mut ids = builder.separated(", ");
            for id in skip {
                ids.push_bind(id.to_string());
            }
            builder.push(")");
        }
        builder
            .push(format!(" ORDER BY {deleted}, r.id LIMIT "))
            .push_bind(i64::from(limit));
        let ids: Vec<String> = builder.build_query_scalar().fetch_all(&self.pool).await?;
        Ok(ids
            .into_iter()
            .filter_map(|id| Uuid::parse_str(&id).ok())
            .collect())
    }
}
