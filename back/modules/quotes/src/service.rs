use aphel_contract_crm::{CrmPermissions, CrmRecordTypes};
use std::collections::BTreeMap;

use chrono::{DateTime, Duration, NaiveDate, Utc};
use chrono_tz::Tz;
use rust_decimal::Decimal;
use serde_json::{Map, Value, json};
use uuid::Uuid;

use crate::QUOTE;
use crate::domain::{
    CreateQuoteCommand, MAX_DEFAULT_VALID_DAYS, MAX_QUOTE_ITEMS, MAX_QUOTE_TEXT_LENGTH,
    MAX_QUOTE_TITLE_LENGTH, QUOTE_ALREADY_ACCEPTED, QUOTE_CREATED, QUOTE_DELETED,
    QUOTE_ITEMS_LIMIT_REACHED, QUOTE_LINK_PATH, QUOTE_NOT_CURRENT, QUOTE_NOT_EDITABLE,
    QUOTE_NOT_FOUND, QUOTE_PRICE_LIST_NOT_FOUND, QUOTE_SETTINGS_UPDATED, QUOTE_UPDATED,
    QUOTE_VERSION_CREATED, Quote, QuoteDetails, QuoteDiscount, QuoteInput, QuoteItem,
    QuoteItemData, QuoteSettings, QuoteStatus, QuoteSummary, QuoteVersionSummary, TOKEN_KEY_NAME,
    TOKEN_KEY_OWNER, TotalsBreakdown, TotalsError, UpdateQuoteCommand, UpdateQuoteSettingsCommand,
    compute_totals, exceeds_discount_limit, max_discount_percent,
};
use crate::gateway::{QuoteApprovals, QuoteCatalog, QuoteGateway, quote_base_url};
use crate::pricing::{PricingContext, item_input_from_deal, item_input_from_quote, resolve_items};
use crate::store::QuoteStore;
use crate::token::encode_link_token;
use crate::{KEY, PricedQuote, QuoteTotals};
use aphel_kernel::Ctx;
use aphel_kernel::application::access::Authorizer;
use aphel_kernel::application::audit::{DiffOptions, diff_values};
use aphel_kernel::application::fields::permissions::check_record_writes;
use aphel_kernel::application::fields::{
    CustomFieldPorts, CustomFieldReader, CustomFieldWriter, RecordDraft, RecordPipeline,
    RecordWrite, ReferenceChecker, WriteMode, decimal_json, saving_unsupported,
};
use aphel_kernel::application::organization_admin::Listing;
use aphel_kernel::application::query::{Filter, FilterOperator, RecordQuery, RecordQueryEngine};
use aphel_kernel::application::secrets::{SecretKey, SecretVault};
use aphel_kernel::application::shared::{
    Actor, AuditEntry, AuditLog, DomainEvent, EventPublisher, FieldChange, RecordRef, RecordType,
    ensure_version,
};
use aphel_kernel::database::transaction::MySqlTx;
use aphel_kernel::domain::access::{Permission, RecordScopeFilter};
use aphel_kernel::domain::organization_profile::ORGANIZATION_RECORD_ID;
use aphel_kernel::domain::{Secret, UserId};
use aphel_kernel::errors::{AppError, FieldViolation};
use aphel_kernel::records::{
    InputValidator, RecordMutation, changes_payload, merge_violations, parse_record_id,
    publish_mutation, value_patch,
};
use aphel_sdk::line_items::{DiscountKind, parse_money};
use approvals::domain::APPROVAL_PENDING;
use price_lists::domain::PriceListSelection;

use scheduling::token::{decode_key, generate_key};

use aphel_kernel::application::shared::money_text;
pub(crate) use aphel_kernel::application::shared::parse_uuid as parse_id;

pub(crate) const INVALID_QUOTE: &str = "invalid quote";

pub(crate) fn quote_not_found() -> AppError {
    AppError::not_found_code(QUOTE_NOT_FOUND, "quote was not found")
}

pub(crate) fn quote_record(id: Uuid) -> RecordRef {
    RecordRef::new(QUOTE, id)
}

fn date_text(value: Option<NaiveDate>) -> Value {
    value.map_or(Value::Null, |date| {
        Value::String(date.format("%Y-%m-%d").to_string())
    })
}

pub(crate) fn item_snapshot(data: &QuoteItemData, total: Decimal) -> Value {
    json!({
        "product_id": data.product_id.map(|id| id.to_string()),
        "name": data.name,
        "quantity": decimal_json(data.quantity),
        "unit_price": money_text(data.unit_price),
        "discount_type": data.discount_kind.as_str(),
        "discount_value": money_text(data.discount_value),
        "tax_percent": decimal_json(data.tax_percent),
        "total": money_text(total),
    })
}

pub(crate) fn quote_snapshot(quote: &Quote) -> Value {
    json!({
        "title": quote.title,
        "number": quote.number,
        "version": quote.version,
        "is_current": quote.is_current,
        "status": quote.status.as_str(),
        "deal_id": quote.deal_id.to_string(),
        "contact_id": quote.contact_id.map(|id| id.to_string()),
        "company_id": quote.company_id.map(|id| id.to_string()),
        "owner_id": quote.owner_id.as_ref().map(UserId::as_str),
        "currency": quote.currency,
        "price_list_id": quote.price_list_id.map(|id| id.to_string()),
        "valid_until": date_text(quote.valid_until),
        "terms": quote.terms,
        "notes": quote.notes,
        "discount_type": quote.discount.kind.as_str(),
        "discount_value": money_text(quote.discount.value),
        "subtotal": money_text(quote.subtotal),
        "discount_total": money_text(quote.discount_total),
        "tax_total": money_text(quote.tax_total),
        "total": money_text(quote.total),
        "exceeds_discount_limit": quote.exceeds_discount_limit,
    })
}

fn snapshot_with_items(quote: &Quote, items: &[QuoteItem]) -> Value {
    let mut snapshot = quote_snapshot(quote);
    if let Value::Object(map) = &mut snapshot {
        map.insert(
            "items".to_owned(),
            Value::Array(
                items
                    .iter()
                    .map(|item| item_snapshot(&item.data, item.amounts.total))
                    .collect(),
            ),
        );
    }
    snapshot
}

pub(crate) fn totals_error(error: TotalsError) -> AppError {
    let violation = match error {
        TotalsError::ItemPercentOutOfRange(index) => FieldViolation::new(
            format!("items.{index}.discount_value"),
            "must be a percentage between 0 and 100",
        ),
        TotalsError::ItemTaxOutOfRange(index) => FieldViolation::new(
            format!("items.{index}.tax_percent"),
            "must be a percentage between 0 and 100",
        ),
        TotalsError::ItemTooLarge(index) => FieldViolation::new(
            format!("items.{index}.unit_price"),
            "the line total is too large",
        ),
        TotalsError::TotalPercentOutOfRange => {
            FieldViolation::new("discount_value", "must be a percentage between 0 and 100")
        }
        TotalsError::TooLarge => FieldViolation::new("items", "the quote total is too large"),
    };
    AppError::invalid_fields(INVALID_QUOTE, vec![violation])
}

pub(crate) fn build_items(
    quote_id: Uuid,
    data: Vec<QuoteItemData>,
    totals: &TotalsBreakdown,
) -> Vec<QuoteItem> {
    data.into_iter()
        .zip(totals.lines.iter().copied())
        .enumerate()
        .map(|(index, (data, amounts))| QuoteItem {
            id: Uuid::now_v7(),
            quote_id,
            position: i32::try_from(index).unwrap_or(i32::MAX),
            data,
            amounts,
        })
        .collect()
}

pub(crate) fn parse_date(text: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(text.trim(), "%Y-%m-%d").ok()
}

pub(crate) fn timezone(name: &str) -> Tz {
    name.parse::<Tz>().unwrap_or(chrono_tz::America::Sao_Paulo)
}

pub(crate) fn not_editable(quote: &Quote) -> Option<AppError> {
    if !quote.is_current {
        return Some(AppError::conflict_code(
            QUOTE_NOT_CURRENT,
            "only the current version of a quote can change; create a new version",
        ));
    }
    match quote.status {
        QuoteStatus::PendingApproval => Some(AppError::conflict_code(
            APPROVAL_PENDING,
            "the quote is waiting for approval and cannot change",
        )),
        status if status.is_editable() => None,
        _ => Some(AppError::conflict_code(
            QUOTE_NOT_EDITABLE,
            "the quote was already sent or closed; create a new version to change it",
        )),
    }
}

pub(crate) struct SystemEvent<'a> {
    pub(crate) event_type: &'a str,
    pub(crate) extra: Value,
    pub(crate) changes: Vec<FieldChange>,
    pub(crate) ip: Option<String>,
}

pub(crate) async fn list(
    ctx: &Ctx<'_>,
    mut query: RecordQuery,
    include_versions: bool,
) -> Result<Listing<QuoteSummary>, AppError> {
    let scope = ctx.scope;
    if !include_versions {
        let current = Filter::condition("is_current", FilterOperator::Eq, Value::Bool(true));
        query.filter = Some(match query.filter.take() {
            Some(filter) => Filter::and(vec![filter, current]),
            None => current,
        });
    }
    let page = RecordQueryEngine::new(&ctx.query())
        .query(scope, QUOTE.as_str(), query)
        .await?;
    let quotes = QuoteStore::of(ctx).find_many(&page.items).await?;
    let ordered: Vec<Quote> = page
        .items
        .iter()
        .filter_map(|id| quotes.iter().find(|quote| quote.id == *id).cloned())
        .collect();
    Ok(Listing {
        items: summaries(ctx, ordered).await?,
        total: page.total,
    })
}

pub(crate) async fn for_deal(ctx: &Ctx<'_>, deal_id: &str) -> Result<Vec<QuoteSummary>, AppError> {
    let scope = ctx.scope;
    let visibility = scope.record_filter(&Permission::QUOTES_READ)?;
    let deal_id = parse_id(deal_id).ok_or_else(|| {
        AppError::not_found_code(crate::gateway::DEAL_NOT_FOUND, "deal was not found")
    })?;
    QuoteGateway::of(ctx).deal(scope, deal_id).await?;
    let quotes = QuoteStore::of(ctx)
        .for_deal(scope, &visibility, deal_id)
        .await?;
    summaries(ctx, quotes).await
}

pub async fn get(ctx: &Ctx<'_>, quote_id: &str) -> Result<QuoteDetails, AppError> {
    let quote = visible(ctx, &Permission::QUOTES_READ, quote_id).await?;
    details(ctx, quote).await
}

pub(crate) async fn versions(
    ctx: &Ctx<'_>,
    quote_id: &str,
) -> Result<Vec<QuoteVersionSummary>, AppError> {
    let quote = visible(ctx, &Permission::QUOTES_READ, quote_id).await?;
    QuoteStore::of(ctx).versions(quote.number).await
}

pub async fn create(ctx: &Ctx<'_>, command: CreateQuoteCommand) -> Result<QuoteDetails, AppError> {
    let scope = ctx.scope;
    scope.require(&Permission::QUOTES_CREATE)?;
    let deal_id = parse_id(&command.deal_id).ok_or_else(|| {
        AppError::invalid_fields(
            INVALID_QUOTE,
            vec![FieldViolation::new("deal_id", "must be a valid deal id")],
        )
    })?;
    let deal = QuoteGateway::of(ctx).deal(scope, deal_id).await?;
    let settings = QuoteStore::of(ctx).settings().await?;
    let today = today(ctx).await?;
    let now = ctx.now();
    let mut validator = InputValidator::new();
    let mut input = command.input;
    if input.title.is_none() {
        input.title = Some(Some(deal.title.clone()));
    }
    let base = Quote {
        id: Uuid::now_v7(),
        deal_id: deal.id,
        number: 0,
        version: 1,
        is_current: true,
        title: String::new(),
        status: QuoteStatus::Draft,
        owner_id: Some(scope.user_id().clone()),
        contact_id: deal.contact_id,
        company_id: deal.company_id,
        currency: deal.currency.clone(),
        price_list_id: None,
        valid_until: settings
            .default_valid_days
            .map(|days| today + Duration::days(i64::from(days))),
        terms: settings.default_terms.clone(),
        notes: None,
        discount: QuoteDiscount::default(),
        subtotal: Decimal::ZERO,
        discount_total: Decimal::ZERO,
        tax_total: Decimal::ZERO,
        total: Decimal::ZERO,
        exceeds_discount_limit: false,
        public_key: None,
        sent_at: None,
        sent_by: None,
        viewed_at: None,
        accepted_at: None,
        rejected_at: None,
        expired_at: None,
        acceptance: None,
        rejection_reason: None,
        deal_synced_at: None,
        deal_sync_error: None,
        created_by: Some(scope.user_id().clone()),
        created_at: now,
        updated_at: now,
    };
    let provided = input.provided_keys();
    let explicit_price_list = input.price_list_id.is_some();
    let mut quote = apply_input(ctx, &base, input, &mut validator).await?;
    if !explicit_price_list {
        quote.price_list_id = QuoteCatalog::of(ctx)
            .default_for(
                scope,
                &PriceListSelection {
                    currency: quote.currency.clone(),
                    company_id: quote.company_id,
                    on: today,
                },
            )
            .await?
            .map(|price_list| price_list.id);
    }
    let inputs = if command.from_line_items {
        QuoteGateway::of(ctx)
            .line_items(deal.id)
            .await?
            .iter()
            .map(item_input_from_deal)
            .collect()
    } else {
        command.items.unwrap_or_default()
    };
    check_item_count(inputs.len())?;
    let data = resolve_items(
        ctx,
        scope,
        inputs,
        &PricingContext {
            currency: &quote.currency,
            price_list_id: quote.price_list_id,
            on: today,
            reprice: false,
        },
        &mut validator,
    )
    .await?;
    let (totals, items) = price(ctx, &mut quote, data).await?;
    quote.exceeds_discount_limit = exceeds(ctx, &quote, &totals).await?;
    check_field_access(
        ctx,
        &provided,
        &quote_snapshot(&base),
        &quote_snapshot(&quote),
    )
    .await?;
    let record = quote_record(quote.id);
    let mut tx = ctx.begin().await?;
    let (mut quote, custom_fields) = saving(
        ctx,
        &mut tx,
        &record,
        WriteMode::Create,
        quote,
        command.custom_fields,
    )
    .await?;
    let custom = CustomFieldWriter::new(&ctx.fields())
        .with_record_values(&quote_snapshot(&quote))
        .apply(&mut tx, scope, &record, &custom_fields, WriteMode::Create)
        .await;
    let custom = merge_violations(validator, custom, INVALID_QUOTE)?;
    quote.number = QuoteStore::of(ctx).next_number(&mut tx).await?;
    QuoteStore::of(ctx).insert(&mut tx, &quote).await?;
    QuoteStore::of(ctx)
        .replace_items(&mut tx, quote.id, &items, now)
        .await?;
    let snapshot = snapshot_with_items(&quote, &items);
    let mut changes = diff_values(&Value::Null, &snapshot, &DiffOptions::default())?;
    changes.extend(custom);
    let record_values = quote_snapshot(&quote);
    let write = RecordWrite {
        record: &record,
        mode: WriteMode::Create,
        changes: &changes,
        values: &record_values,
    };
    let pipeline = ctx.pipeline();
    pipeline.validate(&mut tx, scope, &write).await?;
    pipeline.saved(&mut tx, scope, &write).await?;
    publish_mutation(
        ctx.events(),
        ctx.audit(),
        &mut tx,
        scope,
        now,
        RecordMutation {
            record: record.clone(),
            event_type: QUOTE_CREATED,
            payload: event_payload(&quote, json!({})),
            changes: changes.clone(),
            ip: command.ip,
        },
    )
    .await?;
    tx.commit().await?;
    pipeline.committed(scope, &write).await;
    reload(ctx, quote.id).await
}

pub(crate) async fn update(
    ctx: &Ctx<'_>,
    quote_id: &str,
    command: UpdateQuoteCommand,
) -> Result<QuoteDetails, AppError> {
    let scope = ctx.scope;
    let current = visible(ctx, &Permission::QUOTES_UPDATE, quote_id).await?;
    ensure_version(command.expected_updated_at, current.updated_at)?;
    if let Some(error) = not_editable(&current) {
        return Err(error);
    }
    let today = today(ctx).await?;
    let existing = QuoteStore::of(ctx).items(current.id).await?;
    let mut validator = InputValidator::new();
    let provided = command.input.provided_keys();
    let mut quote = apply_input(ctx, &current, command.input, &mut validator).await?;
    let inputs = match command.items {
        Some(inputs) => Some(inputs),
        None if command.reprice || quote.price_list_id != current.price_list_id => Some(
            existing
                .iter()
                .map(|item| item_input_from_quote(&item.data))
                .collect(),
        ),
        None => None,
    };
    let data = match inputs {
        Some(inputs) => {
            check_item_count(inputs.len())?;
            resolve_items(
                ctx,
                scope,
                inputs,
                &PricingContext {
                    currency: &quote.currency,
                    price_list_id: quote.price_list_id,
                    on: today,
                    reprice: command.reprice,
                },
                &mut validator,
            )
            .await?
        }
        None => existing.iter().map(|item| item.data.clone()).collect(),
    };
    let (totals, items) = price(ctx, &mut quote, data).await?;
    quote.exceeds_discount_limit = exceeds(ctx, &quote, &totals).await?;
    check_field_access(
        ctx,
        &provided,
        &quote_snapshot(&current),
        &quote_snapshot(&quote),
    )
    .await?;
    let record = quote_record(quote.id);
    let mut tx = ctx.begin().await?;
    let locked = QuoteStore::of(ctx)
        .lock(&mut tx, current.id)
        .await?
        .ok_or_else(quote_not_found)?;
    ensure_version(Some(current.updated_at), locked.updated_at)?;
    let (mut quote, custom_fields) = saving(
        ctx,
        &mut tx,
        &record,
        WriteMode::Update,
        quote,
        command.custom_fields,
    )
    .await?;
    let custom = CustomFieldWriter::new(&ctx.fields())
        .with_record_values(&quote_snapshot(&quote))
        .apply(&mut tx, scope, &record, &custom_fields, WriteMode::Update)
        .await;
    let custom = merge_violations(validator, custom, INVALID_QUOTE)?;
    let before = snapshot_with_items(&current, &existing);
    let mut changes = diff_values(
        &before,
        &snapshot_with_items(&quote, &items),
        &DiffOptions::default(),
    )?;
    changes.extend(custom);
    if changes.is_empty() {
        drop(tx);
        return details(ctx, current).await;
    }
    let returned = current.status == QuoteStatus::Approved;
    if returned {
        quote.status = QuoteStatus::Draft;
        changes.push(FieldChange::new(
            "status",
            json!(QuoteStatus::Approved.as_str()),
            json!(QuoteStatus::Draft.as_str()),
        ));
    }
    let record_values = quote_snapshot(&quote);
    let write = RecordWrite {
        record: &record,
        mode: WriteMode::Update,
        changes: &changes,
        values: &record_values,
    };
    let pipeline = ctx.pipeline();
    pipeline.validate(&mut tx, scope, &write).await?;
    let now = ctx.now();
    quote.updated_at = now;
    QuoteStore::of(ctx).update(&mut tx, &quote).await?;
    QuoteStore::of(ctx)
        .replace_items(&mut tx, quote.id, &items, now)
        .await?;
    pipeline.saved(&mut tx, scope, &write).await?;
    if returned {
        QuoteApprovals::of(ctx)
            .cancel_open(&mut tx, scope, &record, "the quote changed after approval")
            .await?;
    }
    publish_mutation(
        ctx.events(),
        ctx.audit(),
        &mut tx,
        scope,
        now,
        RecordMutation {
            record: record.clone(),
            event_type: QUOTE_UPDATED,
            payload: event_payload(&quote, changes_payload(&changes)),
            changes: changes.clone(),
            ip: command.ip,
        },
    )
    .await?;
    tx.commit().await?;
    pipeline.committed(scope, &write).await;
    reload(ctx, quote.id).await
}

pub(crate) async fn delete(
    ctx: &Ctx<'_>,
    quote_id: &str,
    ip: Option<String>,
) -> Result<(), AppError> {
    let scope = ctx.scope;
    let current = visible(ctx, &Permission::QUOTES_DELETE, quote_id).await?;
    let versions = QuoteStore::of(ctx).versions(current.number).await?;
    if versions
        .iter()
        .any(|version| version.status == QuoteStatus::Accepted)
    {
        return Err(AppError::conflict_code(
            QUOTE_ALREADY_ACCEPTED,
            "an accepted quote cannot be deleted",
        ));
    }
    let now = ctx.now();
    let mut tx = ctx.begin().await?;
    let deleted = QuoteStore::of(ctx)
        .delete_number(&mut tx, current.number, now)
        .await?;
    for id in deleted {
        let record = quote_record(id);
        QuoteApprovals::of(ctx)
            .cancel_open(&mut tx, scope, &record, "the quote was deleted")
            .await?;
        let pipeline = ctx.pipeline();
        pipeline.deleting(&mut tx, scope, &record).await?;
        pipeline.deleted(&mut tx, scope, &record).await?;
        publish_mutation(
            ctx.events(),
            ctx.audit(),
            &mut tx,
            scope,
            now,
            RecordMutation {
                record,
                event_type: QUOTE_DELETED,
                payload: json!({
                    "title": current.title,
                    "number": current.number,
                    "deal_id": current.deal_id.to_string(),
                }),
                changes: if id == current.id {
                    diff_values(
                        &quote_snapshot(&current),
                        &Value::Null,
                        &DiffOptions::default(),
                    )?
                } else {
                    Vec::new()
                },
                ip: ip.clone(),
            },
        )
        .await?;
    }
    Ok(tx.commit().await?)
}

pub(crate) async fn create_version(
    ctx: &Ctx<'_>,
    quote_id: &str,
    ip: Option<String>,
) -> Result<QuoteDetails, AppError> {
    let scope = ctx.scope;
    scope.require(&Permission::QUOTES_CREATE)?;
    let current = visible(ctx, &Permission::QUOTES_UPDATE, quote_id).await?;
    if !current.is_current {
        return Err(AppError::conflict_code(
            QUOTE_NOT_CURRENT,
            "a new version can only be created from the current one",
        ));
    }
    if current.status == QuoteStatus::Accepted {
        return Err(AppError::conflict_code(
            QUOTE_ALREADY_ACCEPTED,
            "the quote was already accepted",
        ));
    }
    let existing = QuoteStore::of(ctx).items(current.id).await?;
    let custom_values = CustomFieldReader::new(&ctx.fields())
        .load_unrestricted(&QUOTE, &[current.id])
        .await?
        .remove(&current.id)
        .unwrap_or_default();
    let now = ctx.now();
    let mut next = Quote {
        id: Uuid::now_v7(),
        version: current.version + 1,
        is_current: true,
        status: QuoteStatus::Draft,
        public_key: None,
        sent_at: None,
        sent_by: None,
        viewed_at: None,
        accepted_at: None,
        rejected_at: None,
        expired_at: None,
        acceptance: None,
        rejection_reason: None,
        deal_synced_at: None,
        deal_sync_error: None,
        created_by: Some(scope.user_id().clone()),
        created_at: now,
        updated_at: now,
        ..current.clone()
    };
    let totals = compute_totals(
        &existing
            .iter()
            .map(|item| item.data.clone())
            .collect::<Vec<_>>(),
        next.discount,
    )
    .map_err(totals_error)?;
    next.apply_totals(&totals);
    next.exceeds_discount_limit = exceeds(ctx, &next, &totals).await?;
    let items = build_items(
        next.id,
        existing.iter().map(|item| item.data.clone()).collect(),
        &totals,
    );
    let previous = quote_record(current.id);
    let record = quote_record(next.id);
    let mut tx = ctx.begin().await?;
    QuoteStore::of(ctx)
        .lock(&mut tx, current.id)
        .await?
        .filter(|locked| locked.is_current)
        .ok_or_else(|| {
            AppError::conflict_code(
                QUOTE_NOT_CURRENT,
                "a new version can only be created from the current one",
            )
        })?;
    QuoteApprovals::of(ctx)
        .cancel_open(
            &mut tx,
            scope,
            &previous,
            "a new version of the quote was created",
        )
        .await?;
    QuoteStore::of(ctx)
        .retire_versions(&mut tx, current.number, now)
        .await?;
    let (next, custom_values) = saving(
        ctx,
        &mut tx,
        &record,
        WriteMode::Create,
        next,
        custom_values,
    )
    .await?;
    QuoteStore::of(ctx).insert(&mut tx, &next).await?;
    QuoteStore::of(ctx)
        .replace_items(&mut tx, next.id, &items, now)
        .await?;
    let custom = CustomFieldWriter::new(&ctx.fields())
        .with_record_values(&quote_snapshot(&next))
        .apply(&mut tx, scope, &record, &custom_values, WriteMode::Create)
        .await
        .unwrap_or_default();
    let mut changes = diff_values(
        &Value::Null,
        &snapshot_with_items(&next, &items),
        &DiffOptions::default(),
    )?;
    changes.extend(custom);
    let record_values = quote_snapshot(&next);
    let write = RecordWrite {
        record: &record,
        mode: WriteMode::Create,
        changes: &changes,
        values: &record_values,
    };
    let pipeline = ctx.pipeline();
    pipeline.validate(&mut tx, scope, &write).await?;
    pipeline.saved(&mut tx, scope, &write).await?;
    publish_mutation(
        ctx.events(),
        ctx.audit(),
        &mut tx,
        scope,
        now,
        RecordMutation {
            record: record.clone(),
            event_type: QUOTE_VERSION_CREATED,
            payload: event_payload(
                &next,
                json!({
                    "previous_id": current.id.to_string(),
                    "previous_version": current.version,
                    "previous_status": current.status.as_str(),
                }),
            ),
            changes: changes.clone(),
            ip,
        },
    )
    .await?;
    tx.commit().await?;
    pipeline.committed(scope, &write).await;
    reload(ctx, next.id).await
}

pub(crate) async fn saving(
    ctx: &Ctx<'_>,
    tx: &mut MySqlTx,
    record: &RecordRef,
    mode: WriteMode,
    quote: Quote,
    custom_fields: Map<String, Value>,
) -> Result<(Quote, Map<String, Value>), AppError> {
    let scope = ctx.scope;
    let values = quote_snapshot(&quote);
    let draft = ctx
        .fields()
        .pipeline()
        .saving(
            tx,
            scope,
            RecordDraft::new(record.clone(), mode, &values, custom_fields),
        )
        .await?;
    let changed = draft.changed_values(&values);
    if changed.is_empty() {
        return Ok((quote, draft.custom_fields));
    }
    if let Some(key) = changed
        .keys()
        .find(|key| !SAVING_FIELDS.contains(&key.as_str()))
    {
        return Err(saving_unsupported(record, key));
    }
    let input = QuoteInput {
        title: value_patch(&changed, "title"),
        owner_id: value_patch(&changed, "owner_id"),
        valid_until: value_patch(&changed, "valid_until"),
        terms: value_patch(&changed, "terms"),
        notes: value_patch(&changed, "notes"),
        ..QuoteInput::default()
    };
    let mut validator = InputValidator::new();
    let quote = apply_input(ctx, &quote, input, &mut validator).await?;
    validator.finish(INVALID_QUOTE)?;
    Ok((quote, draft.custom_fields))
}

const SAVING_FIELDS: [&str; 5] = ["title", "owner_id", "valid_until", "terms", "notes"];

pub(crate) async fn settings(ctx: &Ctx<'_>) -> Result<QuoteSettings, AppError> {
    let scope = ctx.scope;
    if scope.require(&Permission::SETTINGS_MANAGE).is_err() {
        scope.require(&Permission::QUOTES_READ)?;
    }
    QuoteStore::of(ctx).settings().await
}

pub(crate) async fn update_settings(
    ctx: &Ctx<'_>,
    command: UpdateQuoteSettingsCommand,
) -> Result<QuoteSettings, AppError> {
    let scope = ctx.scope;
    scope.require(&Permission::SETTINGS_MANAGE)?;
    let current = QuoteStore::of(ctx).settings().await?;
    if let Some(updated_at) = current.updated_at {
        ensure_version(command.expected_updated_at, updated_at)?;
    }
    let mut validator = InputValidator::new();
    let mut settings = current.clone();
    if let Some(days) = command.default_valid_days {
        match days {
            Some(days) if days == 0 || days > MAX_DEFAULT_VALID_DAYS => validator.violation(
                "default_valid_days",
                format!("must be between 1 and {MAX_DEFAULT_VALID_DAYS}"),
            ),
            days => settings.default_valid_days = days,
        }
    }
    if let Some(terms) = command.default_terms {
        settings.default_terms = validator.text("default_terms", terms, MAX_QUOTE_TEXT_LENGTH);
    }
    if let Some(stages) = command.accept_stages {
        let mut parsed = BTreeMap::new();
        for (pipeline, stage) in stages {
            let field = format!("accept_stages.{pipeline}");
            let Some(pipeline_id) = parse_id(&pipeline) else {
                validator.violation(&field, "must be keyed by a pipeline id");
                continue;
            };
            let Some(stage) = stage.filter(|stage| !stage.trim().is_empty()) else {
                continue;
            };
            match parse_id(&stage) {
                Some(stage_id)
                    if QuoteStore::of(ctx)
                        .stage_in_pipeline(pipeline_id, stage_id)
                        .await? =>
                {
                    parsed.insert(pipeline_id, stage_id);
                }
                _ => validator.violation(&field, "must be a stage of that pipeline"),
            }
        }
        settings.accept_stages = parsed;
    }
    validator.finish("invalid quote settings")?;
    let stages_json = |settings: &QuoteSettings| {
        json!(
            settings
                .accept_stages
                .iter()
                .map(|(pipeline, stage)| (pipeline.to_string(), json!(stage.to_string())))
                .collect::<Map<String, Value>>()
        )
    };
    let snapshot = |settings: &QuoteSettings| {
        json!({
            "default_valid_days": settings.default_valid_days,
            "default_terms": settings.default_terms,
            "accept_stages": stages_json(settings),
        })
    };
    let changes = diff_values(
        &snapshot(&current),
        &snapshot(&settings),
        &DiffOptions::default(),
    )?;
    if changes.is_empty() && current.updated_at.is_some() {
        return Ok(current);
    }
    let now = ctx.now();
    settings.updated_at = Some(now);
    settings.updated_by = Some(scope.user_id().clone());
    let mut tx = ctx.begin().await?;
    QuoteStore::of(ctx)
        .save_settings(&mut tx, &settings)
        .await?;
    let record_id = Uuid::nil();
    publish_mutation(
        ctx.events(),
        ctx.audit(),
        &mut tx,
        scope,
        now,
        RecordMutation {
            record: RecordRef::new(RecordType::from_static("quotes.quote_settings"), record_id),
            event_type: QUOTE_SETTINGS_UPDATED,
            payload: changes_payload(&changes),
            changes,
            ip: command.ip,
        },
    )
    .await?;
    tx.commit().await?;
    Ok(settings)
}

pub(crate) async fn visible(
    ctx: &Ctx<'_>,
    permission: &Permission,
    quote_id: &str,
) -> Result<Quote, AppError> {
    let scope = ctx.scope;
    let visibility = scope.record_filter(permission)?;
    let id = parse_record_id(quote_id, quote_not_found)?;
    QuoteStore::of(ctx)
        .find(scope, &visibility, id)
        .await?
        .ok_or_else(quote_not_found)
}

pub(crate) async fn any(ctx: &Ctx<'_>, id: Uuid) -> Result<Quote, AppError> {
    let scope = ctx.scope;
    QuoteStore::of(ctx)
        .find(scope, &RecordScopeFilter::Organization, id)
        .await?
        .ok_or_else(quote_not_found)
}

pub(crate) async fn today(ctx: &Ctx<'_>) -> Result<NaiveDate, AppError> {
    let organization = QuoteStore::of(ctx).organization().await?;
    Ok(ctx
        .now()
        .with_timezone(&timezone(&organization.timezone))
        .date_naive())
}

pub(crate) async fn discount_limit(
    ctx: &Ctx<'_>,
    quote: &Quote,
) -> Result<Option<Decimal>, AppError> {
    let scope = ctx.scope;
    match quote.owner_id.as_ref().or(quote.created_by.as_ref()) {
        Some(owner) => {
            QuoteCatalog::of(ctx)
                .max_discount_percent(scope, owner)
                .await
        }
        None => Ok(None),
    }
}

pub(crate) async fn exceeds(
    ctx: &Ctx<'_>,
    quote: &Quote,
    totals: &TotalsBreakdown,
) -> Result<bool, AppError> {
    Ok(exceeds_discount_limit(
        totals,
        discount_limit(ctx, quote).await?,
    ))
}

pub(crate) async fn price(
    ctx: &Ctx<'_>,
    quote: &mut Quote,
    data: Vec<QuoteItemData>,
) -> Result<(TotalsBreakdown, Vec<QuoteItem>), AppError> {
    let totals = compute_totals(&data, quote.discount).map_err(totals_error)?;
    let totals = filter_totals(ctx, quote, &data, totals).await?;
    quote.apply_totals(&totals);
    let items = build_items(quote.id, data, &totals);
    Ok((totals, items))
}

async fn filter_totals(
    ctx: &Ctx<'_>,
    quote: &Quote,
    items: &[QuoteItemData],
    totals: TotalsBreakdown,
) -> Result<TotalsBreakdown, AppError> {
    if !ctx.app().hooks().has_filters_except(QuoteTotals, None) {
        return Ok(totals);
    }
    let lines = totals.lines.len();
    let mut tx = ctx.begin().await?;
    let priced = ctx
        .apply_hook(
            QuoteTotals,
            &mut tx,
            KEY,
            PricedQuote {
                quote_id: quote.id,
                deal_id: quote.deal_id,
                currency: quote.currency.clone(),
                discount: quote.discount,
                items: items.to_vec(),
                totals,
            },
        )
        .await?;
    tx.rollback().await?;
    if priced.totals.lines.len() != lines {
        return Err(AppError::Internal(
            "a quote totals hook changed the number of lines".to_owned(),
        ));
    }
    Ok(priced.totals)
}

pub(crate) fn check_item_count(count: usize) -> Result<(), AppError> {
    if count > MAX_QUOTE_ITEMS {
        return Err(AppError::validation_code(
            QUOTE_ITEMS_LIMIT_REACHED,
            format!("a quote has at most {MAX_QUOTE_ITEMS} items"),
        ));
    }
    Ok(())
}

pub(crate) async fn apply_input(
    ctx: &Ctx<'_>,
    base: &Quote,
    input: QuoteInput,
    validator: &mut InputValidator,
) -> Result<Quote, AppError> {
    let scope = ctx.scope;
    let mut quote = base.clone();
    if let Some(title) = input.title {
        quote.title = validator.required_text("title", title, MAX_QUOTE_TITLE_LENGTH);
    }
    if let Some(owner) = input.owner_id {
        match owner.as_deref().map(str::trim) {
            None | Some("") => quote.owner_id = None,
            Some(text) => match UserId::new(text.to_owned()) {
                Ok(user) => {
                    let exists = !ctx
                        .fields()
                        .references()
                        .existing_users(std::slice::from_ref(&user))
                        .await?
                        .is_empty();
                    if exists {
                        quote.owner_id = Some(user);
                    } else {
                        validator.violation("owner_id", "user was not found");
                    }
                }
                Err(_) => validator.violation("owner_id", "must be a valid user id"),
            },
        }
    }
    if let Some(price_list) = input.price_list_id {
        match price_list.as_deref().map(str::trim) {
            None | Some("") => quote.price_list_id = None,
            Some(text) => match parse_id(text) {
                Some(id) => match QuoteCatalog::of(ctx).find(scope, id).await? {
                    Some(found) if found.currency == quote.currency => {
                        quote.price_list_id = Some(found.id);
                    }
                    Some(found) => validator.violation(
                        "price_list_id",
                        format!(
                            "the price list uses {} but the quote uses {}",
                            found.currency, quote.currency
                        ),
                    ),
                    None => {
                        return Err(AppError::invalid_fields_code(
                            QUOTE_PRICE_LIST_NOT_FOUND,
                            "the price list was not found",
                            vec![FieldViolation::new("price_list_id", "was not found")],
                        ));
                    }
                },
                None => validator.violation("price_list_id", "must be a valid id"),
            },
        }
    }
    if let Some(valid_until) = input.valid_until {
        match valid_until.as_deref().map(str::trim) {
            None | Some("") => quote.valid_until = None,
            Some(text) => match parse_date(text) {
                Some(date) => quote.valid_until = Some(date),
                None => validator.violation("valid_until", "must be a date (YYYY-MM-DD)"),
            },
        }
    }
    if let Some(terms) = input.terms {
        quote.terms = validator.text("terms", terms, MAX_QUOTE_TEXT_LENGTH);
    }
    if let Some(notes) = input.notes {
        quote.notes = validator.text("notes", notes, MAX_QUOTE_TEXT_LENGTH);
    }
    if let Some(kind) = input.discount_type {
        match kind.as_deref().map(str::trim) {
            None | Some("") => quote.discount.kind = DiscountKind::None,
            Some(text) => match DiscountKind::parse(text) {
                Some(kind) => quote.discount.kind = kind,
                None => validator.violation("discount_type", "must be none, percent or amount"),
            },
        }
    }
    if let Some(value) = input.discount_value {
        match value.as_deref().map(str::trim) {
            None | Some("") => quote.discount.value = Decimal::ZERO,
            Some(text) => match parse_money(text) {
                Some(value) => quote.discount.value = value,
                None => validator.violation(
                    "discount_value",
                    "must be a non-negative number with at most 2 decimal places",
                ),
            },
        }
    }
    if quote.discount.kind == DiscountKind::None {
        quote.discount.value = Decimal::ZERO;
    }
    if quote.discount.kind == DiscountKind::Percent && quote.discount.value > Decimal::ONE_HUNDRED {
        validator.violation("discount_value", "must be a percentage between 0 and 100");
    }
    Ok(quote)
}

pub(crate) async fn check_field_access(
    ctx: &Ctx<'_>,
    provided: &[&str],
    old: &Value,
    new: &Value,
) -> Result<(), AppError> {
    let scope = ctx.scope;
    check_record_writes(
        ctx.fields().definitions(),
        scope,
        &QUOTE,
        provided,
        old,
        new,
    )
    .await
}

pub(crate) fn event_payload(quote: &Quote, extra: Value) -> Value {
    let mut payload = json!({
        "number": quote.number,
        "version": quote.version,
        "title": quote.title,
        "status": quote.status.as_str(),
        "deal_id": quote.deal_id.to_string(),
        "owner_id": quote.owner_id.as_ref().map(UserId::as_str),
        "currency": quote.currency,
        "total": money_text(quote.total),
        "records": [{ "record_type": RecordType::DEAL.as_str(), "record_id": quote.deal_id }],
    });
    if let (Value::Object(target), Value::Object(extra)) = (&mut payload, extra) {
        target.extend(extra);
    }
    payload
}

pub(crate) async fn record_system_event(
    ctx: &Ctx<'_>,
    tx: &mut MySqlTx,
    quote: &Quote,
    event: SystemEvent<'_>,
) -> Result<(), AppError> {
    let SystemEvent {
        event_type,
        extra,
        changes,
        ip,
    } = event;
    let now = ctx.now();
    let record = quote_record(quote.id);
    ctx.events()
        .publish(
            tx,
            DomainEvent::new(Actor::System, event_type, now)
                .with_record(record.clone())
                .with_payload(event_payload(quote, extra)),
        )
        .await?;
    ctx.audit()
        .record(
            tx,
            AuditEntry::new(Actor::System, event_type, now)
                .with_record(record)
                .with_changes(changes)
                .with_ip(ip),
        )
        .await
}

pub(crate) async fn record_deal_event(
    ctx: &Ctx<'_>,
    tx: &mut MySqlTx,
    actor: Actor,
    quote: &Quote,
    event_type: &str,
    extra: Value,
) -> Result<(), AppError> {
    let now = ctx.now();
    let mut payload = json!({
        "quote_id": quote.id.to_string(),
        "number": quote.number,
        "version": quote.version,
        "title": quote.title,
        "currency": quote.currency,
        "total": money_text(quote.total),
    });
    if let (Value::Object(target), Value::Object(extra)) = (&mut payload, extra) {
        target.extend(extra);
    }
    ctx.events()
        .publish(
            tx,
            DomainEvent::new(actor, event_type, now)
                .with_record(RecordRef::new(RecordType::DEAL, quote.deal_id))
                .with_payload(payload),
        )
        .await
}

fn key_reference() -> SecretKey {
    SecretKey::new(TOKEN_KEY_OWNER, ORGANIZATION_RECORD_ID, TOKEN_KEY_NAME)
}

pub(crate) async fn existing_token_key(ctx: &Ctx<'_>) -> Result<Option<Vec<u8>>, AppError> {
    let scope = ctx.scope;
    Ok(ctx
        .state()
        .secrets
        .reveal(scope, &key_reference())
        .await?
        .and_then(|secret| decode_key(secret.expose())))
}

pub(crate) async fn token_key(ctx: &Ctx<'_>, tx: &mut MySqlTx) -> Result<Vec<u8>, AppError> {
    if let Some(key) = existing_token_key(ctx).await? {
        return Ok(key);
    }
    sqlx::query("SELECT id FROM organization_settings WHERE id = 1 FOR UPDATE")
        .fetch_one(&mut **tx)
        .await?;
    if let Some(key) = existing_token_key(ctx).await? {
        return Ok(key);
    }
    let encoded = generate_key()?;
    let key = decode_key(&encoded)
        .ok_or_else(|| AppError::Internal("generated quote link key is invalid".to_owned()))?;
    ctx.state()
        .secrets
        .store(tx, ctx.scope, &key_reference(), &Secret::new(encoded))
        .await?;
    Ok(key)
}

pub(crate) fn link_url(ctx: &Ctx<'_>, public_key: &str, key: &[u8]) -> Result<String, AppError> {
    Ok(format!(
        "{}{QUOTE_LINK_PATH}/{}",
        quote_base_url(ctx).trim_end_matches('/'),
        encode_link_token(public_key, key)?
    ))
}

pub(crate) async fn public_url(ctx: &Ctx<'_>, quote: &Quote) -> Result<Option<String>, AppError> {
    let Some(public_key) = &quote.public_key else {
        return Ok(None);
    };
    match existing_token_key(ctx).await? {
        Some(key) => link_url(ctx, public_key, &key).map(Some),
        None => Ok(None),
    }
}

pub(crate) async fn reload(ctx: &Ctx<'_>, id: Uuid) -> Result<QuoteDetails, AppError> {
    let quote = any(ctx, id).await?;
    details(ctx, quote).await
}

pub(crate) async fn details(ctx: &Ctx<'_>, quote: Quote) -> Result<QuoteDetails, AppError> {
    let scope = ctx.scope;
    let items = QuoteStore::of(ctx).items(quote.id).await?;
    let versions = QuoteStore::of(ctx).versions(quote.number).await?;
    let custom_fields = CustomFieldReader::new(&ctx.fields())
        .load(scope, &QUOTE, &[quote.id])
        .await?
        .remove(&quote.id)
        .unwrap_or_default();
    let totals = compute_totals(
        &items
            .iter()
            .map(|item| item.data.clone())
            .collect::<Vec<_>>(),
        quote.discount,
    )
    .unwrap_or_default();
    let discount_limit = discount_limit(ctx, &quote).await?;
    let public_url = public_url(ctx, &quote).await?;
    Ok(QuoteDetails {
        items_discount: totals.items_discount,
        total_discount: totals.total_discount,
        max_discount_percent: max_discount_percent(&totals).round_dp(2),
        discount_limit,
        items,
        versions,
        custom_fields,
        public_url,
        quote,
    })
}

pub(crate) async fn summaries(
    ctx: &Ctx<'_>,
    quotes: Vec<Quote>,
) -> Result<Vec<QuoteSummary>, AppError> {
    let scope = ctx.scope;
    let ids: Vec<Uuid> = quotes.iter().map(|quote| quote.id).collect();
    let mut values = CustomFieldReader::new(&ctx.fields())
        .load(scope, &QUOTE, &ids)
        .await?;
    Ok(quotes
        .into_iter()
        .map(|quote| QuoteSummary {
            custom_fields: values.remove(&quote.id).unwrap_or_default(),
            quote,
        })
        .collect())
}

pub(crate) fn now(ctx: &Ctx<'_>) -> DateTime<Utc> {
    ctx.now()
}
