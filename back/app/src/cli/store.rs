use aphel_kernel::database::Db;

pub(crate) async fn server_version(pool: &Db) -> Option<String> {
    sqlx::query_scalar::<_, String>("SELECT VERSION()")
        .fetch_one(pool)
        .await
        .ok()
}

pub(crate) async fn smtp_endpoint(pool: &Db) -> Option<(Option<String>, Option<i64>)> {
    sqlx::query_as(
        "SELECT JSON_UNQUOTE(JSON_EXTRACT(config, '$.host')), \
         CAST(JSON_EXTRACT(config, '$.port') AS SIGNED) FROM integrations \
         WHERE provider_key = 'smtp' AND status IN ('active', 'error')",
    )
    .fetch_optional(pool)
    .await
    .unwrap_or(None)
}

pub(crate) async fn active_tenant_count(control: &Db) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM tenants WHERE status = 'active'")
        .fetch_one(control)
        .await
        .unwrap_or_default()
}

pub(crate) async fn recreate_database(database_url: &str) -> Result<(), sqlx::Error> {
    use sqlx::migrate::MigrateDatabase;
    sqlx::MySql::drop_database(database_url).await?;
    sqlx::MySql::create_database(database_url).await
}
