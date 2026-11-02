use chrono::NaiveDateTime;
use serde_json::{Map, Value};
use sqlx::mysql::MySqlRow;
use sqlx::types::Json;
use sqlx::{MySqlPool, QueryBuilder, Row};
use uuid::Uuid;

use crate::domain::custom::{ReportVisualization, SavedReport};
use ::saved_views::application::ViewVisibility;
use aphel_kernel::Ctx;
use aphel_kernel::application::query::AggregateQuery;
use aphel_kernel::application::shared::RecordType;
use aphel_kernel::database::transaction::MySqlTx;
use aphel_kernel::domain::access::AccessScope;
use aphel_kernel::domain::{TeamId, UserId};
use aphel_kernel::errors::AppError;
use aphel_kernel::infrastructure::fields::mysql_definitions::uuid_column;
use aphel_kernel::infrastructure::repositories::mapper;

const REPORT_COLUMNS: &str = "id, record_type, name, description, owner_id, visibility, team_id, \
     visualization, definition, settings, created_at, updated_at";

#[derive(Clone)]
pub(crate) struct ReportStore {
    pool: MySqlPool,
}

impl ReportStore {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    pub(crate) fn of(ctx: &Ctx<'_>) -> Self {
        Self::new(ctx.pool().clone())
    }
}

pub(crate) fn to_json<T: serde::Serialize>(value: &T) -> Result<Json<Value>, AppError> {
    serde_json::to_value(value)
        .map(Json)
        .map_err(|error| AppError::Internal(format!("cannot serialize report: {error}")))
}

fn report_from_row(row: &MySqlRow) -> Result<SavedReport, AppError> {
    let record_type: String = row.try_get("record_type")?;
    let visibility: String = row.try_get("visibility")?;
    let visualization: String = row.try_get("visualization")?;
    let Json(definition): Json<Value> = row.try_get("definition")?;
    let Json(settings): Json<Value> = row.try_get("settings")?;
    let created_at: NaiveDateTime = row.try_get("created_at")?;
    let updated_at: NaiveDateTime = row.try_get("updated_at")?;
    Ok(SavedReport {
        id: uuid_column(row, "id")?,
        record_type: RecordType::parse(record_type.clone())
            .map_err(|_| mapper::invalid_column("record_type", &record_type))?,
        name: row.try_get("name")?,
        description: row.try_get("description")?,
        owner_id: row
            .try_get::<Option<String>, _>("owner_id")?
            .map(|id| mapper::typed_id(id, UserId::new))
            .transpose()?,
        visibility: ViewVisibility::parse(&visibility)
            .ok_or_else(|| mapper::invalid_column("visibility", &visibility))?,
        team_id: row
            .try_get::<Option<String>, _>("team_id")?
            .map(|id| mapper::typed_id(id, TeamId::new))
            .transpose()?,
        visualization: ReportVisualization::parse(&visualization)
            .ok_or_else(|| mapper::invalid_column("visualization", &visualization))?,
        definition: serde_json::from_value::<AggregateQuery>(definition.clone())
            .map_err(|_| mapper::invalid_column("definition", &definition.to_string()))?,
        settings: serde_json::from_value::<Map<String, Value>>(settings.clone())
            .map_err(|_| mapper::invalid_column("settings", &settings.to_string()))?,
        created_at: mapper::utc(created_at),
        updated_at: mapper::utc(updated_at),
    })
}

impl ReportStore {
    pub(crate) async fn list_visible(
        &self,
        scope: &AccessScope,
    ) -> Result<Vec<SavedReport>, AppError> {
        let mut builder = QueryBuilder::new(format!(
            "SELECT {REPORT_COLUMNS} FROM reports WHERE (visibility = 'organization' OR owner_id = "
        ));
        builder.push_bind(scope.user_id().as_str().to_owned());
        if !scope.team_ids().is_empty() {
            builder.push(" OR (visibility = 'team' AND team_id IN (");
            let mut teams = builder.separated(", ");
            for team in scope.team_ids() {
                teams.push_bind(team.as_str().to_owned());
            }
            builder.push("))");
        }
        builder.push(") ORDER BY name, id");
        let rows = builder.build().fetch_all(&self.pool).await?;
        rows.iter().map(report_from_row).collect()
    }

    pub(crate) async fn find(&self, report_id: Uuid) -> Result<Option<SavedReport>, AppError> {
        let row = sqlx::query(sqlx::AssertSqlSafe(format!(
            "SELECT {REPORT_COLUMNS} FROM reports WHERE id = ?"
        )))
        .bind(report_id.to_string())
        .fetch_optional(&self.pool)
        .await?;
        row.as_ref().map(report_from_row).transpose()
    }

    pub(crate) async fn team_exists(&self, team_id: &TeamId) -> Result<bool, AppError> {
        Ok(
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM teams WHERE id = ?)")
                .bind(team_id.as_str())
                .fetch_one(&self.pool)
                .await?,
        )
    }

    pub(crate) async fn insert(
        &self,
        tx: &mut MySqlTx,
        report: &SavedReport,
    ) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO reports (id, record_type, name, description, owner_id, visibility, \
             team_id, visualization, definition, settings, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(report.id.to_string())
        .bind(report.record_type.as_str())
        .bind(&report.name)
        .bind(report.description.as_deref())
        .bind(report.owner_id.as_ref().map(UserId::as_str))
        .bind(report.visibility.as_str())
        .bind(report.team_id.as_ref().map(TeamId::as_str))
        .bind(report.visualization.as_str())
        .bind(to_json(&report.definition)?)
        .bind(to_json(&report.settings)?)
        .bind(mapper::naive(report.created_at))
        .bind(mapper::naive(report.updated_at))
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub(crate) async fn update(
        &self,
        tx: &mut MySqlTx,
        report: &SavedReport,
    ) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE reports SET record_type = ?, name = ?, description = ?, visibility = ?, team_id = ?, \
             visualization = ?, definition = ?, settings = ?, updated_at = ? \
             WHERE id = ?",
        )
        .bind(report.record_type.as_str())
        .bind(&report.name)
        .bind(report.description.as_deref())
        .bind(report.visibility.as_str())
        .bind(report.team_id.as_ref().map(TeamId::as_str))
        .bind(report.visualization.as_str())
        .bind(to_json(&report.definition)?)
        .bind(to_json(&report.settings)?)
        .bind(mapper::naive(report.updated_at))
        .bind(report.id.to_string())
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub(crate) async fn delete(&self, tx: &mut MySqlTx, report_id: Uuid) -> Result<(), AppError> {
        sqlx::query("DELETE FROM reports WHERE id = ?")
            .bind(report_id.to_string())
            .execute(&mut **tx)
            .await?;
        Ok(())
    }
}
