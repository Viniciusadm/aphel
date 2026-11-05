use std::collections::BTreeMap;

use aphel_kernel::errors::CodedKind;
use aphel_sdk::maintenance::{DatasetSeed, DatasetSession, ids_of};
use aphel_sdk::platform::database::transaction::MySqlTx;
use aphel_sdk::shared::{RecordRef, RecordType};
use aphel_sdk::{AppError, Ctx};
use async_trait::async_trait;
use chrono::Duration;
use sqlx::QueryBuilder;

use crate::domain::{CreateExchangeRateCommand, SetCurrenciesCommand};

const EXCHANGE_RATE: RecordType = RecordType::from_static("money.exchange_rate");
const FOREIGN_CURRENCIES: [(&str, &str); 2] = [("USD", "5.4500"), ("EUR", "5.9000")];
const FOREIGN_CURRENCY_VALUE: &str = "money.foreign_currency";

pub(crate) struct MoneySeed;

fn is_conflict(error: &AppError) -> bool {
    matches!(
        error,
        AppError::Conflict(_)
            | AppError::Coded {
                kind: CodedKind::Conflict,
                ..
            }
    )
}

#[async_trait]
impl DatasetSeed for MoneySeed {
    fn order(&self) -> i32 {
        50
    }

    fn entities(&self) -> Vec<RecordType> {
        vec![EXCHANGE_RATE]
    }

    async fn seed(&self, ctx: &Ctx<'_>, session: &mut DatasetSession) -> Result<(), AppError> {
        let settings = crate::service::settings(ctx).await?;
        let Some((code, rate)) = FOREIGN_CURRENCIES
            .into_iter()
            .find(|(code, _)| *code != settings.base)
        else {
            return Ok(());
        };
        let mut codes: Vec<String> = settings
            .currencies
            .iter()
            .filter(|currency| !currency.is_base)
            .map(|currency| currency.code.clone())
            .collect();
        if !codes.iter().any(|existing| existing == code) {
            codes.push(code.to_owned());
            crate::service::set_currencies(
                ctx,
                SetCurrenciesCommand {
                    currencies: codes,
                    ip: None,
                },
            )
            .await?;
        }
        session.set_value(FOREIGN_CURRENCY_VALUE, code.to_owned());
        let now = session.now();
        let today = now.date_naive();
        for days in [90, 30, 0] {
            let effective_date = (today - Duration::days(days))
                .format("%Y-%m-%d")
                .to_string();
            match crate::service::create_rate(
                ctx,
                CreateExchangeRateCommand {
                    currency: code.to_owned(),
                    effective_date: Some(effective_date),
                    rate: rate.to_owned(),
                    ip: None,
                },
            )
            .await
            {
                Ok(created) => session.record(
                    RecordRef::new(EXCHANGE_RATE, created.id),
                    now,
                    now,
                    BTreeMap::new(),
                ),
                Err(error) if is_conflict(&error) => {}
                Err(error) => return Err(error),
            }
        }
        Ok(())
    }

    async fn cleanup(
        &self,
        tx: &mut MySqlTx,
        records: Option<&[RecordRef]>,
    ) -> Result<bool, AppError> {
        let Some(records) = records else {
            sqlx::query("DELETE FROM money_exchange_rates")
                .execute(&mut **tx)
                .await?;
            return Ok(true);
        };
        let ids = ids_of(records, &EXCHANGE_RATE);
        if !ids.is_empty() {
            let mut delete = QueryBuilder::new("DELETE FROM money_exchange_rates WHERE id IN (");
            let mut list = delete.separated(", ");
            for id in ids {
                list.push_bind(id);
            }
            delete.push(")");
            delete.build().execute(&mut **tx).await?;
        }
        Ok(true)
    }
}
