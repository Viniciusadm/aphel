use aphel_kernel::{application::shared::RecordType, errors::AppError};
use aphel_sdk::deals::{DealLifecycle, DealOutcomes, DealParties};
use sqlx::{MySqlPool, Row};
use uuid::Uuid;
pub(crate) struct Lifecycle;
#[async_trait::async_trait]
impl DealLifecycle for Lifecycle {
    async fn parties(&self, pool: &MySqlPool, id: Uuid) -> Result<Option<DealParties>, AppError> {
        let row = sqlx::query("SELECT contact_id,company_id FROM deals WHERE id = ?")
            .bind(id.to_string())
            .fetch_optional(pool)
            .await?;
        row.map(|row| {
            let parse = |key| -> Result<Option<Uuid>, AppError> {
                row.try_get::<Option<String>, _>(key)?
                    .map(|value| {
                        Uuid::parse_str(&value)
                            .map_err(|_| AppError::Internal("invalid deal party identity".into()))
                    })
                    .transpose()
            };
            Ok(DealParties {
                contact_id: parse("contact_id")?,
                company_id: parse("company_id")?,
            })
        })
        .transpose()
    }
    async fn outcomes(
        &self,
        pool: &MySqlPool,
        entity: &RecordType,
        id: Uuid,
    ) -> Result<DealOutcomes, AppError> {
        let column = match entity.as_str() {
            "people.person" => "contact_id",
            "organizations.organization" => "company_id",
            _ => return Err(AppError::Validation("unsupported deal party entity".into())),
        };
        let rows: Vec<(String,i64)> = sqlx::query_as(sqlx::AssertSqlSafe(format!("SELECT status,COUNT(*) FROM deals WHERE {column} = ? AND deleted_at IS NULL GROUP BY status"))).bind(id.to_string()).fetch_all(pool).await?;
        let mut outcomes = DealOutcomes::default();
        for (status, total) in rows {
            let total = u64::try_from(total).unwrap_or_default();
            match status.as_str() {
                "open" => outcomes.open += total,
                "won" => outcomes.won += total,
                "lost" => outcomes.lost += total,
                _ => {}
            }
        }
        Ok(outcomes)
    }
}
