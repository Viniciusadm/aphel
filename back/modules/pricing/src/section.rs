use aphel_sdk::configuration::ConfigReferenceKind;
use std::collections::{BTreeMap, BTreeSet};

use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::MySqlPool;
use uuid::Uuid;

use crate::domain::{
    MAX_PRICE_LIST_DESCRIPTION_LENGTH, MAX_PRICE_LIST_ITEMS, MAX_PRICE_LIST_NAME_LENGTH,
    MAX_PRICE_LISTS, NewPriceListItem, PriceList, PriceListAppliesTo,
};
use crate::service::{parse_min_quantity, parse_percent};
use crate::store::{DiscountLimitStore, PriceListStore};
use aphel_kernel::database::transaction::MySqlTx;
use aphel_kernel::domain::access::AccessScope;
use aphel_kernel::errors::AppError;
use aphel_sdk::configuration::sections::common::{check_key, diff};
use aphel_sdk::configuration::{
    ApplyContext, ConfigPackage, ConfigSection, Decisions, PackageIssue, PlanChange, PlanItem,
    parse_section,
};
use aphel_sdk::line_items::parse_money;
use aphel_sdk::money::normalize_currency_code as normalize_currency;

pub(crate) const PRICE_LISTS_SECTION: &str = "price_lists";
pub(crate) const DISCOUNT_LIMITS_SECTION: &str = "discount_limits";
const PRICE_LIST_KIND: &str = "price_list";
const DISCOUNT_LIMIT_KIND: &str = "discount_limit";
const CURRENCIES_SECTION: &str = "currencies";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PackagePriceListItem {
    pub(crate) product_code: String,
    pub(crate) unit_price: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) min_quantity: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PackagePriceList {
    pub(crate) key: String,
    pub(crate) name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) description: Option<String>,
    pub(crate) currency: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) valid_from: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) valid_to: Option<String>,
    #[serde(default = "enabled")]
    pub(crate) is_active: bool,
    #[serde(default)]
    pub(crate) is_default: bool,
    #[serde(default)]
    pub(crate) items: Vec<PackagePriceListItem>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PackageDiscountLimit {
    pub(crate) role: String,
    pub(crate) max_discount_percent: Option<String>,
}

fn enabled() -> bool {
    true
}

fn parse_date(value: Option<&str>) -> Result<Option<NaiveDate>, ()> {
    match value.map(str::trim).filter(|text| !text.is_empty()) {
        None => Ok(None),
        Some(text) => NaiveDate::parse_from_str(text, "%Y-%m-%d")
            .map(Some)
            .map_err(|_| ()),
    }
}

fn date_text(date: Option<NaiveDate>) -> Value {
    date.map_or(Value::Null, |date| {
        json!(date.format("%Y-%m-%d").to_string())
    })
}

fn parse<T: for<'de> Deserialize<'de>>(content: &Value) -> Result<T, AppError> {
    parse_section(content).map_err(|issue| AppError::Validation(issue.message))
}

fn package_currencies(package: &ConfigPackage) -> BTreeSet<String> {
    package
        .sections
        .get(CURRENCIES_SECTION)
        .and_then(|value| serde_json::from_value::<Vec<String>>(value.clone()).ok())
        .unwrap_or_default()
        .iter()
        .filter_map(|code| normalize_currency(code))
        .collect()
}

fn list_snapshot(list: &PriceList, items: &[String]) -> Vec<(String, Value)> {
    vec![
        ("name".to_owned(), json!(list.name)),
        ("description".to_owned(), json!(list.description)),
        ("currency".to_owned(), json!(list.currency)),
        ("valid_from".to_owned(), date_text(list.valid_from)),
        ("valid_to".to_owned(), date_text(list.valid_to)),
        ("is_active".to_owned(), json!(list.is_active)),
        ("is_default".to_owned(), json!(list.is_default)),
        ("items".to_owned(), json!(items)),
    ]
}

fn item_signature(code: &str, min_quantity: Decimal, unit_price: Decimal) -> String {
    format!(
        "{code}:{}@{}",
        min_quantity.normalize(),
        unit_price.normalize()
    )
}

struct PriceListPlan {
    item: PlanItem,
    list: PriceList,
    items: Option<Vec<NewPriceListItem>>,
    is_new: bool,
}

pub(crate) struct PriceListsSection {
    lists: PriceListStore,
}

impl PriceListsSection {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self {
            lists: PriceListStore::new(pool),
        }
    }

    async fn reconcile(
        &self,
        scope: &AccessScope,
        package: &ConfigPackage,
        entries: &[PackagePriceList],
        at: DateTime<Utc>,
    ) -> Result<Vec<PriceListPlan>, AppError> {
        let incoming = package_currencies(package);
        let codes: Vec<String> = entries
            .iter()
            .flat_map(|entry| {
                entry
                    .items
                    .iter()
                    .map(|item| item.product_code.trim().to_owned())
            })
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        let products: BTreeMap<String, Uuid> = self
            .lists
            .products_by_code(&codes)
            .await?
            .into_iter()
            .collect();
        let mut plans = Vec::new();
        for entry in entries {
            let label = entry.name.trim().to_owned();
            let item = PlanItem::new(
                PRICE_LISTS_SECTION,
                PRICE_LIST_KIND,
                entry.key.clone(),
                label,
            );
            let currency = normalize_currency(&entry.currency).unwrap_or_default();
            let existing = self.lists.find_by_key(&entry.key).await?;
            let base = existing.clone().unwrap_or_else(|| PriceList {
                id: Uuid::now_v7(),
                key: entry.key.clone(),
                name: String::new(),
                description: None,
                currency: currency.clone(),
                applies_to: PriceListAppliesTo::All,
                segment_id: None,
                company_ids: Vec::new(),
                valid_from: None,
                valid_to: None,
                is_active: true,
                is_default: false,
                item_count: 0,
                created_by: Some(scope.user_id().clone()),
                created_at: at,
                updated_at: at,
            });
            let mut list = base.clone();
            list.name = entry.name.trim().to_owned();
            list.description = entry
                .description
                .as_deref()
                .map(str::trim)
                .filter(|text| !text.is_empty())
                .map(str::to_owned);
            list.currency = currency.clone();
            list.valid_from = parse_date(entry.valid_from.as_deref()).unwrap_or(None);
            list.valid_to = parse_date(entry.valid_to.as_deref()).unwrap_or(None);
            list.is_active = entry.is_active;
            list.is_default = entry.is_default && list.applies_to == PriceListAppliesTo::All;
            list.updated_at = at;
            let mut missing = Vec::new();
            let mut new_items = Vec::new();
            let mut proposed = BTreeSet::new();
            for (position, package_item) in entry.items.iter().enumerate() {
                let code = package_item.product_code.trim();
                let (Some(unit_price), Some(min_quantity)) = (
                    parse_money(&package_item.unit_price),
                    parse_min_quantity(package_item.min_quantity.as_deref()),
                ) else {
                    continue;
                };
                match products.get(code) {
                    None => missing.push(code.to_owned()),
                    Some(product_id) => {
                        proposed.insert(item_signature(code, min_quantity, unit_price));
                        new_items.push(NewPriceListItem {
                            id: Uuid::now_v7(),
                            product_id: *product_id,
                            min_quantity,
                            unit_price,
                            position: position as i32,
                        });
                    }
                }
            }
            let current_items: BTreeSet<String> = match &existing {
                None => BTreeSet::new(),
                Some(current) => self
                    .lists
                    .items(current.id)
                    .await?
                    .iter()
                    .map(|item| {
                        item_signature(
                            item.product_code
                                .as_deref()
                                .unwrap_or(&item.product_id.to_string()),
                            item.min_quantity,
                            item.unit_price,
                        )
                    })
                    .collect(),
            };
            let current_signature: Vec<String> = current_items.iter().cloned().collect();
            let proposed_signature: Vec<String> = proposed.iter().cloned().collect();
            let changes = match &existing {
                None => diff(&[], &list_snapshot(&list, &proposed_signature)),
                Some(current) => diff(
                    &list_snapshot(current, &current_signature),
                    &list_snapshot(&list, &proposed_signature),
                ),
            };
            let items_changed = current_items != proposed;
            let currency_known = currency == base.currency && existing.is_some()
                || incoming.contains(&currency)
                || self.lists.currency_enabled(&currency).await?;
            let item = if !currency_known {
                item.conflicting(
                    changes,
                    "currency_not_enabled",
                    format!("{currency} is not enabled for the organization"),
                    false,
                )
            } else if !missing.is_empty() {
                item.conflicting(
                    changes,
                    "products_not_found",
                    format!(
                        "no product uses the codes {}; their prices are skipped",
                        missing.join(", ")
                    ),
                    true,
                )
            } else if existing.is_none() {
                item.created(changes)
            } else {
                item.compared(changes)
            };
            plans.push(PriceListPlan {
                item,
                list,
                items: (existing.is_none() || items_changed).then_some(new_items),
                is_new: existing.is_none(),
            });
        }
        Ok(plans)
    }
}

#[async_trait]
impl ConfigSection<MySqlTx> for PriceListsSection {
    fn key(&self) -> &'static str {
        PRICE_LISTS_SECTION
    }

    async fn export(&self, _scope: &AccessScope) -> Result<Option<Value>, AppError> {
        let mut exported = Vec::new();
        for list in self.lists.all().await? {
            if list.applies_to != PriceListAppliesTo::All {
                continue;
            }
            let items = self
                .lists
                .items(list.id)
                .await?
                .into_iter()
                .filter_map(|item| {
                    item.product_code.map(|code| PackagePriceListItem {
                        product_code: code,
                        unit_price: item.unit_price.to_string(),
                        min_quantity: (item.min_quantity != Decimal::ONE)
                            .then(|| item.min_quantity.normalize().to_string()),
                    })
                })
                .collect();
            exported.push(PackagePriceList {
                key: list.key,
                name: list.name,
                description: list.description,
                currency: list.currency,
                valid_from: list
                    .valid_from
                    .map(|date| date.format("%Y-%m-%d").to_string()),
                valid_to: list
                    .valid_to
                    .map(|date| date.format("%Y-%m-%d").to_string()),
                is_active: list.is_active,
                is_default: list.is_default,
                items,
            });
        }
        if exported.is_empty() {
            return Ok(None);
        }
        Ok(Some(json!(exported)))
    }

    async fn validate(
        &self,
        _scope: &AccessScope,
        _package: &ConfigPackage,
        content: &Value,
    ) -> Result<Vec<PackageIssue>, AppError> {
        let lists: Vec<PackagePriceList> = match parse_section(content) {
            Ok(lists) => lists,
            Err(issue) => return Ok(vec![issue]),
        };
        let mut issues = Vec::new();
        if lists.len() as u64 > MAX_PRICE_LISTS {
            issues.push(PackageIssue::new(
                "",
                "too_many_price_lists",
                format!("a package can have at most {MAX_PRICE_LISTS} price lists"),
            ));
        }
        let mut keys = BTreeSet::new();
        let mut defaults = BTreeSet::new();
        for (index, list) in lists.iter().enumerate() {
            check_key(&list.key, &format!("{index}.key"), &mut keys, &mut issues);
            let name = list.name.trim().chars().count();
            if name == 0 || name > MAX_PRICE_LIST_NAME_LENGTH {
                issues.push(PackageIssue::new(
                    format!("{index}.name"),
                    "invalid_name",
                    format!("must have between 1 and {MAX_PRICE_LIST_NAME_LENGTH} characters"),
                ));
            }
            if list
                .description
                .as_deref()
                .is_some_and(|text| text.chars().count() > MAX_PRICE_LIST_DESCRIPTION_LENGTH)
            {
                issues.push(PackageIssue::new(
                    format!("{index}.description"),
                    "invalid_description",
                    format!("must have at most {MAX_PRICE_LIST_DESCRIPTION_LENGTH} characters"),
                ));
            }
            match normalize_currency(&list.currency) {
                None => issues.push(PackageIssue::new(
                    format!("{index}.currency"),
                    "invalid_currency",
                    "must be an ISO 4217 code with three letters",
                )),
                Some(code) if list.is_default && !defaults.insert(code.clone()) => {
                    issues.push(PackageIssue::new(
                        format!("{index}.is_default"),
                        "duplicated_default",
                        format!("another price list is already the default of {code}"),
                    ))
                }
                Some(_) => {}
            }
            let from = parse_date(list.valid_from.as_deref());
            let to = parse_date(list.valid_to.as_deref());
            for (field, value) in [("valid_from", &from), ("valid_to", &to)] {
                if value.is_err() {
                    issues.push(PackageIssue::new(
                        format!("{index}.{field}"),
                        "invalid_date",
                        "must be a date in the YYYY-MM-DD format",
                    ));
                }
            }
            if let (Ok(Some(from)), Ok(Some(to))) = (from, to)
                && from > to
            {
                issues.push(PackageIssue::new(
                    format!("{index}.valid_to"),
                    "invalid_period",
                    "must be on or after valid_from",
                ));
            }
            if list.items.len() > MAX_PRICE_LIST_ITEMS {
                issues.push(PackageIssue::new(
                    format!("{index}.items"),
                    "too_many_items",
                    format!("a price list can have at most {MAX_PRICE_LIST_ITEMS} items"),
                ));
            }
            let mut tiers = BTreeSet::new();
            for (position, item) in list.items.iter().enumerate() {
                let path = format!("{index}.items.{position}");
                if item.product_code.trim().is_empty() {
                    issues.push(PackageIssue::new(
                        format!("{path}.product_code"),
                        "invalid_product_code",
                        "is required",
                    ));
                }
                if parse_money(&item.unit_price).is_none() {
                    issues.push(PackageIssue::new(
                        format!("{path}.unit_price"),
                        "invalid_price",
                        "must be a non-negative number with at most 2 decimal places",
                    ));
                }
                match parse_min_quantity(item.min_quantity.as_deref()) {
                    None => issues.push(PackageIssue::new(
                        format!("{path}.min_quantity"),
                        "invalid_quantity",
                        "must be a number greater than zero",
                    )),
                    Some(quantity) => {
                        if !tiers.insert((
                            item.product_code.trim().to_owned(),
                            quantity.normalize().to_string(),
                        )) {
                            issues.push(PackageIssue::new(
                                format!("{path}.min_quantity"),
                                "duplicated_tier",
                                "the product already has a price for this minimum quantity",
                            ));
                        }
                    }
                }
            }
        }
        Ok(issues)
    }

    async fn plan(
        &self,
        scope: &AccessScope,
        package: &ConfigPackage,
        content: &Value,
    ) -> Result<Vec<PlanItem>, AppError> {
        Ok(self
            .reconcile(
                scope,
                package,
                &parse::<Vec<_>>(content)?,
                DateTime::<Utc>::UNIX_EPOCH,
            )
            .await?
            .into_iter()
            .map(|plan| plan.item)
            .collect())
    }

    async fn apply(
        &self,
        tx: &mut MySqlTx,
        scope: &AccessScope,
        package: &ConfigPackage,
        content: &Value,
        decisions: &Decisions,
        context: &mut ApplyContext,
    ) -> Result<(), AppError> {
        for plan in self
            .reconcile(scope, package, &parse::<Vec<_>>(content)?, context.at)
            .await?
        {
            if !decisions.executes(&plan.item) {
                continue;
            }
            if plan.list.is_default {
                self.lists
                    .clear_default(tx, &plan.list.currency, plan.list.id)
                    .await?;
            }
            if plan.is_new {
                self.lists.insert(tx, &plan.list).await?;
            } else {
                self.lists.update(tx, &plan.list).await?;
            }
            if let Some(items) = &plan.items {
                self.lists.replace_items(tx, plan.list.id, items).await?;
            }
            context.resolve(
                ConfigReferenceKind::new("pricing.price_list"),
                &plan.list.key,
                plan.list.id,
            );
        }
        Ok(())
    }
}

pub(crate) struct DiscountLimitsSection {
    limits: DiscountLimitStore,
}

impl DiscountLimitsSection {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self {
            limits: DiscountLimitStore::new(pool),
        }
    }

    async fn reconcile(
        &self,
        entries: &[PackageDiscountLimit],
    ) -> Result<Vec<(PlanItem, Option<(Uuid, Option<Decimal>)>)>, AppError> {
        let roles = self.limits.list().await?;
        Ok(entries
            .iter()
            .map(|entry| {
                let percent = entry
                    .max_discount_percent
                    .as_deref()
                    .and_then(parse_percent);
                let item = PlanItem::new(
                    DISCOUNT_LIMITS_SECTION,
                    DISCOUNT_LIMIT_KIND,
                    entry.role.clone(),
                    entry.role.clone(),
                );
                match roles.iter().find(|role| role.role_key == entry.role) {
                    None => (
                        item.conflicting(
                            Vec::new(),
                            "role_not_found",
                            format!("the organization has no role with the key {}", entry.role),
                            false,
                        ),
                        None,
                    ),
                    Some(role) => {
                        let current = role.max_discount_percent.map(|value| value.to_string());
                        let proposed = percent.map(|value| value.to_string());
                        let changes = if current == proposed {
                            Vec::new()
                        } else {
                            vec![PlanChange::new(
                                "max_discount_percent",
                                json!(current),
                                json!(proposed),
                            )]
                        };
                        (item.compared(changes), Some((role.role_id, percent)))
                    }
                }
            })
            .collect())
    }
}

#[async_trait]
impl ConfigSection<MySqlTx> for DiscountLimitsSection {
    fn key(&self) -> &'static str {
        DISCOUNT_LIMITS_SECTION
    }

    async fn export(&self, _scope: &AccessScope) -> Result<Option<Value>, AppError> {
        let limits: Vec<PackageDiscountLimit> = self
            .limits
            .list()
            .await?
            .into_iter()
            .filter_map(|role| {
                role.max_discount_percent
                    .map(|percent| PackageDiscountLimit {
                        role: role.role_key,
                        max_discount_percent: Some(percent.to_string()),
                    })
            })
            .collect();
        if limits.is_empty() {
            return Ok(None);
        }
        Ok(Some(json!(limits)))
    }

    async fn validate(
        &self,
        _scope: &AccessScope,
        _package: &ConfigPackage,
        content: &Value,
    ) -> Result<Vec<PackageIssue>, AppError> {
        let limits: Vec<PackageDiscountLimit> = match parse_section(content) {
            Ok(limits) => limits,
            Err(issue) => return Ok(vec![issue]),
        };
        let mut issues = Vec::new();
        let mut roles = BTreeSet::new();
        for (index, limit) in limits.iter().enumerate() {
            if limit.role.trim().is_empty() {
                issues.push(PackageIssue::new(
                    format!("{index}.role"),
                    "invalid_role",
                    "is required",
                ));
            } else if !roles.insert(limit.role.clone()) {
                issues.push(PackageIssue::new(
                    format!("{index}.role"),
                    "duplicated_role",
                    "is declared more than once",
                ));
            }
            if let Some(percent) = &limit.max_discount_percent
                && parse_percent(percent).is_none()
            {
                issues.push(PackageIssue::new(
                    format!("{index}.max_discount_percent"),
                    "invalid_percent",
                    "must be a number between 0 and 100 with at most 2 decimal places",
                ));
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
            .reconcile(&parse::<Vec<_>>(content)?)
            .await?
            .into_iter()
            .map(|(item, _)| item)
            .collect())
    }

    async fn apply(
        &self,
        tx: &mut MySqlTx,
        scope: &AccessScope,
        _package: &ConfigPackage,
        content: &Value,
        decisions: &Decisions,
        context: &mut ApplyContext,
    ) -> Result<(), AppError> {
        for (item, target) in self.reconcile(&parse::<Vec<_>>(content)?).await? {
            let Some((role_id, percent)) = target else {
                continue;
            };
            if decisions.executes(&item) {
                self.limits
                    .set(tx, role_id, percent, Some(scope.user_id()), context.at)
                    .await?;
            }
        }
        Ok(())
    }
}
