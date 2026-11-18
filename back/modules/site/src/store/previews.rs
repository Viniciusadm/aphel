use aphel_sdk::platform::database::Db;

pub(crate) struct NewPreview<'a> {
    pub token_hash: &'a str,
    pub theme: Option<&'a str>,
    pub record_type: Option<&'a str>,
    pub record_id: Option<&'a str>,
    pub draft_fields: Option<String>,
    pub target_path: Option<&'a str>,
    pub minutes: i64,
}

pub(crate) type StoredPreview = (
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
);

pub(crate) async fn create(pool: &Db, preview: NewPreview<'_>) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM site_previews WHERE expires_at < NOW(6)")
        .execute(pool)
        .await?;
    sqlx::query(
        "INSERT INTO site_previews (token_hash, theme, record_type, record_id, draft_fields, target_path, expires_at) \
         VALUES (?, ?, ?, ?, ?, ?, NOW(6) + INTERVAL ? MINUTE)",
    )
    .bind(preview.token_hash)
    .bind(preview.theme)
    .bind(preview.record_type)
    .bind(preview.record_id)
    .bind(preview.draft_fields)
    .bind(preview.target_path)
    .bind(preview.minutes)
    .execute(pool)
    .await?;
    Ok(())
}

pub(crate) async fn live(
    pool: &Db,
    token_hash: &str,
) -> Result<Option<StoredPreview>, sqlx::Error> {
    sqlx::query_as(
        "SELECT theme, record_type, record_id, draft_fields, target_path FROM site_previews \
         WHERE token_hash = ? AND expires_at > NOW(6)",
    )
    .bind(token_hash)
    .fetch_optional(pool)
    .await
}
