use aphel_kernel::{application::shared::RecordType, errors::AppError};
use aphel_sdk::deals::{DealLifecycle, DealOutcomes, DealParties};
use sqlx::{MySqlPool, Row};
use uuid::Uuid;
pub(crate) struct Lifecycle;
#[async_trait::async_trait]
impl DealLifecycle for Lifecycle {
    async fn lifecycle_people(
        &self,
        ctx: &aphel_kernel::Ctx<'_>,
        id: Uuid,
    ) -> Result<Vec<Uuid>, AppError> {
        if !ctx.scope.is_system() {
            return Err(AppError::Forbidden(
                "deal lifecycle maintenance requires system scope".into(),
            ));
        }
        let rows: Vec<String> = sqlx::query_scalar(
            "SELECT contact_id FROM deals WHERE id = ? AND contact_id IS NOT NULL UNION SELECT contact_id FROM deal_contacts WHERE deal_id = ?",
        ).bind(id.to_string()).bind(id.to_string()).fetch_all(ctx.pool()).await?;
        rows.into_iter()
            .map(|value| {
                aphel_kernel::infrastructure::repositories::mapper::parse_uuid(&value, "contact_id")
            })
            .collect()
    }

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
