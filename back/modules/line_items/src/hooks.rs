use crate::domain::RECORD_CURRENCY_HAS_ITEMS;
use aphel_kernel::{
    application::fields::WriteMode,
    errors::{AppError, FieldViolation},
    modules::HookCtx,
    records::ChangedRecord,
};
use aphel_sdk::line_items::LineItemTraits;
pub(crate) const CURRENCY_LOCK_PRIORITY: i32 = 4;

pub(crate) async fn block_currency_change_with_items(
    ctx: &mut HookCtx<'_>,
    args: &ChangedRecord,
) -> Result<(), AppError> {
    if args.mode != WriteMode::Update {
        return Ok(());
    }
    let Some(mapping) = args
        .registry
        .require(args.record_type.as_str())?
        .capabilities
        .line_item_mapping()?
    else {
        return Ok(());
    };
    if args.write().changed(&mapping.currency_field).is_none() {
        return Ok(());
    }
    let has_items = crate::store::record_has_items(
        ctx.tx(),
        args.record.record_type.as_str(),
        &args.record.record_id.to_string(),
    )
    .await?;
    if !has_items {
        return Ok(());
    }
    Err(AppError::invalid_fields_code(
        RECORD_CURRENCY_HAS_ITEMS,
        "the currency of a record with items cannot be changed",
        vec![FieldViolation::new(
            &mapping.currency_field,
            "remove the items of the record before changing its currency",
        )],
    ))
}
