use std::collections::BTreeMap;

use serde_json::Value;
use sqlx::{MySql, MySqlPool, QueryBuilder, Row};

use crate::application::fields::EntityKeyAliases;
use crate::application::shared::RecordType;
use crate::errors::AppError;
use crate::infrastructure::query::compiler::quoted;

#[derive(Clone)]
struct Alias {
    legacy: String,
    key: String,
    owner: String,
    dynamic: bool,
}

pub async fn migrate_entity_keys(
    pool: &MySqlPool,
    app: &crate::modules::App,
) -> Result<(), AppError> {
    let mut aliases = BTreeMap::<String, Alias>::new();
    for (_, values) in app.all_contributions::<EntityKeyAliases>() {
        for (legacy, key) in values.0 {
            RecordType::parse((*key).to_owned())?;
            if aliases.get(*legacy).is_some_and(|alias| alias.key != *key) {
                return Err(AppError::Configuration(format!(
                    "ambiguous entity alias {legacy}"
                )));
            }
            aliases.insert(
                (*legacy).to_owned(),
                Alias {
                    legacy: (*legacy).to_owned(),
                    key: (*key).to_owned(),
                    owner: key.split('.').next().unwrap_or_default().to_owned(),
                    dynamic: false,
                },
            );
        }
    }
    let objects = sqlx::query("SELECT `key`, module_key FROM custom_objects")
        .fetch_all(pool)
        .await?;
    for row in objects {
        let object: String = row.try_get("key")?;
        let owner: Option<String> = row.try_get("module_key")?;
        let key = match &owner {
            Some(owner) => format!(
                "{owner}.{}",
                object.strip_prefix(&format!("{owner}_")).unwrap_or(&object)
            ),
            None => format!("custom.{object}"),
        };
        let legacy = format!("co_{object}");
        aliases.insert(
            legacy.clone(),
            Alias {
                legacy,
                key,
                owner: owner.unwrap_or_else(|| "custom".to_owned()),
                dynamic: true,
            },
        );
    }
    let mut tx = pool.begin().await?;
    let mut pending = Vec::new();
    for alias in aliases.values() {
        let stored: Option<(String, bool)> = sqlx::query_as(
            "SELECT entity_key, applied FROM entity_key_aliases WHERE legacy_key = ? FOR UPDATE",
        )
        .bind(&alias.legacy)
        .fetch_optional(&mut *tx)
        .await?;
        match stored {
            Some((key, _)) if key != alias.key => {
                return Err(AppError::Configuration(
                    "an entity alias changed its owner".to_owned(),
                ));
            }
            Some((_, true)) => continue,
            Some(_) => {}
            None => {
                sqlx::query("INSERT INTO entity_key_aliases (legacy_key, entity_key, owner_key) VALUES (?, ?, ?)").bind(&alias.legacy).bind(&alias.key).bind(&alias.owner).execute(&mut *tx).await?;
            }
        }
        if alias.dynamic {
            let exists: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM field_definitions WHERE record_type = ?)",
            )
            .bind(&alias.legacy)
            .fetch_one(&mut *tx)
            .await?;
            if !exists {
                sqlx::query("UPDATE entity_key_aliases SET applied = TRUE WHERE legacy_key = ?")
                    .bind(&alias.legacy)
                    .execute(&mut *tx)
                    .await?;
                continue;
            }
        }
        pending.push(alias.clone());
    }
    if pending.is_empty() {
        tx.commit().await?;
        return Ok(());
    }
    let columns = sqlx::query("SELECT TABLE_NAME, COLUMN_NAME, DATA_TYPE, COLUMN_KEY, ORDINAL_POSITION FROM information_schema.COLUMNS WHERE TABLE_SCHEMA = DATABASE() ORDER BY TABLE_NAME, ORDINAL_POSITION").fetch_all(&mut *tx).await?;
    let mut shapes = BTreeMap::<String, Vec<(String, String, bool)>>::new();
    for row in columns {
        shapes.entry(row.try_get("TABLE_NAME")?).or_default().push((
            row.try_get("COLUMN_NAME")?,
            row.try_get("DATA_TYPE")?,
            row.try_get::<String, _>("COLUMN_KEY")? == "PRI",
        ));
    }
    for (table, columns) in &shapes {
        if table == "entity_key_aliases" {
            continue;
        }
        for (column, kind, _) in columns {
            if !matches!(kind.as_str(), "varchar" | "char" | "text") {
                continue;
            }
            if [
                "record_type",
                "from_type",
                "to_type",
                "target_record_type",
                "related_type",
                "subject_type",
                "subject_kind",
                "entity_type",
            ]
            .contains(&column.as_str())
                || column.ends_with("_record_type")
            {
                for alias in &pending {
                    sqlx::query(sqlx::AssertSqlSafe(format!(
                        "UPDATE {} SET {} = ? WHERE {} = ?",
                        quoted(table),
                        quoted(column),
                        quoted(column)
                    )))
                    .bind(&alias.key)
                    .bind(&alias.legacy)
                    .execute(&mut *tx)
                    .await?;
                }
            }
            if ["permission", "scope", "action", "event_type"].contains(&column.as_str()) {
                for alias in pending.iter().filter(|alias| alias.dynamic) {
                    for delimiter in ['.', ':'] {
                        let prefix = format!("{}{delimiter}", alias.legacy);
                        sqlx::query(sqlx::AssertSqlSafe(format!(
                            "UPDATE {} SET {} = CONCAT(?, SUBSTRING({}, ?)) WHERE LEFT({}, ?) = ?",
                            quoted(table),
                            quoted(column),
                            quoted(column),
                            quoted(column)
                        )))
                        .bind(&alias.key)
                        .bind(alias.legacy.len() as u32 + 1)
                        .bind(prefix.len() as u32)
                        .bind(&prefix)
                        .execute(&mut *tx)
                        .await?;
                    }
                }
            }
        }
        let keys = columns
            .iter()
            .filter(|(_, _, key)| *key)
            .collect::<Vec<_>>();
        if keys.is_empty() {
            continue;
        }
        for (column, kind, _) in columns.iter().filter(|(_, kind, _)| kind == "json") {
            let _ = kind;
            let mut offset = 0_u64;
            loop {
                let mut query =
                    QueryBuilder::<MySql>::new(format!("SELECT {} AS document", quoted(column)));
                for (index, (key, kind, _)) in keys.iter().enumerate() {
                    let expression = if kind.contains("binary") || kind.contains("blob") {
                        format!("HEX({})", quoted(key))
                    } else {
                        format!("CAST({} AS CHAR)", quoted(key))
                    };
                    query.push(format!(", {expression} AS key_{index}"));
                }
                query
                    .push(format!(
                        " FROM {} WHERE {} IS NOT NULL ORDER BY {} LIMIT 500 OFFSET ",
                        quoted(table),
                        quoted(column),
                        keys.iter()
                            .map(|(key, _, _)| quoted(key))
                            .collect::<Vec<_>>()
                            .join(", ")
                    ))
                    .push_bind(offset);
                let rows = query.build().fetch_all(&mut *tx).await?;
                let count = rows.len();
                for row in rows {
                    let mut value = row.try_get::<sqlx::types::Json<Value>, _>("document")?.0;
                    if !rewrite(&mut value, column, &pending) {
                        continue;
                    }
                    let mut update = QueryBuilder::<MySql>::new(format!(
                        "UPDATE {} SET {} = ",
                        quoted(table),
                        quoted(column)
                    ));
                    update.push_bind(sqlx::types::Json(value)).push(" WHERE ");
                    for (index, (key, kind, _)) in keys.iter().enumerate() {
                        if index > 0 {
                            update.push(" AND ");
                        }
                        let value: String = row.try_get(format!("key_{index}").as_str())?;
                        update.push(format!("{} = ", quoted(key)));
                        if kind.contains("binary") || kind.contains("blob") {
                            update.push("UNHEX(").push_bind(value).push(")");
                        } else {
                            update.push_bind(value);
                        }
                    }
                    update.build().execute(&mut *tx).await?;
                }
                if count < 500 {
                    break;
                }
                offset += count as u64;
            }
        }
    }
    for alias in &pending {
        sqlx::query("UPDATE entity_key_aliases SET applied = TRUE WHERE legacy_key = ?")
            .bind(&alias.legacy)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    Ok(())
}

fn rewrite(value: &mut Value, context: &str, aliases: &[Alias]) -> bool {
    let mut changed = false;
    match value {
        Value::String(text) => {
            if [
                "record_type",
                "record_types",
                "target_record_type",
                "from_type",
                "to_type",
                "entity_type",
                "entity",
                "target",
                "subject_type",
                "related_type",
            ]
            .contains(&context)
                && let Some(alias) = aliases.iter().find(|alias| alias.legacy == *text)
            {
                *text = alias.key.clone();
                changed = true;
            }
            if [
                "permission",
                "permissions",
                "requires",
                "optional_requires",
                "scope",
                "scopes",
                "event_type",
                "events",
                "action",
            ]
            .contains(&context)
                && let Some(alias) = aliases.iter().filter(|alias| alias.dynamic).find(|alias| {
                    text.strip_prefix(&alias.legacy)
                        .is_some_and(|rest| rest.starts_with('.') || rest.starts_with(':'))
                })
            {
                *text = format!("{}{}", alias.key, &text[alias.legacy.len()..]);
                changed = true;
            }
        }
        Value::Array(values) => {
            for value in values {
                changed |= rewrite(value, context, aliases);
            }
        }
        Value::Object(values) => {
            let filter_type = values
                .get("field")
                .and_then(Value::as_str)
                .is_some_and(|field| ["record_type", "target_record_type"].contains(&field));
            if filter_type && let Some(value) = values.get_mut("value") {
                changed |= rewrite(value, "record_types", aliases);
            }
            if ["record_types", "record_counts", "entity_counts"].contains(&context) {
                for alias in aliases {
                    if let Some(value) = values.remove(&alias.legacy) {
                        values.insert(alias.key.clone(), value);
                        changed = true;
                    }
                }
            }
            for (key, value) in values.iter_mut() {
                changed |= rewrite(value, key, aliases);
            }
        }
        _ => {}
    }
    changed
}
