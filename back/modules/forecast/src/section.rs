use std::sync::Arc;

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};
use sqlx::MySqlPool;

use crate::domain::{ForecastSettings, UpdateForecastSettingsCommand};
use crate::service::{merge_settings, settings_snapshot, write_settings};
use crate::store::ForecastStore;
use ::config_packages::application::{
    ApplyContext, ConfigPackage, ConfigSection, Decisions, PackageIssue, PlanChange, PlanItem,
    parse_section,
};
use aphel_kernel::Ctx;
use aphel_kernel::application::fields::RecordTypeRegistry;
use aphel_kernel::database::transaction::MySqlTx;
use aphel_kernel::domain::access::AccessScope;
use aphel_kernel::errors::AppError;
use aphel_kernel::infrastructure::audit::MySqlAuditLog;
use aphel_kernel::infrastructure::events::MySqlEventPublisher;

pub(crate) const FORECAST_SECTION: &str = "forecast";
const SETTINGS_KIND: &str = "forecast_settings";
const SETTINGS_KEY: &str = "settings";

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct ForecastSectionContent {
    method: Option<String>,
    categories_enabled: Option<bool>,
    best_case_min_probability: Option<i64>,
    commit_min_probability: Option<i64>,
    horizon_months: Option<i64>,
}

impl From<ForecastSectionContent> for UpdateForecastSettingsCommand {
    fn from(content: ForecastSectionContent) -> Self {
        Self {
            method: content.method,
            categories_enabled: content.categories_enabled,
            best_case_min_probability: content.best_case_min_probability,
            commit_min_probability: content.commit_min_probability,
            horizon_months: content.horizon_months,
            ip: None,
            expected_updated_at: None,
        }
    }
}

pub(crate) struct ForecastSection {
    pool: MySqlPool,
    registry: Arc<RecordTypeRegistry>,
}

impl ForecastSection {
    pub(crate) fn new(pool: MySqlPool, registry: Arc<RecordTypeRegistry>) -> Self {
        Self { pool, registry }
    }

    async fn current(&self) -> Result<ForecastSettings, AppError> {
        Ok(ForecastStore::new(self.pool.clone())
            .settings()
            .await?
            .unwrap_or_default())
    }

    fn proposed(
        current: &ForecastSettings,
        content: &Value,
    ) -> Result<ForecastSettings, Vec<PackageIssue>> {
        let parsed: ForecastSectionContent = parse_section(content).map_err(|issue| vec![issue])?;
        merge_settings(current, &parsed.into()).map_err(|violations| {
            violations
                .into_iter()
                .map(|violation| {
                    PackageIssue::new(
                        violation.field,
                        "invalid_forecast_settings",
                        violation.message,
                    )
                })
                .collect()
        })
    }
}

#[async_trait]
impl ConfigSection<MySqlTx> for ForecastSection {
    fn key(&self) -> &'static str {
        FORECAST_SECTION
    }

    async fn export(&self, _scope: &AccessScope) -> Result<Option<Value>, AppError> {
        let current = self.current().await?;
        if current.updated_at.is_none() {
            return Ok(None);
        }
        Ok(Some(settings_snapshot(&current)))
    }

    async fn validate(
        &self,
        _scope: &AccessScope,
        _package: &ConfigPackage,
        content: &Value,
    ) -> Result<Vec<PackageIssue>, AppError> {
        let current = self.current().await?;
        Ok(Self::proposed(&current, content).err().unwrap_or_default())
    }

    async fn plan(
        &self,
        _scope: &AccessScope,
        _package: &ConfigPackage,
        content: &Value,
    ) -> Result<Vec<PlanItem>, AppError> {
        let current = self.current().await?;
        let Ok(proposed) = Self::proposed(&current, content) else {
            return Ok(Vec::new());
        };
        let before = settings_snapshot(&current);
        let after = settings_snapshot(&proposed);
        let changes = after
            .as_object()
            .into_iter()
            .flatten()
            .filter(|(property, value)| before.get(property.as_str()) != Some(value))
            .map(|(property, value)| {
                PlanChange::new(
                    property.clone(),
                    before
                        .get(property.as_str())
                        .cloned()
                        .unwrap_or(Value::Null),
                    value.clone(),
                )
            })
            .collect();
        Ok(vec![
            PlanItem::new(
                FORECAST_SECTION,
                SETTINGS_KIND,
                SETTINGS_KEY,
                "Previsão de receita",
            )
            .compared(changes),
        ])
    }

    async fn apply(
        &self,
        tx: &mut MySqlTx,
        scope: &AccessScope,
        _package: &ConfigPackage,
        content: &Value,
        _decisions: &Decisions,
        _context: &mut ApplyContext,
    ) -> Result<(), AppError> {
        let current = self.current().await?;
        let proposed = Self::proposed(&current, content).map_err(|issues| {
            AppError::Validation(
                issues
                    .first()
                    .map(|issue| issue.message.clone())
                    .unwrap_or_else(|| json!(issues.len()).to_string()),
            )
        })?;
        let ctx = Ctx::from_parts(
            scope,
            self.pool.clone(),
            self.registry.clone(),
            Arc::new(MySqlEventPublisher::new()),
            Arc::new(MySqlAuditLog::new()),
        );
        write_settings(&ctx, tx, &current, proposed, None).await?;
        Ok(())
    }
}
