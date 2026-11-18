use std::collections::BTreeSet;

use async_trait::async_trait;
use serde_json::{Value, json};

use crate::domain::{MAX_ENABLED_CURRENCIES, normalize_currency_code};
use crate::store::CurrencyStore;
use aphel_kernel::errors::AppError;
use aphel_sdk::configuration::{
    ApplyContext, ConfigPackage, ConfigSection, Decisions, PackageIssue, PlanChange, PlanItem,
    parse_section,
};
use aphel_sdk::platform::database::Db;
use aphel_sdk::platform::database::transaction::Tx;
use aphel_sdk::platform::domain::access::AccessScope;

pub(crate) const CURRENCIES_SECTION: &str = "currencies";
const CURRENCY_KIND: &str = "currency";

pub(crate) struct CurrenciesSection {
    currencies: CurrencyStore,
}

impl CurrenciesSection {
    pub(crate) fn new(pool: Db) -> Self {
        Self {
            currencies: CurrencyStore::new(pool),
        }
    }

    async fn missing(&self, content: &Value) -> Result<Vec<(String, bool)>, AppError> {
        let requested: Vec<String> =
            parse_section(content).map_err(|issue| AppError::Validation(issue.message))?;
        let base = self.currencies.context().await?.base;
        let enabled: BTreeSet<String> = self
            .currencies
            .enabled()
            .await?
            .into_iter()
            .map(|currency| currency.code)
            .collect();
        let mut seen = BTreeSet::new();
        Ok(requested
            .iter()
            .filter_map(|code| normalize_currency_code(code))
            .filter(|code| *code != base && seen.insert(code.clone()))
            .map(|code| {
                let present = enabled.contains(&code);
                (code, !present)
            })
            .collect())
    }
}

#[async_trait]
impl ConfigSection<Tx> for CurrenciesSection {
    fn key(&self) -> &'static str {
        CURRENCIES_SECTION
    }

    async fn export(&self, _scope: &AccessScope) -> Result<Option<Value>, AppError> {
        let base = self.currencies.context().await?.base;
        let mut codes: Vec<String> = self
            .currencies
            .enabled()
            .await?
            .into_iter()
            .map(|currency| currency.code)
            .filter(|code| *code != base)
            .collect();
        if codes.is_empty() {
            return Ok(None);
        }
        codes.sort();
        Ok(Some(json!(codes)))
    }

    async fn validate(
        &self,
        _scope: &AccessScope,
        _package: &ConfigPackage,
        content: &Value,
    ) -> Result<Vec<PackageIssue>, AppError> {
        let codes: Vec<String> = match parse_section(content) {
            Ok(codes) => codes,
            Err(issue) => return Ok(vec![issue]),
        };
        let mut issues = Vec::new();
        if codes.len() > MAX_ENABLED_CURRENCIES {
            issues.push(PackageIssue::new(
                "",
                "too_many_currencies",
                format!("a package can enable at most {MAX_ENABLED_CURRENCIES} currencies"),
            ));
        }
        let mut seen = BTreeSet::new();
        for (index, code) in codes.iter().enumerate() {
            match normalize_currency_code(code) {
                None => issues.push(PackageIssue::new(
                    index.to_string(),
                    "invalid_currency",
                    "must be an ISO 4217 code with three letters",
                )),
                Some(code) if !seen.insert(code.clone()) => issues.push(PackageIssue::new(
                    index.to_string(),
                    "duplicated_currency",
                    "is declared more than once",
                )),
                Some(_) => {}
            }
        }
        Ok(issues)
    }

    async fn plan(
        &self,
        _scope: &AccessScope,
        _package: &ConfigPackage,
        content: &Value,
    ) -> Result<Vec<PlanItem>, AppError> {
        Ok(self
            .missing(content)
            .await?
            .into_iter()
            .map(|(code, missing)| {
                let item = PlanItem::new(CURRENCIES_SECTION, CURRENCY_KIND, code.clone(), code);
                if missing {
                    item.created(vec![PlanChange::new("enabled", Value::Null, json!(true))])
                } else {
                    item.compared(Vec::new())
                }
            })
            .collect())
    }

    async fn apply(
        &self,
        tx: &mut Tx,
        scope: &AccessScope,
        _package: &ConfigPackage,
        content: &Value,
        _decisions: &Decisions,
        context: &mut ApplyContext,
    ) -> Result<(), AppError> {
        for (code, missing) in self.missing(content).await? {
            if missing {
                self.currencies
                    .enable(tx, &code, Some(scope.user_id()), context.at)
                    .await?;
            }
        }
        Ok(())
    }
}
