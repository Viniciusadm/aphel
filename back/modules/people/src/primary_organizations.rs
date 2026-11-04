use aphel_sdk::{
    AppError,
    access::AccessScope,
    people::{PrimaryOrganization, PrimaryOrganizationReader},
};
use sqlx::{MySqlPool, QueryBuilder};
use uuid::Uuid;
pub(crate) struct Reader;
fn authorize(scope: &AccessScope, ids: &[Uuid]) -> Result<(), AppError> {
    if !scope.is_system() {
        return Err(AppError::Forbidden(
            "primary organization maintenance requires system scope".into(),
        ));
    }
    if ids.len() > 1000 {
        return Err(AppError::Validation(
            "at most 1000 primary organization targets may be requested".into(),
        ));
    }
    Ok(())
}
async fn read(
    pool: &MySqlPool,
    query: &mut QueryBuilder<sqlx::MySql>,
) -> Result<Vec<PrimaryOrganization>, AppError> {
    query
        .build()
        .fetch_all(pool)
        .await?
        .iter()
        .map(|row| {
            Ok(PrimaryOrganization {
                person_id: aphel_kernel::infrastructure::fields::mysql_definitions::uuid_column(
                    row, "id",
                )?,
                organization_id:
                    aphel_kernel::infrastructure::fields::mysql_definitions::uuid_column(
                        row,
                        "company_id",
                    )?,
            })
        })
        .collect()
}
#[async_trait::async_trait]
impl PrimaryOrganizationReader for Reader {
    async fn by_people(
        &self,
        pool: &MySqlPool,
        scope: &AccessScope,
        ids: &[Uuid],
        include_deleted: bool,
    ) -> Result<Vec<PrimaryOrganization>, AppError> {
        authorize(scope, ids)?;
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut query = QueryBuilder::new(
            "SELECT id, company_id FROM people WHERE company_id IS NOT NULL AND id IN (",
        );
        let mut separated = query.separated(", ");
        for id in ids {
            separated.push_bind(id.to_string());
        }
        query.push(")");
        if !include_deleted {
            query.push(" AND deleted_at IS NULL");
        }
        read(pool, &mut query).await
    }
    async fn people_page(
        &self,
        pool: &MySqlPool,
        scope: &AccessScope,
        organizations: &[Uuid],
        after: Option<Uuid>,
    ) -> Result<Vec<PrimaryOrganization>, AppError> {
        authorize(scope, organizations)?;
        if organizations.is_empty() {
            return Ok(Vec::new());
        }
        let mut query = QueryBuilder::new(
            "SELECT id, company_id FROM people WHERE deleted_at IS NULL AND company_id IN (",
        );
        let mut separated = query.separated(", ");
        for id in organizations {
            separated.push_bind(id.to_string());
        }
        query.push(")");
        if let Some(after) = after {
            query.push(" AND id > ").push_bind(after.to_string());
        }
        query.push(" ORDER BY id LIMIT 1000");
        read(pool, &mut query).await
    }
}
