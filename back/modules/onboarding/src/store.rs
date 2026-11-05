use chrono::{DateTime, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::types::Json;
use sqlx::{MySqlPool, Row};
use uuid::Uuid;

use crate::domain::{
    BusinessType, Offering, OnboardingAnswers, OnboardingState, OnboardingStatus, TeamSize,
};
use aphel_kernel::Ctx;
use aphel_kernel::domain::UserId;
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::database::transaction::MySqlTx;
use aphel_sdk::platform::infrastructure::repositories::mapper;

#[derive(Debug, Default, Serialize, Deserialize)]
struct StoredAnswers {
    #[serde(default)]
    business_type: Option<String>,
    #[serde(default)]
    offering: Vec<String>,
    #[serde(default)]
    team_size: Option<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct StoredOnboarding {
    status: String,
    #[serde(default)]
    answers: StoredAnswers,
    #[serde(default)]
    template_key: Option<String>,
    #[serde(default)]
    install_id: Option<String>,
    #[serde(default)]
    completed_by: Option<String>,
    #[serde(default)]
    completed_at: Option<String>,
    #[serde(default)]
    skipped_by: Option<String>,
    #[serde(default)]
    skipped_at: Option<String>,
    #[serde(default)]
    updated_at: Option<String>,
}

fn timestamp(value: Option<DateTime<Utc>>) -> Option<String> {
    value.map(|instant| instant.to_rfc3339_opts(SecondsFormat::Secs, true))
}

fn parse_timestamp(value: Option<String>) -> Result<Option<DateTime<Utc>>, AppError> {
    value
        .map(|raw| {
            DateTime::parse_from_rfc3339(&raw)
                .map(|instant| instant.with_timezone(&Utc))
                .map_err(|_| mapper::invalid_column("settings.onboarding", &raw))
        })
        .transpose()
}

fn parse_user(value: Option<String>) -> Result<Option<UserId>, AppError> {
    value
        .map(|id| mapper::typed_id(id, UserId::new))
        .transpose()
}

fn answers_from_stored(stored: &StoredAnswers) -> OnboardingAnswers {
    let mut offering: Vec<Offering> = stored
        .offering
        .iter()
        .filter_map(|value| Offering::parse(value))
        .collect();
    offering.sort();
    offering.dedup();
    OnboardingAnswers {
        business_type: stored
            .business_type
            .as_deref()
            .and_then(BusinessType::parse),
        offering,
        team_size: stored.team_size.as_deref().and_then(TeamSize::parse),
    }
}

impl From<&OnboardingState> for StoredOnboarding {
    fn from(state: &OnboardingState) -> Self {
        Self {
            status: state.status.as_str().to_owned(),
            answers: StoredAnswers {
                business_type: state
                    .answers
                    .business_type
                    .map(|kind| kind.as_str().to_owned()),
                offering: state
                    .answers
                    .offering
                    .iter()
                    .map(|offering| offering.as_str().to_owned())
                    .collect(),
                team_size: state.answers.team_size.map(|size| size.as_str().to_owned()),
            },
            template_key: state.template_key.clone(),
            install_id: state.install_id.map(|id| id.to_string()),
            completed_by: state.completed_by.as_ref().map(|id| id.as_str().to_owned()),
            completed_at: timestamp(state.completed_at),
            skipped_by: state.skipped_by.as_ref().map(|id| id.as_str().to_owned()),
            skipped_at: timestamp(state.skipped_at),
            updated_at: timestamp(state.updated_at),
        }
    }
}

impl TryFrom<StoredOnboarding> for OnboardingState {
    type Error = AppError;

    fn try_from(stored: StoredOnboarding) -> Result<Self, AppError> {
        let status = OnboardingStatus::parse(&stored.status)
            .ok_or_else(|| mapper::invalid_column("settings.onboarding.status", &stored.status))?;
        Ok(Self {
            status,
            answers: answers_from_stored(&stored.answers),
            installer_answers: OnboardingAnswers::default(),
            template_key: stored.template_key,
            install_id: stored
                .install_id
                .map(|id| {
                    Uuid::parse_str(&id)
                        .map_err(|_| mapper::invalid_column("settings.onboarding.install_id", &id))
                })
                .transpose()?,
            completed_by: parse_user(stored.completed_by)?,
            completed_at: parse_timestamp(stored.completed_at)?,
            skipped_by: parse_user(stored.skipped_by)?,
            skipped_at: parse_timestamp(stored.skipped_at)?,
            updated_at: parse_timestamp(stored.updated_at)?,
        })
    }
}

#[derive(Clone)]
pub(crate) struct OnboardingStore {
    pool: MySqlPool,
}

impl OnboardingStore {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    pub(crate) fn of(ctx: &Ctx<'_>) -> Self {
        Self::new(ctx.pool().clone())
    }

    pub(crate) async fn find(&self) -> Result<Option<OnboardingState>, AppError> {
        let row = sqlx::query(
            "SELECT JSON_EXTRACT(settings, '$.onboarding') AS onboarding \
             FROM organization_settings WHERE id = 1",
        )
        .fetch_optional(&self.pool)
        .await?;
        let Some(row) = row else {
            return Ok(None);
        };
        let Some(Json(document)): Option<Json<Value>> = row.try_get("onboarding")? else {
            return Ok(None);
        };
        if document.is_null() {
            return Ok(None);
        }
        let stored = serde_json::from_value::<StoredOnboarding>(document.clone())
            .map_err(|_| mapper::invalid_column("settings.onboarding", &document.to_string()))?;
        stored.try_into().map(Some)
    }

    pub(crate) async fn save(
        &self,
        tx: &mut MySqlTx,
        state: &OnboardingState,
    ) -> Result<(), AppError> {
        let document = serde_json::to_string(&StoredOnboarding::from(state)).map_err(|error| {
            AppError::Internal(format!("cannot serialize onboarding state: {error}"))
        })?;
        let result = sqlx::query(
            "UPDATE organization_settings \
             SET settings = JSON_SET(COALESCE(settings, JSON_OBJECT()), '$.onboarding', CAST(? AS JSON)) \
             WHERE id = 1",
        )
        .bind(document)
        .execute(&mut **tx)
        .await?;
        if result.rows_affected() == 0 {
            return Err(AppError::NotFound("organization was not found".to_owned()));
        }
        Ok(())
    }

    pub(crate) async fn installer_answers(&self) -> Result<OnboardingAnswers, AppError> {
        let document: Option<Option<Json<Value>>> = sqlx::query_scalar(
            "SELECT JSON_EXTRACT(settings, '$.installer') FROM organization_settings WHERE id = 1",
        )
        .fetch_optional(&self.pool)
        .await?;
        let Some(Json(document)) = document.flatten() else {
            return Ok(OnboardingAnswers::default());
        };
        let stored: StoredAnswers = serde_json::from_value(document).unwrap_or_default();
        Ok(answers_from_stored(&stored))
    }
}
