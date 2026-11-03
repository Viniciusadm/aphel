use std::collections::BTreeSet;
use std::str::FromStr;

use chrono::{NaiveDate, TimeZone};
use chrono_tz::Tz;
use rust_decimal::Decimal;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::domain::{
    CURRENCIES_RECALCULATE_JOB, CURRENCY_DISABLED, CURRENCY_ENABLED, CreateExchangeRateCommand,
    CurrencyContext, CurrencySettings, CurrencyView, EXCHANGE_RATE_CREATED, EXCHANGE_RATE_DELETED,
    EXCHANGE_RATE_EXISTS, EXCHANGE_RATE_NOT_FOUND, EXCHANGE_RATE_RECORD_TYPE,
    EXCHANGE_RATE_UPDATED, ExchangeRate, ExchangeRateFilter, MAX_ENABLED_CURRENCIES,
    MAX_PROVIDER_URL_LENGTH, RATE_DECIMALS, RATES_PROVIDER_NOT_CONFIGURED, RATES_PROVIDER_UPDATED,
    RATES_PROVIDERS, RateSource, RatesProviderSettings, SetCurrenciesCommand, SyncOutcome,
    UpdateExchangeRateCommand, UpdateRatesProviderCommand, currency_not_enabled,
    normalize_currency_code, normalize_rate, organization_today,
};
use crate::provider::configured_provider;
use crate::sync::sync_rates;
use std::sync::Arc;

use sqlx::MySqlPool;

use crate::store::CurrencyStore;
use aphel_kernel::Ctx;
use aphel_kernel::application::access::Authorizer;
use aphel_kernel::application::organization_admin::{ListWindow, Listing};
use aphel_kernel::application::shared::{
    Actor, AuditEntry, AuditLog, DomainEvent, EventPublisher, FieldChange, Job, JobQueue,
    RecordRef, ensure_version,
};
use aphel_kernel::database::transaction::MySqlTx;
use aphel_kernel::domain::access::{AccessScope, Permission};
use aphel_kernel::errors::{AppError, FieldViolation};
use aphel_kernel::infrastructure::audit::MySqlAuditLog;
use aphel_kernel::infrastructure::events::MySqlEventPublisher;
use aphel_kernel::infrastructure::jobs::MySqlJobQueue;
use aphel_kernel::infrastructure::platform::{
    SharedAuditLog, SharedEventPublisher, SharedJobQueue,
};

const INVALID_RATE: &str = "invalid exchange rate";

fn rate_not_found() -> AppError {
    AppError::not_found_code(EXCHANGE_RATE_NOT_FOUND, "exchange rate was not found")
}

fn rate_exists() -> AppError {
    AppError::conflict_code(
        EXCHANGE_RATE_EXISTS,
        "the currency already has a rate on this date",
    )
}

pub(crate) fn require_reader(scope: &AccessScope) -> Result<(), AppError> {
    if scope.require(&Permission::SETTINGS_MANAGE).is_ok() {
        return Ok(());
    }
    scope
        .require(&aphel_sdk::money::READ_PERMISSION)
        .map(|_| ())
}

fn parse_rate(value: &str, violations: &mut Vec<FieldViolation>) -> Option<Decimal> {
    let text = value.trim();
    let rate = Decimal::from_str(text)
        .ok()
        .or_else(|| Decimal::from_scientific(text).ok())
        .and_then(normalize_rate);
    if rate.is_none() {
        violations.push(FieldViolation::new(
            "rate",
            format!(
                "must be a number greater than zero with at most {RATE_DECIMALS} decimal places"
            ),
        ));
    }
    rate
}

fn parse_date(value: &str, violations: &mut Vec<FieldViolation>) -> Option<NaiveDate> {
    let date = NaiveDate::parse_from_str(value.trim(), "%Y-%m-%d").ok();
    if date.is_none() {
        violations.push(FieldViolation::new(
            "effective_date",
            "must be a date in the YYYY-MM-DD format",
        ));
    }
    date
}

pub(crate) fn rate_snapshot(rate: &ExchangeRate) -> Value {
    json!({
        "currency": rate.currency,
        "effective_date": rate.effective_date.format("%Y-%m-%d").to_string(),
        "rate": rate.rate.normalize().to_string(),
        "source": rate.source.as_str(),
    })
}

pub(crate) fn snapshot_changes(old: &Value, new: &Value) -> Vec<FieldChange> {
    let keys = ["currency", "effective_date", "rate", "source"];
    keys.iter()
        .filter_map(|key| {
            let before = old.get(*key).cloned().unwrap_or(Value::Null);
            let after = new.get(*key).cloned().unwrap_or(Value::Null);
            (before != after).then(|| FieldChange::new(*key, before, after))
        })
        .collect()
}

pub(crate) fn recalculation_job(currency: Option<&str>) -> Job {
    Job::new(CURRENCIES_RECALCULATE_JOB, json!({ "currency": currency })).with_max_attempts(3)
}

fn today(ctx: &Ctx<'_>, context: &CurrencyContext) -> NaiveDate {
    organization_today(&context.timezone, ctx.now())
}

pub async fn settings(ctx: &Ctx<'_>) -> Result<CurrencySettings, AppError> {
    require_reader(ctx.scope)?;
    let store = CurrencyStore::of(ctx);
    let context = store.context().await?;
    let today = today(ctx, &context);
    let mut currencies = vec![CurrencyView {
        code: context.base.clone(),
        is_base: true,
        enabled_at: None,
        current_rate: None,
    }];
    let mut enabled = store.enabled().await?;
    enabled.retain(|currency| currency.code != context.base);
    enabled.sort_by(|left, right| left.code.cmp(&right.code));
    for currency in enabled {
        let current_rate = store.rate_on(&currency.code, today).await?;
        currencies.push(CurrencyView {
            code: currency.code,
            is_base: false,
            enabled_at: Some(currency.enabled_at),
            current_rate,
        });
    }
    Ok(CurrencySettings {
        base: context.base,
        today,
        currencies,
        provider: store.provider_settings().await?,
    })
}

pub async fn set_currencies(
    ctx: &Ctx<'_>,
    command: SetCurrenciesCommand,
) -> Result<CurrencySettings, AppError> {
    let scope = ctx.scope;
    scope.require(&Permission::SETTINGS_MANAGE)?;
    let store = CurrencyStore::of(ctx);
    let context = store.context().await?;
    let mut violations = Vec::new();
    let mut requested = BTreeSet::new();
    for (index, code) in command.currencies.iter().enumerate() {
        match normalize_currency_code(code) {
            Some(code) if code == context.base => {}
            Some(code) => {
                requested.insert(code);
            }
            None => violations.push(FieldViolation::new(
                format!("currencies.{index}"),
                "must be an ISO 4217 code with three letters",
            )),
        }
    }
    if requested.len() > MAX_ENABLED_CURRENCIES {
        violations.push(FieldViolation::new(
            "currencies",
            format!("at most {MAX_ENABLED_CURRENCIES} currencies can be enabled"),
        ));
    }
    if !violations.is_empty() {
        return Err(AppError::invalid_fields("invalid currencies", violations));
    }
    let current: BTreeSet<String> = store
        .enabled()
        .await?
        .into_iter()
        .map(|currency| currency.code)
        .filter(|code| *code != context.base)
        .collect();
    let added: Vec<&String> = requested.difference(&current).collect();
    let removed: Vec<&String> = current.difference(&requested).collect();
    if added.is_empty() && removed.is_empty() {
        return settings(ctx).await;
    }
    let now = ctx.now();
    let actor = Actor::from_scope(scope);
    let mut tx = ctx.begin().await?;
    for (code, event_type, enabled) in added
        .iter()
        .map(|code| (*code, CURRENCY_ENABLED, true))
        .chain(removed.iter().map(|code| (*code, CURRENCY_DISABLED, false)))
    {
        if enabled {
            store
                .enable(&mut tx, code, Some(scope.user_id()), now)
                .await?;
        } else {
            store.disable(&mut tx, code).await?;
        }
        ctx.events()
            .publish(
                &mut tx,
                DomainEvent::new(actor.clone(), event_type, now)
                    .with_payload(json!({ "currency": code, "base": context.base })),
            )
            .await?;
        ctx.audit()
            .record(
                &mut tx,
                AuditEntry::new(actor.clone(), event_type, now)
                    .with_changes(vec![FieldChange::new(
                        format!("currencies.{code}"),
                        json!(!enabled),
                        json!(enabled),
                    )])
                    .with_ip(command.ip.clone()),
            )
            .await?;
    }
    tx.commit().await?;
    settings(ctx).await
}

pub(crate) async fn list_rates(
    ctx: &Ctx<'_>,
    filter: ExchangeRateFilter,
    window: ListWindow,
) -> Result<Listing<ExchangeRate>, AppError> {
    require_reader(ctx.scope)?;
    let currency = match filter.currency.as_deref().map(str::trim) {
        None | Some("") => None,
        Some(code) => Some(normalize_currency_code(code).ok_or_else(|| {
            AppError::invalid_fields(
                "invalid filter",
                vec![FieldViolation::new(
                    "currency",
                    "must be an ISO 4217 code with three letters",
                )],
            )
        })?),
    };
    CurrencyStore::of(ctx)
        .rates(&ExchangeRateFilter { currency }, window)
        .await
}

pub(crate) async fn get_rate(ctx: &Ctx<'_>, id: &str) -> Result<ExchangeRate, AppError> {
    require_reader(ctx.scope)?;
    require_rate(&CurrencyStore::of(ctx), id).await
}

fn provider_snapshot(settings: &RatesProviderSettings) -> Value {
    json!({ "provider": settings.provider, "url": settings.url })
}

pub(crate) async fn update_provider(
    ctx: &Ctx<'_>,
    command: UpdateRatesProviderCommand,
) -> Result<CurrencySettings, AppError> {
    let scope = ctx.scope;
    scope.require(&Permission::SETTINGS_MANAGE)?;
    let store = CurrencyStore::of(ctx);
    let mut violations = Vec::new();
    let provider = command.provider.trim().to_ascii_lowercase();
    if !RATES_PROVIDERS.contains(&provider.as_str()) {
        violations.push(FieldViolation::new(
            "provider",
            format!("must be one of {}", RATES_PROVIDERS.join(", ")),
        ));
    }
    let url = command
        .url
        .map(|url| url.trim().trim_end_matches('/').to_owned())
        .filter(|url| !url.is_empty());
    if let Some(url) = &url
        && (url.len() > MAX_PROVIDER_URL_LENGTH
            || !(url.starts_with("https://") || url.starts_with("http://")))
    {
        violations.push(FieldViolation::new(
            "url",
            format!("must be an http(s) address with at most {MAX_PROVIDER_URL_LENGTH} characters"),
        ));
    }
    if !violations.is_empty() {
        return Err(AppError::invalid_fields(
            "invalid exchange rate provider",
            violations,
        ));
    }
    let current = store.provider_settings().await?;
    let next = RatesProviderSettings {
        provider,
        url,
        last_synced_at: current.last_synced_at,
    };
    let before = provider_snapshot(&current);
    let after = provider_snapshot(&next);
    let changes: Vec<FieldChange> = ["provider", "url"]
        .iter()
        .filter_map(|key| {
            let old = before.get(*key).cloned().unwrap_or(Value::Null);
            let new = after.get(*key).cloned().unwrap_or(Value::Null);
            (old != new).then(|| FieldChange::new(format!("rates_{key}"), old, new))
        })
        .collect();
    if changes.is_empty() {
        return settings(ctx).await;
    }
    let now = ctx.now();
    let actor = Actor::from_scope(scope);
    let mut tx = ctx.begin().await?;
    store
        .save_provider_settings(&mut tx, &next, Some(scope.user_id()), now)
        .await?;
    ctx.events()
        .publish(
            &mut tx,
            DomainEvent::new(actor.clone(), RATES_PROVIDER_UPDATED, now).with_payload(after),
        )
        .await?;
    ctx.audit()
        .record(
            &mut tx,
            AuditEntry::new(actor, RATES_PROVIDER_UPDATED, now)
                .with_changes(changes)
                .with_ip(command.ip),
        )
        .await?;
    tx.commit().await?;
    settings(ctx).await
}

pub(crate) async fn sync_now(ctx: &Ctx<'_>, ip: Option<String>) -> Result<SyncOutcome, AppError> {
    ctx.scope.require(&Permission::SETTINGS_MANAGE)?;
    let settings = CurrencyStore::of(ctx).provider_settings().await?;
    let provider = configured_provider(&settings).ok_or_else(|| {
        AppError::validation_code(
            RATES_PROVIDER_NOT_CONFIGURED,
            "no exchange rate provider is configured",
        )
    })?;
    sync_rates(
        &RatePorts::of(ctx),
        &Actor::from_scope(ctx.scope),
        ip,
        provider.as_ref(),
    )
    .await
}

pub async fn create_rate(
    ctx: &Ctx<'_>,
    command: CreateExchangeRateCommand,
) -> Result<ExchangeRate, AppError> {
    let scope = ctx.scope;
    scope.require(&Permission::SETTINGS_MANAGE)?;
    let store = CurrencyStore::of(ctx);
    let context = store.context().await?;
    let mut violations = Vec::new();
    let currency = normalize_currency_code(&command.currency);
    match &currency {
        None => violations.push(FieldViolation::new(
            "currency",
            "must be an ISO 4217 code with three letters",
        )),
        Some(code) if *code == context.base => violations.push(FieldViolation::new(
            "currency",
            "the base currency always has the rate 1",
        )),
        Some(_) => {}
    }
    let effective_date = match command.effective_date.as_deref().map(str::trim) {
        None | Some("") => Some(today(ctx, &context)),
        Some(text) => parse_date(text, &mut violations),
    };
    let rate = parse_rate(&command.rate, &mut violations);
    let (Some(currency), Some(effective_date), Some(rate), true) =
        (currency, effective_date, rate, violations.is_empty())
    else {
        return Err(AppError::invalid_fields(INVALID_RATE, violations));
    };
    if !store.is_enabled(&currency).await? {
        return Err(currency_not_enabled(&currency));
    }
    if store.rate_dated(&currency, effective_date).await?.is_some() {
        return Err(rate_exists());
    }
    let now = ctx.now();
    let exchange_rate = ExchangeRate {
        id: Uuid::now_v7(),
        currency,
        effective_date,
        rate,
        source: RateSource::Manual,
        provider: None,
        created_by: Some(scope.user_id().clone()),
        created_at: now,
        updated_at: now,
    };
    let snapshot = rate_snapshot(&exchange_rate);
    let mut tx = ctx.begin().await?;
    store.insert_rate(&mut tx, &exchange_rate).await?;
    publish_rate(
        ctx,
        &mut tx,
        &exchange_rate,
        EXCHANGE_RATE_CREATED,
        snapshot_changes(&Value::Null, &snapshot),
        command.ip,
    )
    .await?;
    enqueue_recalculation(
        ctx,
        &mut tx,
        &context,
        &exchange_rate.currency,
        &[effective_date],
    )
    .await?;
    tx.commit().await?;
    Ok(exchange_rate)
}

pub(crate) async fn update_rate(
    ctx: &Ctx<'_>,
    id: &str,
    command: UpdateExchangeRateCommand,
) -> Result<ExchangeRate, AppError> {
    ctx.scope.require(&Permission::SETTINGS_MANAGE)?;
    let store = CurrencyStore::of(ctx);
    let current = require_rate(&store, id).await?;
    ensure_version(command.expected_updated_at, current.updated_at)?;
    let context = store.context().await?;
    let mut violations = Vec::new();
    let mut next = current.clone();
    if let Some(text) = command.effective_date.as_deref()
        && let Some(date) = parse_date(text, &mut violations)
    {
        next.effective_date = date;
    }
    if let Some(text) = command.rate.as_deref()
        && let Some(rate) = parse_rate(text, &mut violations)
    {
        next.rate = rate;
    }
    if !violations.is_empty() {
        return Err(AppError::invalid_fields(INVALID_RATE, violations));
    }
    if next.effective_date != current.effective_date
        && store
            .rate_dated(&next.currency, next.effective_date)
            .await?
            .is_some_and(|other| other.id != current.id)
    {
        return Err(rate_exists());
    }
    if next.rate != current.rate || next.effective_date != current.effective_date {
        next.source = RateSource::Manual;
        next.provider = None;
    }
    let changes = snapshot_changes(&rate_snapshot(&current), &rate_snapshot(&next));
    if changes.is_empty() {
        return Ok(current);
    }
    next.updated_at = ctx.now();
    let mut tx = ctx.begin().await?;
    store.update_rate(&mut tx, &next).await?;
    publish_rate(
        ctx,
        &mut tx,
        &next,
        EXCHANGE_RATE_UPDATED,
        changes,
        command.ip,
    )
    .await?;
    enqueue_recalculation(
        ctx,
        &mut tx,
        &context,
        &next.currency,
        &[current.effective_date, next.effective_date],
    )
    .await?;
    tx.commit().await?;
    Ok(next)
}

pub(crate) async fn delete_rate(
    ctx: &Ctx<'_>,
    id: &str,
    ip: Option<String>,
) -> Result<(), AppError> {
    ctx.scope.require(&Permission::SETTINGS_MANAGE)?;
    let store = CurrencyStore::of(ctx);
    let current = require_rate(&store, id).await?;
    let context = store.context().await?;
    let mut tx = ctx.begin().await?;
    store.delete_rate(&mut tx, current.id).await?;
    publish_rate(
        ctx,
        &mut tx,
        &current,
        EXCHANGE_RATE_DELETED,
        snapshot_changes(&rate_snapshot(&current), &Value::Null),
        ip,
    )
    .await?;
    enqueue_recalculation(ctx, &mut tx, &context, &current.currency, &[]).await?;
    tx.commit().await?;
    Ok(())
}

async fn require_rate(store: &CurrencyStore, id: &str) -> Result<ExchangeRate, AppError> {
    let id = Uuid::parse_str(id.trim()).map_err(|_| rate_not_found())?;
    store.find_rate(id).await?.ok_or_else(rate_not_found)
}

async fn enqueue_recalculation(
    ctx: &Ctx<'_>,
    tx: &mut MySqlTx,
    context: &CurrencyContext,
    currency: &str,
    dates: &[NaiveDate],
) -> Result<(), AppError> {
    ctx.jobs()
        .enqueue(tx, recalculation_job(Some(currency)))
        .await?;
    let today = today(ctx, context);
    let zone = context.timezone.parse::<Tz>().unwrap_or(Tz::UTC);
    let future: BTreeSet<NaiveDate> = dates.iter().copied().filter(|date| *date > today).collect();
    for date in future {
        let Some(starts_at) = date
            .and_hms_opt(0, 0, 0)
            .and_then(|midnight| zone.from_local_datetime(&midnight).earliest())
        else {
            continue;
        };
        let job = recalculation_job(Some(currency))
            .run_at(starts_at.with_timezone(&chrono::Utc))
            .with_unique_key(format!("{CURRENCIES_RECALCULATE_JOB}:{currency}:{date}"));
        ctx.jobs().enqueue(tx, job).await?;
    }
    Ok(())
}

async fn publish_rate(
    ctx: &Ctx<'_>,
    tx: &mut MySqlTx,
    rate: &ExchangeRate,
    event_type: &'static str,
    changes: Vec<FieldChange>,
    ip: Option<String>,
) -> Result<(), AppError> {
    publish_rate_change(
        &RatePorts::of(ctx),
        tx,
        &Actor::from_scope(ctx.scope),
        rate,
        event_type,
        changes,
        ip,
    )
    .await
}

#[derive(Clone)]
pub(crate) struct RatePorts {
    pub(crate) store: CurrencyStore,
    pub(crate) events: SharedEventPublisher,
    pub(crate) audit: SharedAuditLog,
    pub(crate) jobs: SharedJobQueue,
}

impl RatePorts {
    pub(crate) fn of(ctx: &Ctx<'_>) -> Self {
        Self {
            store: CurrencyStore::of(ctx),
            events: ctx.events().clone(),
            audit: ctx.audit().clone(),
            jobs: ctx.jobs().clone(),
        }
    }

    pub(crate) fn system(pool: MySqlPool) -> Self {
        Self {
            store: CurrencyStore::new(pool.clone()),
            events: Arc::new(MySqlEventPublisher::new()),
            audit: Arc::new(MySqlAuditLog::new()),
            jobs: Arc::new(MySqlJobQueue::new(pool)),
        }
    }
}

fn changes_payload(changes: &[FieldChange]) -> Value {
    Value::Array(
        changes
            .iter()
            .map(|change| json!({ "field": change.field, "old": change.old, "new": change.new }))
            .collect(),
    )
}

pub(crate) async fn publish_rate_change(
    ports: &RatePorts,
    tx: &mut MySqlTx,
    actor: &Actor,
    rate: &ExchangeRate,
    event_type: &'static str,
    changes: Vec<FieldChange>,
    ip: Option<String>,
) -> Result<(), AppError> {
    let now = chrono::Utc::now();
    let record = RecordRef::new(EXCHANGE_RATE_RECORD_TYPE, rate.id);
    let mut payload = rate_snapshot(rate);
    payload["provider"] = json!(rate.provider);
    payload["changes"] = changes_payload(&changes);
    ports
        .events
        .publish(
            tx,
            DomainEvent::new(actor.clone(), event_type, now)
                .with_record(record.clone())
                .with_payload(payload),
        )
        .await?;
    ports
        .audit
        .record(
            tx,
            AuditEntry::new(actor.clone(), event_type, now)
                .with_record(record)
                .with_changes(changes)
                .with_ip(ip),
        )
        .await
}
