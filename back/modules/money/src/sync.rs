use chrono::{NaiveDate, Utc};
use rust_decimal::Decimal;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::domain::{
    CURRENCY_ENABLED, EXCHANGE_RATE_CREATED, EXCHANGE_RATE_UPDATED, EXCHANGE_RATES_REBASED,
    ExchangeRate, RateSource, RebaseOutcome, SyncOutcome, normalize_rate, organization_today,
};
use crate::provider::ExchangeRateProvider;
use crate::service::{
    RatePorts, publish_rate_change, rate_snapshot, recalculation_job, snapshot_changes,
};
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::application::shared::{
    Actor, AuditEntry, AuditLog, DomainEvent, EventPublisher, FieldChange, JobQueue,
};
use aphel_sdk::platform::database::transaction::MySqlTx;

enum SyncedRate {
    Created,
    Updated,
    Unchanged,
    KeptManual,
}

pub(crate) async fn sync_rates(
    ports: &RatePorts,
    actor: &Actor,
    ip: Option<String>,
    provider: &dyn ExchangeRateProvider,
) -> Result<SyncOutcome, AppError> {
    rebase(ports, actor).await?;
    let context = ports.store.context().await?;
    let currencies = ports.store.enabled_codes().await?;
    let today = organization_today(&context.timezone, Utc::now());
    let mut outcome = SyncOutcome {
        provider: provider.key().to_owned(),
        effective_date: today,
        created: Vec::new(),
        updated: Vec::new(),
        unchanged: Vec::new(),
        kept_manual: Vec::new(),
        failed: Vec::new(),
    };
    for currency in currencies {
        let rate = match provider.rate(&context.base, &currency).await {
            Ok(rate) => normalize_rate(rate),
            Err(error) => {
                tracing::warn!(provider = provider.key(), %currency, %error, "exchange rate provider failed");
                None
            }
        };
        let Some(rate) = rate else {
            outcome.failed.push(currency);
            continue;
        };
        let synced = store_rate(
            ports,
            actor,
            ip.clone(),
            provider.key(),
            &currency,
            today,
            rate,
        )
        .await?;
        let bucket = match synced {
            SyncedRate::Created => &mut outcome.created,
            SyncedRate::Updated => &mut outcome.updated,
            SyncedRate::Unchanged => &mut outcome.unchanged,
            SyncedRate::KeptManual => &mut outcome.kept_manual,
        };
        bucket.push(currency);
    }
    ports.store.mark_synced(Utc::now()).await?;
    Ok(outcome)
}

async fn store_rate(
    ports: &RatePorts,
    actor: &Actor,
    ip: Option<String>,
    provider: &str,
    currency: &str,
    date: NaiveDate,
    rate: Decimal,
) -> Result<SyncedRate, AppError> {
    let now = Utc::now();
    let mut tx = ports.store.begin().await?;
    let synced = match ports.store.rate_dated_in(&mut tx, currency, date).await? {
        None => {
            let created = ExchangeRate {
                id: Uuid::now_v7(),
                currency: currency.to_owned(),
                effective_date: date,
                rate,
                source: RateSource::Integration,
                provider: Some(provider.to_owned()),
                created_by: None,
                created_at: now,
                updated_at: now,
            };
            ports.store.insert_rate(&mut tx, &created).await?;
            let changes = snapshot_changes(&Value::Null, &rate_snapshot(&created));
            publish_rate_change(
                ports,
                &mut tx,
                actor,
                &created,
                EXCHANGE_RATE_CREATED,
                changes,
                ip,
            )
            .await?;
            SyncedRate::Created
        }
        Some(existing) if existing.source == RateSource::Manual => SyncedRate::KeptManual,
        Some(existing) if existing.rate == rate => SyncedRate::Unchanged,
        Some(existing) => {
            let mut next = existing.clone();
            next.rate = rate;
            next.provider = Some(provider.to_owned());
            next.updated_at = now;
            ports.store.update_rate(&mut tx, &next).await?;
            let changes = snapshot_changes(&rate_snapshot(&existing), &rate_snapshot(&next));
            publish_rate_change(
                ports,
                &mut tx,
                actor,
                &next,
                EXCHANGE_RATE_UPDATED,
                changes,
                ip,
            )
            .await?;
            SyncedRate::Updated
        }
    };
    if matches!(synced, SyncedRate::Created | SyncedRate::Updated) {
        ports
            .jobs
            .enqueue(&mut tx, recalculation_job(Some(currency)))
            .await?;
        tx.commit().await?;
    } else {
        tx.rollback().await?;
    }
    Ok(synced)
}

fn anchor_on(anchor: &[(NaiveDate, Decimal)], date: NaiveDate) -> Option<Decimal> {
    anchor
        .iter()
        .rev()
        .find(|(effective_date, _)| *effective_date <= date)
        .or_else(|| anchor.first())
        .map(|(_, rate)| *rate)
}

pub(crate) async fn rebase(
    ports: &RatePorts,
    actor: &Actor,
) -> Result<Option<RebaseOutcome>, AppError> {
    let mut tx = ports.store.begin().await?;
    let stored = ports.store.lock_rates_base(&mut tx).await?;
    let Some(base) = ports.store.base_in(&mut tx).await? else {
        tx.rollback().await?;
        return Ok(None);
    };
    let previous = match stored {
        Some(previous) if previous != base => previous,
        Some(_) => {
            tx.rollback().await?;
            return Ok(None);
        }
        None => {
            ports.store.set_rates_base(&mut tx, &base).await?;
            tx.commit().await?;
            return Ok(None);
        }
    };
    let outcome = rebase_rates(ports, &mut tx, actor, &previous, &base).await?;
    tx.commit().await?;
    Ok(Some(outcome))
}

async fn rebase_rates(
    ports: &RatePorts,
    tx: &mut MySqlTx,
    actor: &Actor,
    previous: &str,
    base: &str,
) -> Result<RebaseOutcome, AppError> {
    let now = Utc::now();
    let store = &ports.store;
    let rates = store.all_rates_in(tx).await?;
    let anchor: Vec<(NaiveDate, Decimal)> = rates
        .iter()
        .filter(|rate| rate.currency == base)
        .map(|rate| (rate.effective_date, rate.rate))
        .collect();
    let mut outcome = RebaseOutcome {
        previous_base: previous.to_owned(),
        base: base.to_owned(),
        ..RebaseOutcome::default()
    };
    for rate in rates.iter().filter(|rate| rate.currency == previous) {
        store.delete_rate(tx, rate.id).await?;
        outcome.removed += 1;
    }
    for rate in rates.iter().filter(|rate| rate.currency != previous) {
        if rate.currency == base {
            store.delete_rate(tx, rate.id).await?;
            if let Some(inverse) = Decimal::ONE.checked_div(rate.rate).and_then(normalize_rate) {
                let created = ExchangeRate {
                    id: Uuid::now_v7(),
                    currency: previous.to_owned(),
                    rate: inverse,
                    created_at: now,
                    updated_at: now,
                    ..rate.clone()
                };
                store.insert_rate(tx, &created).await?;
                outcome.created += 1;
            }
            outcome.removed += 1;
            continue;
        }
        match anchor_on(&anchor, rate.effective_date)
            .and_then(|anchor| rate.rate.checked_div(anchor))
            .and_then(normalize_rate)
        {
            Some(converted) => {
                if converted != rate.rate {
                    let next = ExchangeRate {
                        rate: converted,
                        updated_at: now,
                        ..rate.clone()
                    };
                    store.update_rate(tx, &next).await?;
                }
                outcome.converted += 1;
            }
            None => {
                store.delete_rate(tx, rate.id).await?;
                outcome.removed += 1;
            }
        }
    }
    if !store.is_enabled(previous).await? {
        store.enable(tx, previous, None, now).await?;
        ports
            .events
            .publish(
                tx,
                DomainEvent::new(actor.clone(), CURRENCY_ENABLED, now)
                    .with_payload(json!({ "currency": previous, "base": base })),
            )
            .await?;
        ports
            .audit
            .record(
                tx,
                AuditEntry::new(actor.clone(), CURRENCY_ENABLED, now).with_changes(vec![
                    FieldChange::new(format!("currencies.{previous}"), json!(false), json!(true)),
                ]),
            )
            .await?;
    }
    store.set_rates_base(tx, base).await?;
    let summary = json!({
        "previous_base": outcome.previous_base,
        "base": outcome.base,
        "converted": outcome.converted,
        "created": outcome.created,
        "removed": outcome.removed,
    });
    ports
        .events
        .publish(
            tx,
            DomainEvent::new(actor.clone(), EXCHANGE_RATES_REBASED, now).with_payload(summary),
        )
        .await?;
    ports
        .audit
        .record(
            tx,
            AuditEntry::new(actor.clone(), EXCHANGE_RATES_REBASED, now).with_changes(vec![
                FieldChange::new("rates_base", json!(previous), json!(base)),
                FieldChange::new("rates_converted", Value::Null, json!(outcome.converted)),
                FieldChange::new("rates_removed", Value::Null, json!(outcome.removed)),
            ]),
        )
        .await?;
    ports.jobs.enqueue(tx, recalculation_job(None)).await?;
    Ok(outcome)
}
