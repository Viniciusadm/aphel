use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::store::DealItemStore;
use aphel_kernel::application::config_packages::{
    ApplyContext, ConfigPackage, ConfigSection, Decisions, PackageIssue, PlanChange, PlanItem,
    parse_section,
};
use aphel_kernel::database::transaction::MySqlTx;
use aphel_kernel::domain::access::AccessScope;
use aphel_kernel::errors::AppError;
use products::domain::DealAmountMode;

pub(crate) const DEAL_ITEMS_SECTION: &str = "deal_items";
const SETTINGS_KIND: &str = "deal_item_settings";
const SETTINGS_KEY: &str = "settings";
const SETTINGS_LABEL: &str = "Itens da oportunidade";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DealItemsContent {
    amount_mode: Option<String>,
}

fn parse_content(content: &Value) -> Result<Option<DealAmountMode>, PackageIssue> {
    let content: DealItemsContent = parse_section(content)?;
    content
        .amount_mode
        .map(|mode| {
            DealAmountMode::parse(mode.trim()).ok_or_else(|| {
                PackageIssue::new(
                    "amount_mode",
                    "invalid_amount_mode",
                    "must be manual or items",
                )
            })
        })
        .transpose()
}

pub(crate) struct DealItemsSection {
    store: DealItemStore,
}

impl DealItemsSection {
    pub(crate) fn new(pool: sqlx::MySqlPool) -> Self {
        Self {
            store: DealItemStore::new(pool),
        }
    }

    async fn current_mode(&self) -> Result<DealAmountMode, AppError> {
        Ok(self
            .store
            .settings()
            .await?
            .map(|settings| settings.amount_mode)
            .unwrap_or_default())
    }

    async fn change(&self, content: &Value) -> Result<Option<PlanChange>, AppError> {
        let requested =
            parse_content(content).map_err(|issue| AppError::Validation(issue.message))?;
        let current = self.current_mode().await?;
        Ok(requested.filter(|mode| *mode != current).map(|mode| {
            PlanChange::new("amount_mode", json!(current.as_str()), json!(mode.as_str()))
        }))
    }
}

#[async_trait]
impl ConfigSection<MySqlTx> for DealItemsSection {
    fn key(&self) -> &'static str {
        DEAL_ITEMS_SECTION
    }

    async fn export(&self, _scope: &AccessScope) -> Result<Option<Value>, AppError> {
        Ok(self
            .store
            .settings()
            .await?
            .map(|settings| json!({ "amount_mode": settings.amount_mode.as_str() })))
    }

    async fn validate(
        &self,
        _scope: &AccessScope,
        _package: &ConfigPackage,
        content: &Value,
    ) -> Result<Vec<PackageIssue>, AppError> {
        Ok(parse_content(content).err().into_iter().collect())
    }

    async fn plan(
        &self,
        _scope: &AccessScope,
        _package: &ConfigPackage,
        content: &Value,
    ) -> Result<Vec<PlanItem>, AppError> {
        let item = PlanItem::new(
            DEAL_ITEMS_SECTION,
            SETTINGS_KIND,
            SETTINGS_KEY,
            SETTINGS_LABEL,
        );
        Ok(vec![item.compared(
            self.change(content).await?.into_iter().collect(),
        )])
    }

    async fn apply(
        &self,
        tx: &mut MySqlTx,
        scope: &AccessScope,
        _package: &ConfigPackage,
        content: &Value,
        _decisions: &Decisions,
        context: &mut ApplyContext,
    ) -> Result<(), AppError> {
        if self.change(content).await?.is_none() {
            return Ok(());
        }
        let Some(mode) =
            parse_content(content).map_err(|issue| AppError::Validation(issue.message))?
        else {
            return Ok(());
        };
        let mut settings = self.store.settings().await?.unwrap_or_default();
        settings.amount_mode = mode;
        settings.updated_at = Some(context.at);
        self.store
            .save_settings(tx, scope.user_id(), &settings, context.at)
            .await
    }
}
