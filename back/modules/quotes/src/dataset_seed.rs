use std::collections::BTreeMap;

use aphel_sdk::maintenance::{DatasetSeed, DatasetSession};
use aphel_sdk::shared::{RecordRef, RecordType};
use aphel_sdk::{AppError, Ctx};
use async_trait::async_trait;
use chrono::Duration;

use crate::domain::{CreateQuoteCommand, QuoteInput};

const QUOTE: RecordType = RecordType::from_static("quotes.quote");
const DEALS_WITH_ITEMS_VALUE: &str = "line_items.deals";
const DEALS_WITH_QUOTE: usize = 3;

pub(crate) struct QuotesSeed;

#[async_trait]
impl DatasetSeed for QuotesSeed {
    fn order(&self) -> i32 {
        520
    }

    fn entities(&self) -> Vec<RecordType> {
        vec![QUOTE]
    }

    async fn seed(&self, ctx: &Ctx<'_>, session: &mut DatasetSession) -> Result<(), AppError> {
        let deals: Vec<String> = session
            .value(DEALS_WITH_ITEMS_VALUE)
            .unwrap_or_default()
            .split(',')
            .filter(|id| !id.is_empty())
            .take(DEALS_WITH_QUOTE)
            .map(str::to_owned)
            .collect();
        let now = session.now();
        let valid_until = (now.date_naive() + Duration::days(30))
            .format("%Y-%m-%d")
            .to_string();
        for deal_id in deals {
            let quote = crate::service::create(
                ctx,
                CreateQuoteCommand {
                    deal_id,
                    from_line_items: true,
                    input: QuoteInput {
                        valid_until: Some(Some(valid_until.clone())),
                        terms: Some(Some(
                            "Pagamento em 30 dias. Proposta válida pelo prazo indicado.".to_owned(),
                        )),
                        ..QuoteInput::default()
                    },
                    ..CreateQuoteCommand::default()
                },
            )
            .await?;
            session.record(
                RecordRef::new(QUOTE, quote.quote.id),
                now,
                now,
                BTreeMap::new(),
            );
        }
        Ok(())
    }
}
