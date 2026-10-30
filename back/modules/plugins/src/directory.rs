use std::collections::BTreeMap;

use serde::Deserialize;
use serde_json::{Value, json};
use sqlx::{MySql, MySqlPool, QueryBuilder, Row};

use aphel_kernel::errors::AppError;

const MAX_PAGE_SIZE: u32 = 100;
const MAX_SEARCH_LENGTH: usize = 100;

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UserQuery {
    #[serde(default)]
    search: Option<String>,
    #[serde(default)]
    role: Option<String>,
    #[serde(default)]
    team_id: Option<String>,
    #[serde(default)]
    ids: Vec<String>,
    #[serde(default)]
    page: Option<u32>,
    #[serde(default)]
    per_page: Option<u32>,
}

fn push_conditions(builder: &mut QueryBuilder<MySql>, query: &UserQuery) {
    builder.push(" WHERE u.status = 'active'");
    if let Some(search) = query
        .search
        .as_deref()
        .map(str::trim)
        .filter(|search| !search.is_empty())
    {
        let pattern = format!("%{}%", search.replace('%', "\\%").replace('_', "\\_"));
        builder
            .push(" AND (u.display_name LIKE ")
            .push_bind(pattern.clone())
            .push(" OR u.email LIKE ")
            .push_bind(pattern)
            .push(")");
    }
    if let Some(role) = &query.role {
        builder.push(" AND r.`key` = ").push_bind(role.clone());
    }
    if let Some(team) = &query.team_id {
        builder
            .push(" AND EXISTS (SELECT 1 FROM team_memberships m WHERE m.user_id = u.id AND m.team_id = ")
            .push_bind(team.clone())
            .push(")");
    }
    if !query.ids.is_empty() {
        builder.push(" AND u.id IN (");
        let mut ids = builder.separated(", ");
        for id in &query.ids {
            ids.push_bind(id.clone());
        }
        builder.push(")");
    }
}

pub async fn users(pool: &MySqlPool, query: UserQuery) -> Result<Value, AppError> {
    if query
        .search
        .as_ref()
        .is_some_and(|search| search.chars().count() > MAX_SEARCH_LENGTH)
        || query.ids.len() > MAX_PAGE_SIZE as usize
    {
        return Err(AppError::Validation(format!(
            "search has at most {MAX_SEARCH_LENGTH} characters and ids at most {MAX_PAGE_SIZE} entries"
        )));
    }
    let per_page = query.per_page.unwrap_or(50).clamp(1, MAX_PAGE_SIZE);
    let page = query.page.unwrap_or(1).max(1);
    let mut count =
        QueryBuilder::<MySql>::new("SELECT COUNT(*) FROM users u JOIN roles r ON r.id = u.role_id");
    push_conditions(&mut count, &query);
    let total: i64 = count.build_query_scalar().fetch_one(pool).await?;
    let mut select = QueryBuilder::<MySql>::new(
        "SELECT u.id, u.display_name, u.email, r.`key` AS role_key, r.name AS role_name \
         FROM users u JOIN roles r ON r.id = u.role_id",
    );
    push_conditions(&mut select, &query);
    select
        .push(" ORDER BY u.display_name, u.id LIMIT ")
        .push_bind(per_page)
        .push(" OFFSET ")
        .push_bind((page - 1) * per_page);
    let rows = select.build().fetch_all(pool).await?;
    let ids: Vec<String> = rows
        .iter()
        .map(|row| row.try_get::<String, _>("id"))
        .collect::<Result<_, _>>()?;
    let mut teams: BTreeMap<String, Vec<Value>> = BTreeMap::new();
    if !ids.is_empty() {
        let mut builder = QueryBuilder::<MySql>::new(
            "SELECT m.user_id, t.id, t.name FROM team_memberships m JOIN teams t ON t.id = m.team_id \
             WHERE m.user_id IN (",
        );
        let mut separated = builder.separated(", ");
        for id in &ids {
            separated.push_bind(id.clone());
        }
        builder.push(") ORDER BY t.name");
        for row in builder.build().fetch_all(pool).await? {
            teams
                .entry(row.try_get("user_id")?)
                .or_default()
                .push(json!({ "id": row.try_get::<String, _>("id")?, "name": row.try_get::<String, _>("name")? }));
        }
    }
    let mut data = Vec::with_capacity(rows.len());
    for row in &rows {
        let id: String = row.try_get("id")?;
        data.push(json!({
            "id": id,
            "name": row.try_get::<String, _>("display_name")?,
            "email": row.try_get::<String, _>("email")?,
            "role": { "key": row.try_get::<String, _>("role_key")?, "name": row.try_get::<String, _>("role_name")? },
            "teams": teams.remove(&id).unwrap_or_default(),
        }));
    }
    Ok(json!({ "data": data, "total": total }))
}

pub async fn roles(pool: &MySqlPool) -> Result<Value, AppError> {
    let rows = sqlx::query("SELECT `key`, name, is_system FROM roles ORDER BY name")
        .fetch_all(pool)
        .await?;
    let roles: Vec<Value> = rows
        .iter()
        .map(|row| {
            Ok(json!({
                "key": row.try_get::<String, _>("key")?,
                "name": row.try_get::<String, _>("name")?,
                "is_system": row.try_get::<bool, _>("is_system")?,
            }))
        })
        .collect::<Result<_, sqlx::Error>>()?;
    Ok(Value::Array(roles))
}

pub async fn teams(pool: &MySqlPool) -> Result<Value, AppError> {
    let rows = sqlx::query("SELECT id, name, parent_team_id FROM teams ORDER BY name")
        .fetch_all(pool)
        .await?;
    let teams: Vec<Value> = rows
        .iter()
        .map(|row| {
            Ok(json!({
                "id": row.try_get::<String, _>("id")?,
                "name": row.try_get::<String, _>("name")?,
                "parent_id": row.try_get::<Option<String>, _>("parent_team_id")?,
            }))
        })
        .collect::<Result<_, sqlx::Error>>()?;
    Ok(Value::Array(teams))
}
