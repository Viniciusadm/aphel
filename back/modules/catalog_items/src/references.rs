use aphel_kernel::application::access::Authorizer;
use aphel_sdk::catalog_items::CatalogReferences;
use aphel_sdk::{
    AppError,
    access::{AccessScope, Permission},
};
use sqlx::{MySqlPool, QueryBuilder, Row};
use std::collections::{BTreeSet, HashMap};
use uuid::Uuid;
pub(crate) struct ReferenceReader;
#[async_trait::async_trait]
impl CatalogReferences for ReferenceReader {
    async fn existing(
        &self,
        pool: &MySqlPool,
        scope: &AccessScope,
        ids: &[Uuid],
    ) -> Result<Vec<Uuid>, AppError> {
        if aphel_sdk::access::require_organization_record_scope(
            scope,
            &Permission::from_static(aphel_sdk::catalog_items::READ),
        )
        .is_err()
            && scope.require(&Permission::SETTINGS_MANAGE).is_err()
        {
            return Ok(Vec::new());
        }
        let ids: Vec<_> = ids
            .iter()
            .copied()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        let mut found = Vec::new();
        for batch in ids.chunks(1000) {
            let mut query =
                QueryBuilder::new("SELECT id FROM products WHERE deleted_at IS NULL AND id IN (");
            let mut separated = query.separated(", ");
            for id in batch {
                separated.push_bind(id.to_string());
            }
            query.push(")");
            let rows: Vec<String> = query.build_query_scalar().fetch_all(pool).await?;
            for id in rows {
                found.push(Uuid::parse_str(&id).map_err(|_| {
                    AppError::Internal("invalid catalog reference identity".into())
                })?);
            }
        }
        Ok(found)
    }
    async fn codes(
        &self,
        pool: &MySqlPool,
        scope: &AccessScope,
        ids: &[Uuid],
    ) -> Result<HashMap<Uuid, Option<String>>, AppError> {
        scope.require(&Permission::SETTINGS_MANAGE)?;
        let ids: Vec<_> = ids
            .iter()
            .copied()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        let mut codes = HashMap::new();
        for batch in ids.chunks(1000) {
            let mut query = QueryBuilder::new(
                "SELECT id, code FROM products WHERE deleted_at IS NULL AND id IN (",
            );
            let mut separated = query.separated(", ");
            for id in batch {
                separated.push_bind(id.to_string());
            }
            query.push(")");
            for row in query.build().fetch_all(pool).await? {
                let id: String = row.try_get("id")?;
                let id = Uuid::parse_str(&id).map_err(|_| {
                    AppError::Internal("invalid catalog configuration identity".into())
                })?;
                codes.insert(id, row.try_get("code")?);
            }
        }
        Ok(codes)
    }
    async fn by_codes(
        &self,
        pool: &MySqlPool,
        scope: &AccessScope,
        codes: &[String],
    ) -> Result<Vec<(String, Uuid)>, AppError> {
        scope.require(&Permission::SETTINGS_MANAGE)?;
        let codes: Vec<_> = codes.iter().collect::<BTreeSet<_>>().into_iter().collect();
        let mut found = Vec::new();
        for batch in codes.chunks(1000) {
            let mut query = QueryBuilder::new(
                "SELECT active_code, id FROM products WHERE deleted_at IS NULL AND active_code IN (",
            );
            let mut separated = query.separated(", ");
            for code in batch {
                separated.push_bind((*code).clone());
            }
            query.push(")");
            for row in query.build().fetch_all(pool).await? {
                let id: String = row.try_get("id")?;
                let id = Uuid::parse_str(&id).map_err(|_| {
                    AppError::Internal("invalid catalog configuration identity".into())
                })?;
                found.push((row.try_get("active_code")?, id));
            }
        }
        Ok(found)
    }
}
