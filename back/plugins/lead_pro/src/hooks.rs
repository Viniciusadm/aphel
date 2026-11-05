use aphel_contract_crm::CrmRecordTypes;
use serde_json::{Value, json};

use aphel_kernel::errors::{AppError, FieldViolation};
use aphel_kernel::modules::HookCtx;
use aphel_kernel::records::{ChangedRecord, SavingRecord};
use aphel_sdk::leads::{Lead, LeadConversion};
use aphel_sdk::platform::application::fields::WriteMode;
use aphel_sdk::platform::application::shared::RecordType;

use crate::domain::{LEAD_NOT_HOT_ENOUGH, LeadProSettings, MAX_TEMPERATURE, TEMPERATURE_FIELD};
use crate::store;

pub(crate) async fn default_temperature(
    ctx: &mut HookCtx<'_>,
    mut saving: SavingRecord,
) -> Result<SavingRecord, AppError> {
    if saving.record_type != RecordType::CONTACT || saving.draft.mode != WriteMode::Create {
        return Ok(saving);
    }
    let missing = saving
        .draft
        .values
        .get(TEMPERATURE_FIELD)
        .is_none_or(Value::is_null);
    if missing {
        let settings = LeadProSettings::load(ctx.pool).await?;
        saving.draft.values.insert(
            TEMPERATURE_FIELD.to_owned(),
            json!(settings.initial_temperature),
        );
    }
    Ok(saving)
}

pub(crate) async fn validate_temperature(changed: &ChangedRecord) -> Result<(), AppError> {
    if changed.record_type != RecordType::CONTACT {
        return Ok(());
    }
    let write = changed.write();
    let Some(change) = write.changed(TEMPERATURE_FIELD) else {
        return Ok(());
    };
    let valid = match &change.new {
        Value::Null => true,
        value => value
            .as_f64()
            .or_else(|| value.as_str().and_then(|text| text.trim().parse().ok()))
            .is_some_and(|number| {
                number.fract() == 0.0 && (0.0..=MAX_TEMPERATURE as f64).contains(&number)
            }),
    };
    if valid {
        return Ok(());
    }
    Err(AppError::invalid_fields(
        "invalid contact",
        vec![FieldViolation::new(
            TEMPERATURE_FIELD,
            format!("must be a whole number from 0 to {MAX_TEMPERATURE}"),
        )],
    ))
}

pub(crate) async fn require_temperature(
    ctx: &mut HookCtx<'_>,
    lead: &Lead,
) -> Result<(), AppError> {
    let settings = LeadProSettings::load(ctx.pool).await?;
    let temperature = store::temperature(ctx.tx(), lead.id()).await?;
    if settings.is_hot(temperature) {
        return Ok(());
    }
    Err(AppError::validation_code(
        LEAD_NOT_HOT_ENOUGH,
        format!(
            "the lead temperature is {} and qualifying needs at least {}",
            temperature.map_or_else(|| "unknown".to_owned(), |value| value.to_string()),
            settings.hot_threshold
        ),
    ))
}

pub(crate) async fn enrich_deal_values(
    ctx: &mut HookCtx<'_>,
    mut conversion: LeadConversion,
) -> Result<LeadConversion, AppError> {
    let settings = LeadProSettings::load(ctx.pool).await?;
    let temperature = store::temperature(ctx.tx(), conversion.lead.id()).await?;
    let label = match temperature {
        Some(value) if settings.is_hot(Some(value)) => format!("Lead Pro: quente ({value})"),
        Some(value) => format!("Lead Pro: temperatura {value}"),
        None => "Lead Pro".to_owned(),
    };
    let source = match conversion.deal.get("source").and_then(Value::as_str) {
        Some(current) if !current.trim().is_empty() => format!("{} · {label}", current.trim()),
        _ => label,
    };
    conversion.deal.insert("source".to_owned(), json!(source));
    Ok(conversion)
}
