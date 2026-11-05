use crate::application::shared::RecordType;
use crate::database::transaction::MySqlTx;
use crate::errors::AppError;

pub async fn purge_record_type_data(
    tx: &mut MySqlTx,
    record_type: &RecordType,
) -> Result<u64, AppError> {
    let mut removed = 0;
    for table in ["custom_field_values", "custom_field_multi_values"] {
        removed += sqlx::query(sqlx::AssertSqlSafe(format!(
            "DELETE FROM {table} WHERE record_type = ?"
        )))
        .bind(record_type.as_str())
        .execute(&mut **tx)
        .await?
        .rows_affected();
    }
    removed += sqlx::query(
        "DELETE l FROM record_relations l JOIN relationship_definitions d ON d.id = l.relationship_id \
         WHERE d.from_type = ? OR d.to_type = ?",
    )
    .bind(record_type.as_str())
    .bind(record_type.as_str())
    .execute(&mut **tx)
    .await?
    .rows_affected();
    Ok(removed)
}

fn dataset_record_filter(
    query: &mut sqlx::QueryBuilder<sqlx::MySql>,
    type_column: &str,
    id_column: &str,
    records: &[crate::application::shared::RecordRef],
) {
    if records.is_empty() {
        query.push("FALSE");
        return;
    }
    query.push(format!("({type_column}, {id_column}) IN ("));
    let mut tuples = query.separated(", ");
    for record in records {
        tuples
            .push("(")
            .push_bind_unseparated(record.record_type.as_str().to_owned())
            .push_unseparated(", ")
            .push_bind_unseparated(record.record_id.to_string())
            .push_unseparated(")");
    }
    query.push(")");
}

pub async fn backdate_dataset_history(
    tx: &mut MySqlTx,
    record: &crate::application::shared::RecordRef,
    created_at: chrono::DateTime<chrono::Utc>,
    changed_at: chrono::DateTime<chrono::Utc>,
) -> Result<(), AppError> {
    let changed_at = changed_at.max(created_at);
    for table in ["domain_events", "audit_logs"] {
        let ids: Vec<String> = sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT id FROM {table} WHERE record_type = ? AND record_id = ? ORDER BY occurred_at, id FOR UPDATE"))).bind(record.record_type.as_str()).bind(record.record_id.to_string()).fetch_all(&mut **tx).await?;
        let steps = i32::try_from(ids.len().saturating_sub(1).max(1)).map_err(|_| {
            AppError::Validation("record history is too large to redistribute".into())
        })?;
        for (index, id) in ids.iter().enumerate() {
            let index = i32::try_from(index).map_err(|_| {
                AppError::Validation("record history is too large to redistribute".into())
            })?;
            let occurred_at = created_at + (changed_at - created_at) * index / steps;
            sqlx::query(sqlx::AssertSqlSafe(format!(
                "UPDATE {table} SET occurred_at = ? WHERE id = ?"
            )))
            .bind(occurred_at.naive_utc())
            .bind(id)
            .execute(&mut **tx)
            .await?;
        }
    }
    Ok(())
}

pub async fn dataset_core_counts(
    tx: &mut MySqlTx,
) -> Result<std::collections::BTreeMap<String, u64>, AppError> {
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM custom_records")
        .fetch_one(&mut **tx)
        .await?;
    let mut counts =
        std::collections::BTreeMap::from([("custom_records".to_owned(), count.max(0) as u64)]);
    for object in crate::infrastructure::custom_objects::load_objects_in(tx).await? {
        let definition = crate::application::custom_objects::custom_record_definition(&object)?;
        if definition.storage == crate::application::fields::EntityStorage::Managed
            && crate::records::existing_tables(tx, &[definition.table.as_str()])
                .await?
                .contains(&definition.table)
        {
            let count: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
                "SELECT COUNT(*) FROM `{}`",
                definition.table
            )))
            .fetch_one(&mut **tx)
            .await?;
            counts.insert(
                definition.record_type.as_str().to_owned(),
                count.max(0) as u64,
            );
        }
    }
    Ok(counts)
}

pub async fn cleanup_dataset_references(
    tx: &mut MySqlTx,
    records: Option<&[crate::application::shared::RecordRef]>,
) -> Result<Vec<(String, String)>, AppError> {
    let mut candidates = std::collections::BTreeMap::new();
    let batches: Vec<_> = match records {
        Some(records) => records.chunks(100).map(Some).collect(),
        None => vec![None],
    };
    for batch in batches {
        let mut query = sqlx::QueryBuilder::<sqlx::MySql>::new(
            "SELECT DISTINCT f.id, f.storage_key FROM attachments a JOIN files f ON f.id = a.file_id WHERE ",
        );
        match batch {
            Some(records) => {
                dataset_record_filter(&mut query, "a.record_type", "a.record_id", records)
            }
            None => {
                query.push("TRUE");
            }
        }
        let files: Vec<(String, String)> = query.build_query_as().fetch_all(&mut **tx).await?;
        candidates.extend(files);
        for table in [
            "attachments",
            "domain_events",
            "audit_logs",
            "custom_field_values",
            "custom_field_multi_values",
            "custom_field_unique_values",
            "record_shares",
        ] {
            let mut query =
                sqlx::QueryBuilder::<sqlx::MySql>::new(format!("DELETE FROM {table} WHERE "));
            match batch {
                Some(records) => {
                    dataset_record_filter(&mut query, "record_type", "record_id", records)
                }
                None => {
                    query.push("TRUE");
                }
            }
            query.build().execute(&mut **tx).await?;
        }
        let mut query = sqlx::QueryBuilder::<sqlx::MySql>::new(
            "DELETE l FROM record_relations l JOIN relationship_definitions d ON d.id = l.relationship_id WHERE ",
        );
        match batch {
            Some(records) => {
                dataset_record_filter(&mut query, "d.from_type", "l.from_id", records);
                query.push(" OR ");
                dataset_record_filter(&mut query, "d.to_type", "l.to_id", records);
            }
            None => {
                query.push("TRUE");
            }
        }
        query.build().execute(&mut **tx).await?;
        if let Some(records) = batch {
            let fields: Vec<_> = records
                .iter()
                .filter(|record| record.record_type.as_str() == "field")
                .map(|record| record.record_id.to_string())
                .collect();
            if !fields.is_empty() {
                for table in [
                    "custom_field_values",
                    "custom_field_multi_values",
                    "custom_field_unique_values",
                ] {
                    let mut query = sqlx::QueryBuilder::<sqlx::MySql>::new(format!(
                        "DELETE FROM {table} WHERE field_id IN ("
                    ));
                    let mut ids = query.separated(", ");
                    for id in &fields {
                        ids.push_bind(id.clone());
                    }
                    query.push(")");
                    query.build().execute(&mut **tx).await?;
                }
            }
        }
    }
    Ok(candidates.into_iter().collect())
}

pub async fn cleanup_dataset_records(
    tx: &mut MySqlTx,
    records: Option<&[crate::application::shared::RecordRef]>,
) -> Result<std::collections::BTreeMap<String, u64>, AppError> {
    let mut counts = std::collections::BTreeMap::new();
    let objects = crate::infrastructure::custom_objects::load_objects_in(tx).await?;
    match records {
        None => {
            let removed = sqlx::query("DELETE FROM custom_records")
                .execute(&mut **tx)
                .await?
                .rows_affected();
            counts.insert("custom_records".to_owned(), removed);
            for object in objects {
                let definition =
                    crate::application::custom_objects::custom_record_definition(&object)?;
                if definition.storage == crate::application::fields::EntityStorage::Managed
                    && crate::records::existing_tables(tx, &[definition.table.as_str()])
                        .await?
                        .contains(&definition.table)
                {
                    let removed = sqlx::query(sqlx::AssertSqlSafe(format!(
                        "DELETE FROM `{}`",
                        definition.table
                    )))
                    .execute(&mut **tx)
                    .await?
                    .rows_affected();
                    counts.insert(definition.record_type.as_str().to_owned(), removed);
                }
            }
            let removed = sqlx::query("DELETE u FROM users u LEFT JOIN roles r ON r.id = u.role_id WHERE r.`key` IS NULL OR r.`key` <> 'admin'").execute(&mut **tx).await?.rows_affected();
            counts.insert("users".to_owned(), removed);
        }
        Some(records) => {
            for object in objects {
                let definition =
                    crate::application::custom_objects::custom_record_definition(&object)?;
                let ids: Vec<_> = records
                    .iter()
                    .filter(|record| record.record_type == definition.record_type)
                    .map(|record| record.record_id)
                    .collect();
                if ids.is_empty()
                    || !crate::records::existing_tables(tx, &[definition.table.as_str()])
                        .await?
                        .contains(&definition.table)
                {
                    continue;
                }
                let mut removed = 0;
                for batch in ids.chunks(1000) {
                    let mut query = sqlx::QueryBuilder::<sqlx::MySql>::new(format!(
                        "DELETE FROM `{}` r WHERE r.id IN (",
                        definition.table
                    ));
                    let mut ids = query.separated(", ");
                    for id in batch {
                        ids.push_bind(id.to_string());
                    }
                    query.push(")");
                    crate::infrastructure::query::compiler::push_partition(
                        &mut query,
                        &definition,
                        "r",
                    );
                    removed += query.build().execute(&mut **tx).await?.rows_affected();
                }
                counts.insert(definition.record_type.as_str().to_owned(), removed);
            }
            let fields: Vec<_> = records
                .iter()
                .filter(|record| record.record_type.as_str() == "field")
                .map(|record| record.record_id)
                .collect();
            let mut removed = 0;
            for batch in fields.chunks(1000) {
                let mut query = sqlx::QueryBuilder::<sqlx::MySql>::new(
                    "DELETE FROM field_definitions WHERE id IN (",
                );
                let mut ids = query.separated(", ");
                for id in batch {
                    ids.push_bind(id.to_string());
                }
                query.push(")");
                removed += query.build().execute(&mut **tx).await?.rows_affected();
            }
            if removed > 0 {
                counts.insert("field".to_owned(), removed);
                sqlx::query("UPDATE organization_settings SET custom_schema_version = custom_schema_version + 1 WHERE id = 1").execute(&mut **tx).await?;
            }
        }
    }
    Ok(counts)
}

pub async fn cleanup_dataset_files(
    tx: &mut MySqlTx,
    candidates: &[(String, String)],
    full: bool,
) -> Result<Vec<String>, AppError> {
    let ids: Vec<String> = if full {
        sqlx::query_scalar("SELECT id FROM files WHERE id NOT IN (SELECT logo_file_id FROM organization_settings WHERE logo_file_id IS NOT NULL UNION SELECT u.avatar_file_id FROM users u JOIN roles r ON r.id = u.role_id WHERE r.`key` = 'admin' AND u.avatar_file_id IS NOT NULL)").fetch_all(&mut **tx).await?
    } else {
        candidates.iter().map(|(id, _)| id.clone()).collect()
    };
    let ids = ids
        .into_iter()
        .map(|id| {
            uuid::Uuid::parse_str(&id)
                .map_err(|_| AppError::Internal("invalid stored file identity".into()))
        })
        .collect::<Result<std::collections::BTreeSet<_>, _>>()?;
    let ids: Vec<_> = ids.into_iter().collect();
    let mut keys = Vec::new();
    for batch in ids.chunks(1000) {
        keys.extend(
            crate::infrastructure::storage::references::delete_unreferenced_files(tx, batch)
                .await?,
        );
    }
    Ok(keys)
}
