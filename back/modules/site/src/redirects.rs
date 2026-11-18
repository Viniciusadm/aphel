use aphel_kernel::errors::AppError;
use aphel_sdk::platform::database::Db;

const MAX_VALUE_LENGTH: usize = 255;

pub(crate) async fn record_change(
    pool: &Db,
    record_type: &str,
    field: &str,
    old_value: &str,
    new_value: &str,
) -> Result<(), AppError> {
    if old_value.is_empty()
        || old_value == new_value
        || old_value.len() > MAX_VALUE_LENGTH
        || new_value.len() > MAX_VALUE_LENGTH
    {
        return Ok(());
    }
    let mut tx = pool.begin().await?;
    sqlx::query("DELETE FROM site_redirects WHERE record_type = ? AND field = ? AND old_value = ?")
        .bind(record_type)
        .bind(field)
        .bind(new_value)
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "UPDATE site_redirects SET new_value = ? WHERE record_type = ? AND field = ? AND new_value = ?",
    )
    .bind(new_value)
    .bind(record_type)
    .bind(field)
    .bind(old_value)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "INSERT INTO site_redirects (record_type, field, old_value, new_value) VALUES (?, ?, ?, ?) \
         ON DUPLICATE KEY UPDATE new_value = VALUES(new_value), created_at = CURRENT_TIMESTAMP(6)",
    )
    .bind(record_type)
    .bind(field)
    .bind(old_value)
    .bind(new_value)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(())
}

pub(crate) async fn lookup(
    pool: &Db,
    record_type: &str,
    field: &str,
    value: &str,
) -> Result<Option<String>, AppError> {
    Ok(sqlx::query_scalar(
        "SELECT new_value FROM site_redirects WHERE record_type = ? AND field = ? AND old_value = ?",
    )
    .bind(record_type)
    .bind(field)
    .bind(value)
    .fetch_optional(pool)
    .await?)
}

const MAX_SOURCE_LENGTH: usize = 255;
const MAX_TARGET_LENGTH: usize = 500;
const UNREACHABLE_PREFIXES: [&str; 2] = ["/_aphel", "/api"];

pub(crate) struct PathRedirect {
    pub target: String,
    pub permanent: bool,
}

fn has_forbidden_characters(value: &str) -> bool {
    value
        .chars()
        .any(|character| character.is_whitespace() || character.is_control())
}

pub(crate) fn normalize_source(source: &str) -> Result<String, AppError> {
    let source = source.trim();
    let invalid = |message: &str| Err(AppError::Validation(message.to_owned()));
    if !source.starts_with('/') || source.starts_with("//") {
        return invalid("the old address starts with / (for example /old-page)");
    }
    if source.len() > MAX_SOURCE_LENGTH
        || has_forbidden_characters(source)
        || source.contains(['?', '#'])
    {
        return invalid("the old address is a plain path, without spaces, ? or #");
    }
    if UNREACHABLE_PREFIXES
        .iter()
        .any(|prefix| source == *prefix || source.starts_with(&format!("{prefix}/")))
    {
        return invalid("the old address is reserved by the system");
    }
    Ok(match source.trim_end_matches('/') {
        "" => "/".to_owned(),
        trimmed => trimmed.to_owned(),
    })
}

pub(crate) fn normalize_target(target: &str, source: &str) -> Result<String, AppError> {
    let target = target.trim();
    let site_path = target.starts_with('/') && !target.starts_with("//");
    let external = target.starts_with("https://") || target.starts_with("http://");
    if !(site_path || external)
        || target.len() > MAX_TARGET_LENGTH
        || has_forbidden_characters(target)
    {
        return Err(AppError::Validation(
            "the target is a site path (for example /blog) or a full address starting with https://"
                .to_owned(),
        ));
    }
    if target == source {
        return Err(AppError::Validation(
            "the target must differ from the old address".to_owned(),
        ));
    }
    Ok(target.to_owned())
}

pub(crate) async fn find_path(pool: &Db, path: &str) -> Result<Option<PathRedirect>, AppError> {
    let path = match path.trim_end_matches('/') {
        "" => "/",
        trimmed => trimmed,
    };
    let row: Option<(String, bool)> =
        sqlx::query_as("SELECT target, permanent FROM site_path_redirects WHERE source_path = ?")
            .bind(path)
            .fetch_optional(pool)
            .await?;
    Ok(row.map(|(target, permanent)| PathRedirect { target, permanent }))
}
