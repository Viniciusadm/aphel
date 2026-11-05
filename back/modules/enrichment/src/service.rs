use aphel_contract_crm::{CrmPermissions, CrmRecordTypes};
use std::collections::BTreeSet;
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde_json::{Map, Value, json};
use uuid::Uuid;

use crate::sources::{self, CNPJ_NOT_FOUND, Fetched, SOURCE_RATE_LIMITED};
use crate::store::{self, LookupRow, NewFieldSource};
use crate::support::{self, actor_id, allow};
use crate::{
    CNPJ_FIELD_SETTING, COMPANY, COMPANY_ENRICHED, DATA_ITEMS, KEY, SOURCE_SETTING, USE, cnpj,
    data_item,
};
use aphel_kernel::context::Ctx;
use aphel_kernel::errors::{AppError, FieldViolation};
use aphel_sdk::platform::application::access::Authorizer;
use aphel_sdk::platform::application::shared::{
    Actor, AuditEntry, AuditLog, DomainEvent, EventPublisher, FieldChange, RecordRef, RecordType,
};
use aphel_sdk::platform::domain::access::Permission;
use aphel_sdk::platform::modules::module_settings;

pub(crate) const COMPANY_NOT_FOUND: &str = "company_not_found";
pub(crate) const CNPJ_MISSING: &str = "enrichment_cnpj_missing";
pub(crate) const INVALID_CNPJ: &str = "enrichment_invalid_cnpj";
pub(crate) const LOOKUP_NOT_FOUND: &str = "enrichment_lookup_not_found";
pub(crate) const LOOKUP_EXPIRED: &str = "enrichment_lookup_expired";
pub(crate) const NOTHING_TO_APPLY: &str = "enrichment_nothing_to_apply";
pub(crate) const CNPJ_ITEM: &str = "cnpj";

const MINUTE: Duration = Duration::from_secs(60);
const ORGANIZATION_LOOKUPS_PER_MINUTE: u32 = 60;
const USER_LOOKUPS_PER_MINUTE: u32 = 15;
const RECEITAWS_PER_MINUTE: u32 = 3;
const LOOKUP_VALIDITY_MINUTES: i64 = 60;
const LOOKUP_RETENTION_HOURS: i64 = 24;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ProposalItem {
    pub(crate) data_key: String,
    pub(crate) label: String,
    pub(crate) field_key: Option<String>,
    pub(crate) field_label: Option<String>,
    pub(crate) current: Value,
    pub(crate) proposed: String,
    pub(crate) changed: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Lookup {
    pub(crate) id: Uuid,
    pub(crate) company_id: Uuid,
    pub(crate) cnpj: String,
    pub(crate) source: String,
    pub(crate) fetched_at: DateTime<Utc>,
    pub(crate) items: Vec<ProposalItem>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Applied {
    pub(crate) company_id: Uuid,
    pub(crate) source: String,
    pub(crate) cnpj: String,
    pub(crate) fields: Vec<String>,
}

pub(crate) fn company_not_found() -> AppError {
    AppError::not_found_code(COMPANY_NOT_FOUND, "empresa não encontrada")
}

fn setting_text(settings: &Value, key: &str) -> Option<String> {
    settings
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

fn mapping(settings: &Value) -> Vec<(String, String, &'static str)> {
    let mut used = BTreeSet::new();
    let mut result = Vec::new();
    if let Some(field) = setting_text(settings, CNPJ_FIELD_SETTING) {
        used.insert(field.clone());
        result.push((CNPJ_ITEM.to_owned(), field, "CNPJ"));
    }
    for item in DATA_ITEMS {
        if let Some(field) = setting_text(settings, item.setting)
            && used.insert(field.clone())
        {
            result.push((item.key.to_owned(), field, item.label));
        }
    }
    result
}

fn current_value(record: &Value, field_key: &str) -> Value {
    match record.get(field_key) {
        Some(value) if !value.is_null() => value.clone(),
        _ => record
            .pointer(&format!("/custom_fields/{field_key}"))
            .cloned()
            .unwrap_or(Value::Null),
    }
}

fn is_blank(value: &Value) -> bool {
    match value {
        Value::Null => true,
        Value::String(text) => text.trim().is_empty(),
        _ => false,
    }
}

fn same_value(current: &Value, proposed: &str) -> bool {
    match current {
        Value::String(text) => text.trim() == proposed.trim(),
        Value::Number(number) => number.to_string() == proposed,
        _ => false,
    }
}

pub(crate) async fn load_company(ctx: &Ctx<'_>, company_id: Uuid) -> Result<Value, AppError> {
    support::load_record(ctx, COMPANY, company_id)
        .await?
        .ok_or_else(company_not_found)
}

pub(crate) async fn settings(ctx: &Ctx<'_>) -> Result<Value, AppError> {
    module_settings(ctx.pool(), KEY).await
}

async fn limit(key: &str, max: u32) -> Result<(), AppError> {
    if allow(KEY, key, max, MINUTE).await? {
        Ok(())
    } else {
        Err(AppError::too_many_requests_code(
            "rate_limited",
            "muitas consultas em sequência; aguarde um minuto e tente novamente",
        ))
    }
}

fn error_code(error: &AppError) -> Option<&'static str> {
    match error {
        AppError::Coded { code, .. }
        | AppError::CodedDetails { code, .. }
        | AppError::CodedFields { code, .. } => Some(code),
        _ => None,
    }
}

pub(crate) fn is_nothing_to_apply(error: &AppError) -> bool {
    error_code(error) == Some(NOTHING_TO_APPLY)
}

async fn fetch(mode: &str, cnpj_value: &str) -> Result<Fetched, AppError> {
    let mut last_error = None;
    for source in sources::chain(mode) {
        if source.key() == sources::RECEITAWS
            && !allow(KEY, "receitaws", RECEITAWS_PER_MINUTE, MINUTE).await?
        {
            last_error = Some(AppError::too_many_requests_code(
                SOURCE_RATE_LIMITED,
                "a ReceitaWS aceita poucas consultas por minuto; tente novamente em instantes",
            ));
            continue;
        }
        match source.fetch(cnpj_value).await {
            Ok(data) => {
                return Ok(Fetched {
                    source: source.key(),
                    data,
                });
            }
            Err(error) => {
                let keep_previous = last_error.as_ref().is_some_and(|previous: &AppError| {
                    error_code(previous) == Some(CNPJ_NOT_FOUND)
                });
                if !keep_previous {
                    last_error = Some(error);
                }
            }
        }
    }
    Err(last_error.unwrap_or_else(|| AppError::Internal("no company data source".to_owned())))
}

pub(crate) async fn lookup(
    ctx: &Ctx<'_>,
    company_id: Uuid,
    typed_cnpj: Option<String>,
) -> Result<Lookup, AppError> {
    ctx.scope.require(&USE)?;
    ctx.scope.require(&Permission::COMPANIES_READ)?;
    let record = load_company(ctx, company_id).await?;
    let settings = settings(ctx).await?;
    let raw = typed_cnpj
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
        .or_else(|| {
            setting_text(&settings, CNPJ_FIELD_SETTING).and_then(|field| {
                current_value(&record, &field)
                    .as_str()
                    .map(str::to_owned)
                    .filter(|value| !value.trim().is_empty())
            })
        })
        .ok_or_else(|| {
            AppError::invalid_fields_code(
                CNPJ_MISSING,
                "informe o CNPJ da empresa",
                vec![FieldViolation::new("cnpj", "informe o CNPJ")],
            )
        })?;
    let normalized = cnpj::normalize(&raw).ok_or_else(|| {
        AppError::invalid_fields_code(
            INVALID_CNPJ,
            "CNPJ inválido",
            vec![FieldViolation::new("cnpj", "CNPJ inválido")],
        )
    })?;
    limit("lookup", ORGANIZATION_LOOKUPS_PER_MINUTE).await?;
    limit(
        &format!("lookup_user:{}", actor_id(ctx.scope)),
        USER_LOOKUPS_PER_MINUTE,
    )
    .await?;
    let mode = setting_text(&settings, SOURCE_SETTING)
        .unwrap_or_else(|| sources::BRASILAPI_THEN_RECEITAWS.to_owned());
    let fetched = fetch(&mode, &normalized).await?;
    let now = Utc::now();
    let row = LookupRow {
        id: Uuid::now_v7(),
        record_id: company_id,
        cnpj: normalized.clone(),
        source: fetched.source.to_owned(),
        data: fetched.data,
        fetched_at: now,
    };
    store::delete_old_lookups(
        ctx.pool(),
        now - chrono::Duration::hours(LOOKUP_RETENTION_HOURS),
    )
    .await?;
    store::insert_lookup(ctx.pool(), COMPANY, &row, actor_id(ctx.scope)).await?;
    let items = proposal(ctx, &settings, &record, &row).await?;
    Ok(Lookup {
        id: row.id,
        company_id,
        cnpj: normalized,
        source: row.source,
        fetched_at: row.fetched_at,
        items,
    })
}

async fn proposal(
    ctx: &Ctx<'_>,
    settings: &Value,
    record: &Value,
    row: &LookupRow,
) -> Result<Vec<ProposalItem>, AppError> {
    let fields = support::fields(ctx, COMPANY).await?;
    let fields: Vec<_> = fields.active().collect();
    let mapped = mapping(settings);
    let mut items = Vec::new();
    for (data_key, label) in std::iter::once((CNPJ_ITEM, "CNPJ"))
        .chain(DATA_ITEMS.iter().map(|item| (item.key, item.label)))
    {
        let Some(proposed) = row.data.get(data_key) else {
            continue;
        };
        let target = mapped
            .iter()
            .find(|(key, _, _)| key == data_key)
            .and_then(|(_, field_key, _)| fields.iter().find(|field| field.key() == field_key));
        let current = target
            .map(|field| current_value(record, field.key()))
            .unwrap_or(Value::Null);
        let changed = target.is_some() && !same_value(&current, proposed);
        items.push(ProposalItem {
            data_key: data_key.to_owned(),
            label: label.to_owned(),
            field_key: target.map(|field| field.key().to_owned()),
            field_label: target.map(|field| field.definition.label.clone()),
            current,
            proposed: proposed.clone(),
            changed,
        });
    }
    Ok(items)
}

pub(crate) async fn apply(
    ctx: &Ctx<'_>,
    lookup_id: Uuid,
    selected: &[String],
    only_blank: bool,
) -> Result<Applied, AppError> {
    ctx.scope.require(&USE)?;
    let lookup = store::find_lookup(ctx.pool(), COMPANY, lookup_id)
        .await?
        .ok_or_else(|| AppError::not_found_code(LOOKUP_NOT_FOUND, "consulta não encontrada"))?;
    if Utc::now() - lookup.fetched_at > chrono::Duration::minutes(LOOKUP_VALIDITY_MINUTES) {
        return Err(AppError::validation_code(
            LOOKUP_EXPIRED,
            "a consulta expirou; consulte o CNPJ novamente",
        ));
    }
    let record = load_company(ctx, lookup.record_id).await?;
    let settings = settings(ctx).await?;
    let fields = support::fields(ctx, COMPANY).await?;
    let mapped = mapping(&settings);
    let mut values = Map::new();
    let mut changes = Vec::new();
    let mut applied = Vec::new();
    for key in selected {
        if key != CNPJ_ITEM && data_item(key).is_none() {
            continue;
        }
        let Some((_, field_key, _)) = mapped.iter().find(|(data_key, _, _)| data_key == key) else {
            continue;
        };
        if fields.find_active(field_key).is_none() || values.contains_key(field_key) {
            continue;
        }
        let Some(proposed) = lookup.data.get(key) else {
            continue;
        };
        let current = current_value(&record, field_key);
        if same_value(&current, proposed) || (only_blank && !is_blank(&current)) {
            continue;
        }
        values.insert(field_key.clone(), json!(proposed));
        changes.push(json!({ "field": field_key, "old": current, "new": proposed }));
        applied.push((key.clone(), field_key.clone(), proposed.clone()));
    }
    if values.is_empty() {
        return Err(AppError::validation_code(
            NOTHING_TO_APPLY,
            "nenhum campo para preencher: escolha dados com campo mapeado e valor diferente do atual",
        ));
    }
    let mut tx = ctx.begin().await?;
    let pending =
        support::update_record_in_tx(ctx, &mut tx, COMPANY, lookup.record_id, values).await?;
    let company = RecordRef::new(RecordType::COMPANY, lookup.record_id);
    let now = Utc::now();
    let applied_by = actor_id(ctx.scope);
    for (data_key, field_key, value) in &applied {
        store::upsert_field_source(
            &mut tx,
            &NewFieldSource {
                record_type: COMPANY,
                record_id: lookup.record_id,
                field_key,
                data_key,
                source: &lookup.source,
                source_reference: &lookup.cnpj,
                value_snapshot: value,
                fetched_at: lookup.fetched_at,
                applied_by,
                applied_at: now,
            },
        )
        .await?;
    }
    let field_keys: Vec<String> = applied.iter().map(|(_, field, _)| field.clone()).collect();
    let actor = Actor::from_scope(ctx.scope);
    ctx.events()
        .publish(
            &mut tx,
            DomainEvent::new(actor.clone(), COMPANY_ENRICHED, now)
                .with_record(company.clone())
                .with_payload(json!({
                    "company_id": lookup.record_id,
                    "cnpj": cnpj::format(&lookup.cnpj),
                    "source": lookup.source,
                    "fetched_at": lookup.fetched_at.to_rfc3339(),
                    "fields": field_keys,
                    "changes": changes,
                })),
        )
        .await?;
    let mut audit_changes: Vec<FieldChange> = changes
        .iter()
        .map(|change| {
            FieldChange::new(
                change["field"].as_str().unwrap_or_default(),
                change["old"].clone(),
                change["new"].clone(),
            )
        })
        .collect();
    audit_changes.push(FieldChange::new(
        "enrichment_source",
        Value::Null,
        json!(lookup.source),
    ));
    ctx.audit()
        .record(
            &mut tx,
            AuditEntry::new(actor, COMPANY_ENRICHED, now)
                .with_record(company)
                .with_changes(audit_changes),
        )
        .await?;
    tx.commit().await?;
    if let Some(pending) = pending {
        pending.committed(ctx).await;
    }
    Ok(Applied {
        company_id: lookup.record_id,
        source: lookup.source,
        cnpj: lookup.cnpj,
        fields: field_keys,
    })
}

pub(crate) fn all_item_keys() -> Vec<String> {
    std::iter::once(CNPJ_ITEM)
        .chain(DATA_ITEMS.iter().map(|item| item.key))
        .map(str::to_owned)
        .collect()
}
