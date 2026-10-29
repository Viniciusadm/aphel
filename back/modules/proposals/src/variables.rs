use std::collections::{BTreeSet, HashMap};

use chrono::NaiveDate;
use chrono_tz::Tz;
use serde::Serialize;
use serde_json::Value;
use utoipa::ToSchema;

use crate::records::{self, FieldInfo, is_tolerated};
use aphel_kernel::context::Ctx;
use aphel_kernel::errors::AppError;

use crate::format::{self, Locale};
use crate::markup::{ITEMS_TABLE, Issue, PAGE_BREAK, Template};

pub(crate) const RECORD_GROUPS: [&str; 3] = ["deal", "contact", "company"];

const QUOTE_VARIABLES: [(&str, &str, Option<&str>); 11] = [
    ("quote.number", "Número da cotação", None),
    ("quote.version", "Versão da cotação", None),
    ("quote.title", "Título da cotação", Some("text")),
    ("quote.valid_until", "Validade da cotação", Some("date")),
    ("quote.terms", "Condições da cotação", Some("long_text")),
    ("quote.notes", "Observações da cotação", Some("long_text")),
    ("quote.subtotal", "Subtotal da cotação", None),
    ("quote.discount_total", "Descontos da cotação", None),
    ("quote.tax_total", "Impostos da cotação", None),
    ("quote.total", "Total da cotação", None),
    ("quote.public_url", "Link público da cotação", None),
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
#[schema(as = ProposalVariable)]
pub(crate) struct VariableEntry {
    pub(crate) key: String,
    pub(crate) label: String,
    pub(crate) group: String,
    pub(crate) field_type: Option<String>,
    pub(crate) block: bool,
}

impl VariableEntry {
    fn new(group: &str, key: &str, label: &str, field_type: Option<&str>) -> Self {
        Self {
            key: key.to_owned(),
            label: label.to_owned(),
            group: group.to_owned(),
            field_type: field_type.map(str::to_owned),
            block: false,
        }
    }

    fn block(key: &str, label: &str) -> Self {
        Self {
            block: true,
            ..Self::new("blocks", key, label, None)
        }
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct Catalog {
    pub(crate) entries: Vec<VariableEntry>,
    keys: BTreeSet<String>,
    field_types: HashMap<String, String>,
}

impl Catalog {
    fn push(&mut self, entry: VariableEntry) {
        if self.keys.insert(entry.key.clone()) {
            if let Some(field_type) = &entry.field_type {
                self.field_types
                    .insert(entry.key.clone(), field_type.clone());
            }
            self.entries.push(entry);
        }
    }

    pub(crate) fn contains(&self, key: &str) -> bool {
        self.keys.contains(key)
    }

    pub(crate) fn field_type(&self, key: &str) -> Option<&str> {
        self.field_types.get(key).map(String::as_str)
    }

    pub(crate) fn validate(&self, template: &Template) -> Vec<Issue> {
        let mut issues = template.issues.clone();
        for (path, line, column) in template.variables() {
            if !self.contains(path) {
                issues.push(Issue {
                    line,
                    column,
                    code: "unknown_variable",
                    variable: Some(path.to_owned()),
                });
            }
        }
        issues.sort_by_key(|issue| (issue.line, issue.column));
        issues
    }
}

async fn record_fields(ctx: &Ctx<'_>, record_type: &str) -> Result<Vec<FieldInfo>, AppError> {
    match records::fields(ctx, record_type).await {
        Ok(fields) => Ok(fields),
        Err(error) if is_tolerated(&error) => Ok(Vec::new()),
        Err(error) => Err(error),
    }
}

pub(crate) async fn catalog(ctx: &Ctx<'_>) -> Result<Catalog, AppError> {
    let mut catalog = Catalog::default();
    catalog.push(VariableEntry::new(
        "deal",
        "deal.title",
        "Título",
        Some("text"),
    ));
    catalog.push(VariableEntry::new(
        "deal",
        "deal.amount",
        "Valor",
        Some("currency"),
    ));
    for field in record_fields(ctx, "deal").await? {
        catalog.push(VariableEntry::new(
            "deal",
            &format!("deal.{}", field.key),
            &field.label,
            Some(&field.field_type),
        ));
    }
    catalog.push(VariableEntry::new(
        "contact",
        "contact.name",
        "Nome",
        Some("text"),
    ));
    for field in record_fields(ctx, "contact").await? {
        catalog.push(VariableEntry::new(
            "contact",
            &format!("contact.{}", field.key),
            &field.label,
            Some(&field.field_type),
        ));
    }
    catalog.push(VariableEntry::new(
        "company",
        "company.name",
        "Nome",
        Some("text"),
    ));
    for field in record_fields(ctx, "company").await? {
        catalog.push(VariableEntry::new(
            "company",
            &format!("company.{}", field.key),
            &field.label,
            Some(&field.field_type),
        ));
    }
    catalog.push(VariableEntry::new(
        "owner",
        "owner.name",
        "Responsável pela oportunidade",
        None,
    ));
    catalog.push(VariableEntry::new(
        "user",
        "user.name",
        "Quem gerou a proposta",
        None,
    ));
    catalog.push(VariableEntry::new(
        "user",
        "user.email",
        "E-mail de quem gerou",
        None,
    ));
    catalog.push(VariableEntry::new(
        "organization",
        "organization.name",
        "Nome da organização",
        None,
    ));
    catalog.push(VariableEntry::new(
        "proposal",
        "proposal.number",
        "Número da proposta",
        None,
    ));
    catalog.push(VariableEntry::new(
        "proposal",
        "proposal.template",
        "Nome do modelo",
        None,
    ));
    catalog.push(VariableEntry::new(
        "proposal",
        "today",
        "Data de hoje",
        Some("date"),
    ));
    catalog.push(VariableEntry::new(
        "items",
        "items.count",
        "Quantidade de itens",
        None,
    ));
    catalog.push(VariableEntry::new(
        "items",
        "items.subtotal",
        "Subtotal dos itens",
        None,
    ));
    catalog.push(VariableEntry::new(
        "items",
        "items.discount_total",
        "Total de descontos",
        None,
    ));
    catalog.push(VariableEntry::new(
        "items",
        "items.total",
        "Total geral dos itens",
        None,
    ));
    for (key, label, field_type) in QUOTE_VARIABLES {
        catalog.push(VariableEntry::new("quote", key, label, field_type));
    }
    catalog.push(VariableEntry::block(ITEMS_TABLE, "Tabela de itens"));
    catalog.push(VariableEntry::block(PAGE_BREAK, "Quebra de página"));
    Ok(catalog)
}

#[derive(Debug, Clone, Default)]
pub(crate) struct ItemRow {
    pub(crate) name: String,
    pub(crate) unit: Option<String>,
    pub(crate) quantity: String,
    pub(crate) unit_price: String,
    pub(crate) discount_type: String,
    pub(crate) discount_value: String,
    pub(crate) discount_amount: String,
    pub(crate) total: String,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct Items {
    pub(crate) currency: String,
    pub(crate) rows: Vec<ItemRow>,
    pub(crate) subtotal: String,
    pub(crate) discount_total: String,
    pub(crate) total: String,
}

fn text(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(format::value_text)
        .unwrap_or_default()
}

impl Items {
    pub(crate) fn from_quote(value: &Value) -> Self {
        let mut items = Self::from_api(value);
        items.discount_total = text(value, "discount_total");
        items
    }

    pub(crate) fn from_api(value: &Value) -> Self {
        let rows = value
            .get("items")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .map(|item| ItemRow {
                        name: text(item, "name"),
                        unit: item.get("unit").and_then(format::value_text),
                        quantity: text(item, "quantity"),
                        unit_price: text(item, "unit_price"),
                        discount_type: text(item, "discount_type"),
                        discount_value: text(item, "discount_value"),
                        discount_amount: text(item, "discount_amount"),
                        total: text(item, "total"),
                    })
                    .collect()
            })
            .unwrap_or_default();
        Self {
            currency: text(value, "currency"),
            rows,
            subtotal: text(value, "subtotal"),
            discount_total: text(value, "discount_total"),
            total: text(value, "total"),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct RenderData {
    pub(crate) deal: Value,
    pub(crate) contact: Value,
    pub(crate) company: Value,
    pub(crate) organization_name: String,
    pub(crate) timezone: Tz,
    pub(crate) locale: Locale,
    pub(crate) currency: String,
    pub(crate) user_name: Option<String>,
    pub(crate) user_email: Option<String>,
    pub(crate) items: Option<Items>,
    pub(crate) quote: Value,
    pub(crate) number: Option<u32>,
    pub(crate) template_name: String,
    pub(crate) today: NaiveDate,
    pub(crate) sample: bool,
}

impl RenderData {
    pub(crate) fn currency(&self) -> String {
        self.items
            .as_ref()
            .map(|items| items.currency.clone())
            .filter(|currency| !currency.is_empty())
            .or_else(|| {
                self.deal
                    .get("currency")
                    .and_then(Value::as_str)
                    .map(str::to_owned)
            })
            .unwrap_or_else(|| self.currency.clone())
    }

    fn record(&self, group: &str) -> &Value {
        match group {
            "deal" => &self.deal,
            "contact" => &self.contact,
            _ => &self.company,
        }
    }

    pub(crate) fn resolve(&self, path: &str, catalog: &Catalog) -> String {
        let placeholder = || format!("[{path}]");
        let (group, key) = path.split_once('.').unwrap_or((path, ""));
        let samples = self.sample;
        let value = match (group, key) {
            ("today", "") => Some(format::date(self.today, self.locale)),
            ("organization", "name") => Some(self.organization_name.clone()),
            ("user", "name") => self.user_name.clone(),
            ("user", "email") => self.user_email.clone(),
            ("proposal", "template") => Some(self.template_name.clone()),
            ("proposal", "number") => match self.number {
                Some(number) => Some(format::proposal_number(number)),
                None => return placeholder(),
            },
            _ if samples => return placeholder(),
            ("owner", "name") => self.deal.get("owner").and_then(format::value_text),
            ("quote", key) => self.quote_value(key),
            ("items", total) => self.items_value(total),
            (group, key) if RECORD_GROUPS.contains(&group) => record_value(
                self.record(group),
                group,
                key,
                catalog.field_type(path),
                self.timezone,
                self.locale,
                &self.currency(),
            ),
            _ => None,
        };
        value.unwrap_or_default()
    }

    fn quote_value(&self, key: &str) -> Option<String> {
        if self.quote.is_null() {
            return None;
        }
        let raw = self.quote.get(key).filter(|value| !value.is_null())?;
        let currency = self.currency();
        let locale = self.locale;
        match key {
            "subtotal" | "discount_total" | "tax_total" | "total" => {
                format::value_text(raw).map(|amount| format::money(&amount, &currency, locale))
            }
            "valid_until" => format::value_text(raw)
                .map(|value| format::date_text(&value, self.timezone, locale)),
            _ => format::display_text(raw, locale),
        }
    }

    fn items_value(&self, key: &str) -> Option<String> {
        let items = self.items.as_ref()?;
        let currency = self.currency();
        let locale = self.locale;
        match key {
            "count" => Some(items.rows.len().to_string()),
            "subtotal" => Some(format::money(&items.subtotal, &currency, locale)),
            "discount_total" => Some(format::money(&items.discount_total, &currency, locale)),
            "total" => Some(format::money(&items.total, &currency, locale)),
            _ => None,
        }
    }
}

fn deal_status(value: &str, locale: Locale) -> String {
    let labels = locale.labels();
    match value {
        "open" => labels.deal_open,
        "won" => labels.deal_won,
        "lost" => labels.deal_lost,
        other => other,
    }
    .to_owned()
}

fn record_value(
    record: &Value,
    group: &str,
    key: &str,
    field_type: Option<&str>,
    timezone: Tz,
    locale: Locale,
    currency: &str,
) -> Option<String> {
    if record.is_null() || key.is_empty() {
        return None;
    }
    if let Some(related) = key.strip_suffix("_id")
        && let Some(object @ Value::Object(_)) = record.get(related)
    {
        return format::value_text(object);
    }
    let raw = record
        .get(key)
        .filter(|value| !value.is_null() && !value.is_object())
        .or_else(|| {
            record
                .get("custom_fields")
                .and_then(|fields| fields.get(key))
                .filter(|value| !value.is_null())
        })?;
    if group == "deal" && key == "status" {
        return raw.as_str().map(|status| deal_status(status, locale));
    }
    let plain = || format::display_text(raw, locale);
    match field_type {
        Some("currency") => match raw {
            Value::Object(map) => {
                let amount = map.get("amount").and_then(format::value_text)?;
                let code = map
                    .get("currency")
                    .and_then(Value::as_str)
                    .unwrap_or(currency);
                Some(format::money(&amount, code, locale))
            }
            _ => format::value_text(raw).map(|amount| format::money(&amount, currency, locale)),
        },
        Some("number") => format::value_text(raw).map(|value| format::number(&value, locale)),
        Some("date") => {
            format::value_text(raw).map(|value| format::date_text(&value, timezone, locale))
        }
        Some("datetime") => {
            format::value_text(raw).map(|value| format::datetime_text(&value, timezone, locale))
        }
        Some("checkbox") => raw
            .as_bool()
            .map(|flag| format::yes_no(flag, locale))
            .or_else(plain),
        _ => plain(),
    }
}
