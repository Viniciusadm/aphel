use async_trait::async_trait;
use sqlx::{MySql, MySqlPool, QueryBuilder, Row};

use crate::application::directory::{DirectoryQuery, DirectoryUser, UserDirectory};
use crate::application::organization_admin::{ListWindow, Listing};
use crate::application::query::{LIKE_ESCAPE, escape_like};
use crate::domain::UserId;
use crate::domain::users::UserStatus;
use crate::errors::AppError;
use crate::infrastructure::repositories::mapper;

#[derive(Clone)]
pub struct MySqlUserDirectory {
    pool: MySqlPool,
}

impl MySqlUserDirectory {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

fn push_conditions(builder: &mut QueryBuilder<MySql>, query: &DirectoryQuery) {
    builder
        .push(" WHERE u.status = ")
        .push_bind(UserStatus::Active.as_str());
    if let Some(search) = &query.search {
        let pattern = format!("%{}%", escape_like(search));
        builder
            .push(" AND (u.display_name LIKE ")
            .push_bind(pattern.clone())
            .push(format!(" ESCAPE '{LIKE_ESCAPE}' OR u.email LIKE "))
            .push_bind(pattern)
            .push(format!(" ESCAPE '{LIKE_ESCAPE}')"));
    }
}

#[async_trait]
impl UserDirectory for MySqlUserDirectory {
    async fn list_active(
        &self,
        query: &DirectoryQuery,
        window: ListWindow,
    ) -> Result<Listing<DirectoryUser>, AppError> {
        let mut count = QueryBuilder::<MySql>::new("SELECT COUNT(*) FROM users u");
        push_conditions(&mut count, query);
        let total: i64 = count.build_query_scalar().fetch_one(&self.pool).await?;

        let mut select =
            QueryBuilder::<MySql>::new("SELECT u.id, u.display_name, u.email FROM users u");
        push_conditions(&mut select, query);
        select
            .push(" ORDER BY u.display_name, u.id LIMIT ")
            .push_bind(window.limit)
            .push(" OFFSET ")
            .push_bind(window.offset);
        let items = select
            .build()
            .fetch_all(&self.pool)
            .await?
            .iter()
            .map(|row| {
                Ok(DirectoryUser {
                    id: mapper::typed_id(row.try_get("id")?, UserId::new)?,
                    display_name: row.try_get("display_name")?,
                    email: row.try_get("email")?,
                })
            })
            .collect::<Result<Vec<_>, AppError>>()?;
        Ok(Listing {
            items,
            total: total as u64,
        })
    }
}
