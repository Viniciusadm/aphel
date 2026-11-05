use std::collections::BTreeMap;

use aphel_contract_crm::CrmRecordTypes;
use aphel_kernel::database::transaction::MySqlTx;
use aphel_sdk::maintenance::{DatasetSeed, DatasetSession, ids_of, push_record_tuples};
use aphel_sdk::shared::{RecordRef, RecordType};
use aphel_sdk::{AppError, Ctx};
use async_trait::async_trait;
use sqlx::QueryBuilder;

use crate::domain::{PolicyCommand, StepInput};

const POLICY: RecordType = RecordType::from_static("approvals.approval_policy");
const POLICY_KEY: &str = "demo_large_deal_win";
const THRESHOLD: &str = "amount > 100000";
const STAGE_WON: &str = "won";

pub(crate) struct ApprovalsSeed;

#[async_trait]
impl DatasetSeed for ApprovalsSeed {
    fn order(&self) -> i32 {
        530
    }

    fn entities(&self) -> Vec<RecordType> {
        vec![POLICY]
    }

    async fn seed(&self, ctx: &Ctx<'_>, session: &mut DatasetSession) -> Result<(), AppError> {
        let Some(won_stage_id) = session
            .catalog()
            .stages
            .iter()
            .find(|(_, kind)| kind == STAGE_WON)
            .map(|(id, _)| id.clone())
        else {
            return Ok(());
        };
        let existing: Option<i64> =
            sqlx::query_scalar("SELECT 1 FROM approval_policies WHERE `key` = ? LIMIT 1")
                .bind(POLICY_KEY)
                .fetch_optional(ctx.pool())
                .await?;
        if existing.is_some() {
            return Ok(());
        }
        let role_id: Option<String> =
            sqlx::query_scalar("SELECT id FROM roles WHERE `key` = 'admin' LIMIT 1")
                .fetch_optional(ctx.pool())
                .await?;
        let Some(role_id) = role_id else {
            return Ok(());
        };
        let policy = crate::service::create_policy(
            ctx,
            PolicyCommand {
                key: Some(POLICY_KEY.to_owned()),
                name: Some("Ganho de oportunidades acima de 100 mil".to_owned()),
                description: Some(Some(
                    "Exemplo da demonstração: oportunidades acima de R$ 100 mil só podem ser marcadas como ganhas depois da aprovação de um administrador.".to_owned(),
                )),
                record_type: Some(RecordType::DEAL.as_str().to_owned()),
                moment: Some("stage_change".to_owned()),
                target_id: Some(Some(won_stage_id)),
                condition_kind: Some("formula".to_owned()),
                condition_formula: Some(Some(THRESHOLD.to_owned())),
                locked_fields: Some(vec!["amount".to_owned()]),
                due_hours: Some(Some(48)),
                is_active: Some(true),
                steps: Some(vec![StepInput {
                    name: Some("Diretoria comercial".to_owned()),
                    approver_kind: "role".to_owned(),
                    approver_user_id: None,
                    approver_role_id: Some(role_id),
                    rule: Some("any".to_owned()),
                }]),
                ..PolicyCommand::default()
            },
        )
        .await?;
        let now = session.now();
        session.record(RecordRef::new(POLICY, policy.id), now, now, BTreeMap::new());
        Ok(())
    }

    async fn cleanup(
        &self,
        tx: &mut MySqlTx,
        records: Option<&[RecordRef]>,
    ) -> Result<bool, AppError> {
        let Some(records) = records else {
            sqlx::query("DELETE FROM approval_requests")
                .execute(&mut **tx)
                .await?;
            sqlx::query("DELETE FROM approval_policies")
                .execute(&mut **tx)
                .await?;
            return Ok(true);
        };
        let mut requests = QueryBuilder::new("DELETE FROM approval_requests WHERE ");
        push_record_tuples(&mut requests, records);
        requests.build().execute(&mut **tx).await?;
        let ids = ids_of(records, &POLICY);
        if !ids.is_empty() {
            let mut delete = QueryBuilder::new("DELETE FROM approval_policies WHERE id IN (");
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
