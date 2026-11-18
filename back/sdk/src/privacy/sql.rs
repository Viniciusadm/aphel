use super::SubjectMatcher;
use aphel_kernel::{
    Ctx, application::shared::RecordRef, database::transaction::Tx, errors::AppError,
};
use sqlx::{MySql, QueryBuilder, Row};
use uuid::Uuid;
pub fn contains_pattern(term: &str) -> String {
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

pub fn push_ids(builder: &mut QueryBuilder<MySql>, ids: &[Uuid]) {
    builder.push(" IN (");
    let mut separated = builder.separated(", ");
    for id in ids {
        separated.push_bind(id.to_string());
    }
    builder.push(")");
}

pub fn push_refs(
    builder: &mut QueryBuilder<MySql>,
    type_column: &str,
    id_column: &str,
    records: &[RecordRef],
) {
    builder.push("(");
    for (index, record) in records.iter().enumerate() {
        if index > 0 {
            builder.push(" OR ");
        }
        builder
            .push(format!("({type_column} = "))
            .push_bind(record.record_type.as_str().to_owned())
            .push(format!(" AND {id_column} = "))
            .push_bind(record.record_id.to_string())
            .push(")");
    }
    builder.push(")");
}

pub fn phone_digits(matcher: &SubjectMatcher) -> Vec<String> {
    let mut digits: Vec<String> = matcher.digits.iter().cloned().collect();
    if let Some(phone) = &matcher.phone {
        let international = phone.trim_start_matches('+').to_owned();
        if !digits.contains(&international) {
            digits.push(international);
        }
    }
    digits
}

pub fn push_text_match(
    builder: &mut QueryBuilder<MySql>,
    expression: &str,
    matcher: &SubjectMatcher,
) {
    if let Some(email) = &matcher.email {
        builder
            .push(format!("LOWER({expression}) LIKE "))
            .push_bind(contains_pattern(email));
        return;
    }
    if let Some(text) = &matcher.text {
        builder
            .push(format!("{expression} LIKE "))
            .push_bind(contains_pattern(text));
        return;
    }
    let digits = phone_digits(matcher);
    if digits.is_empty() {
        builder.push("FALSE");
        return;
    }
    builder.push("(");
    for (index, value) in digits.iter().enumerate() {
        if index > 0 {
            builder.push(" OR ");
        }
        builder
            .push(format!("REGEXP_REPLACE({expression}, '[^0-9]', '') LIKE "))
            .push_bind(contains_pattern(value));
    }
    builder.push(")");
}

pub async fn table_exists(ctx: &Ctx<'_>, table: &str) -> Result<bool, AppError> {
    Ok(sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM information_schema.TABLES WHERE TABLE_SCHEMA = DATABASE() AND TABLE_NAME = ?)").bind(table).fetch_one(ctx.pool()).await?)
}
pub async fn redact_text(
    tx: &mut Tx,
    table: &str,
    column: &str,
    redaction: &super::Redaction,
) -> Result<u64, AppError> {
    if !aphel_kernel::application::fields::is_sql_identifier(table)
        || !aphel_kernel::application::fields::is_sql_identifier(column)
    {
        return Err(AppError::Configuration(
            "invalid owned text column".to_owned(),
        ));
    }
    let redaction = redaction.mentions_only();
    let patterns: Vec<String> = redaction.values().map(contains_pattern).collect();
    if patterns.is_empty() {
        return Ok(0);
    }
    let mut updated = 0;
    let mut after = String::new();
    loop {
        let mut builder = QueryBuilder::<MySql>::new(format!(
            "SELECT id, CAST({column} AS CHAR) AS content FROM {table} WHERE id > "
        ));
        builder.push_bind(after.clone()).push(" AND (");
        for (index, pattern) in patterns.iter().enumerate() {
            if index > 0 {
                builder.push(" OR ");
            }
            builder
                .push(format!("LOWER(CAST({column} AS CHAR)) LIKE "))
                .push_bind(pattern.clone());
        }
        builder.push(") ORDER BY id LIMIT 500 FOR UPDATE");
        let rows = builder.build().fetch_all(&mut **tx).await?;
        for row in &rows {
            let id: String = row.try_get("id")?;
            let content: Option<String> = row.try_get("content")?;
            if let Some(redacted) = content
                .as_deref()
                .and_then(|text| redaction.apply_text(text))
            {
                sqlx::query(sqlx::AssertSqlSafe(format!(
                    "UPDATE {table} SET {column} = ? WHERE id = ?"
                )))
                .bind(redacted)
                .bind(&id)
                .execute(&mut **tx)
                .await?;
                updated += 1;
            }
            after = id;
        }
        if rows.len() < 500 {
            break;
        }
    }
    Ok(updated)
}
