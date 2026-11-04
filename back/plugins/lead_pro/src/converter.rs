use std::sync::Arc;

use async_trait::async_trait;

use aphel_kernel::Ctx;
use aphel_kernel::database::transaction::MySqlTx;
use aphel_kernel::errors::AppError;
use aphel_sdk::leads::{Conversion, ConvertLeadCommand};
use aphel_sdk::leads::{Lead, LeadConverter};

use crate::domain::CONVERTED_TOUCH;
use crate::store;

pub(crate) struct WithTouches(pub(crate) Arc<dyn LeadConverter>);

#[async_trait]
impl LeadConverter for WithTouches {
    async fn convert(
        &self,
        ctx: &Ctx<'_>,
        tx: &mut MySqlTx,
        lead: &Lead,
        command: &ConvertLeadCommand,
    ) -> Result<Conversion, AppError> {
        let conversion = self.0.convert(ctx, tx, lead, command).await?;
        store::insert_touch(tx, lead.id(), CONVERTED_TOUCH, None, ctx.now()).await?;
        Ok(conversion)
    }
}
