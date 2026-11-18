use crate::database::Db;
use crate::errors::AppError;

pub(crate) struct TenantRow {
    pub id: String,
    pub slug: String,
    pub name: String,
    pub database_name: String,
    pub status: String,
    pub plugin_management: bool,
}

type HostRow = (String, String, String, String, String, bool, Option<String>);

pub(crate) async fn with_hosts(pool: &Db) -> Result<Vec<(TenantRow, Option<String>)>, AppError> {
    let rows: Vec<HostRow> =
        sqlx::query_as(
            "SELECT t.id, t.slug, t.name, t.database_name, t.status, t.plugin_management_enabled, h.host \
             FROM tenants t LEFT JOIN tenant_hosts h ON h.tenant_id = t.id",
        )
        .fetch_all(pool)
        .await?;
    Ok(rows
        .into_iter()
        .map(
            |(id, slug, name, database_name, status, plugin_management, host)| {
                (
                    TenantRow {
                        id,
                        slug,
                        name,
                        database_name,
                        status,
                        plugin_management,
                    },
                    host,
                )
            },
        )
        .collect())
}

pub(crate) async fn by_slug(pool: &Db, slug: &str) -> Result<Option<TenantRow>, AppError> {
    let row: Option<(String, String, String, String, String, bool)> = sqlx::query_as(
        "SELECT id, slug, name, database_name, status, plugin_management_enabled \
         FROM tenants WHERE slug = ?",
    )
    .bind(slug)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(
        |(id, slug, name, database_name, status, plugin_management)| TenantRow {
            id,
            slug,
            name,
            database_name,
            status,
            plugin_management,
        },
    ))
}
