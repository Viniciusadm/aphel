use std::collections::BTreeMap;

use aphel_sdk::maintenance::{DatasetSeed, DatasetSession, ids_of};
use aphel_sdk::platform::database::transaction::MySqlTx;
use aphel_sdk::shared::{RecordRef, RecordType};
use aphel_sdk::{AppError, Ctx};
use async_trait::async_trait;
use sqlx::QueryBuilder;

use crate::application::{CreateValidationRuleCommand, ValidationRuleUseCases};
use crate::infrastructure::MySqlValidationRulePorts;

const RULE: RecordType = RecordType::from_static("validation_rule");
const RULE_KEY: &str = "demo_large_deal_source";
const EXPRESSION: &str = "amount > 10000 and is_blank(source)";
const DEAL_RECORD_TYPE_VALUE: &str = "deals.record_type";

pub(crate) struct ValidationRulesSeed;

#[async_trait]
impl DatasetSeed for ValidationRulesSeed {
    fn order(&self) -> i32 {
        540
    }

    fn entities(&self) -> Vec<RecordType> {
        vec![RULE]
    }

    async fn seed(&self, ctx: &Ctx<'_>, session: &mut DatasetSession) -> Result<(), AppError> {
        let Some(deal_type) = session.value(DEAL_RECORD_TYPE_VALUE).map(str::to_owned) else {
            return Ok(());
        };
        let existing: Option<i64> =
            sqlx::query_scalar("SELECT 1 FROM validation_rules WHERE `key` = ? LIMIT 1")
                .bind(RULE_KEY)
                .fetch_optional(ctx.pool())
                .await?;
        if existing.is_some() {
            return Ok(());
        }
        let rule = ValidationRuleUseCases::new(MySqlValidationRulePorts::new(
            ctx.pool().clone(),
            ctx.registry().clone(),
            ctx.events().clone(),
            ctx.audit().clone(),
        ))
        .create(
            ctx.scope,
            CreateValidationRuleCommand {
                record_type: deal_type,
                key: Some(RULE_KEY.to_owned()),
                name: "Origem obrigatória acima de 10 mil".to_owned(),
                expression: EXPRESSION.to_owned(),
                message: "Informe a origem para oportunidades acima de 10 mil".to_owned(),
                field: Some("source".to_owned()),
                applies_on: None,
                is_active: Some(true),
                ip: None,
            },
        )
        .await?;
        let now = session.now();
        session.record(RecordRef::new(RULE, rule.id), now, now, BTreeMap::new());
        Ok(())
    }

    async fn cleanup(
        &self,
        tx: &mut MySqlTx,
        records: Option<&[RecordRef]>,
    ) -> Result<bool, AppError> {
        let Some(records) = records else {
            sqlx::query("DELETE FROM validation_rules")
                .execute(&mut **tx)
                .await?;
            return Ok(true);
        };
        let ids = ids_of(records, &RULE);
        if !ids.is_empty() {
            let mut delete = QueryBuilder::new("DELETE FROM validation_rules WHERE id IN (");
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
