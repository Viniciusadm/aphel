use std::sync::Arc;

use async_trait::async_trait;

use leads::hooks::{Conversion, ConvertLeadCommand};
use leads::{Lead, LeadConverter};
use aphel_kernel::Ctx;
use aphel_kernel::errors::AppError;

use crate::domain::CONVERTED_TOUCH;
use crate::store;

pub(crate) struct WithTouches(pub(crate) Arc<dyn LeadConverter>);

#[async_trait]
impl LeadConverter for WithTouches {
    async fn convert(
        &self,
        ctx: &Ctx<'_>,
        lead: &Lead,
        command: &ConvertLeadCommand,
    ) -> Result<Conversion, AppError> {
        let conversion = self.0.convert(ctx, lead, command).await?;
        let mut tx = ctx.begin().await?;
        store::insert_touch(&mut tx, lead.id(), CONVERTED_TOUCH, None, ctx.now()).await?;
        tx.commit().await?;
        Ok(conversion)
    }
}
