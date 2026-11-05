use std::collections::BTreeMap;

use aphel_contract_crm::CrmRecordTypes;
use aphel_sdk::maintenance::{
    DatasetEntry, DatasetSeed, DatasetSession, ids_of, push_record_tuples,
};
use aphel_sdk::notes::CreateNoteCommand;
use aphel_sdk::platform::database::transaction::MySqlTx;
use aphel_sdk::platform::infrastructure::repositories::mapper;
use aphel_sdk::shared::{RecordRef, RecordType};
use aphel_sdk::{AppError, Ctx};
use async_trait::async_trait;
use chrono::Duration;
use sqlx::QueryBuilder;

const DEAL_ENTITY: &str = aphel_contract_crm::keys::DEAL;
const ORGANIZATION_ENTITY: &str = aphel_contract_crm::keys::COMPANY;
const NOTE_BODIES: [&str; 6] = [
    "Reunião produtiva. O cliente quer **reduzir custos operacionais** e pediu uma proposta até o fim do mês.",
    "Pontos levantados:\n\n- prazo de implantação\n- integração com o ERP atual\n- treinamento da equipe",
    "Contato prefere conversar pela manhã. Decisão final passa pela diretoria financeira.",
    "Concorrente apresentou preço menor, mas sem suporte local. Reforçar nosso diferencial de atendimento.",
    "Cliente elogiou o atendimento e pode indicar outras empresas do grupo.",
    "Enviado material com estudos de caso do setor. Aguardando retorno.",
];

pub(crate) struct NotesSeed;

#[async_trait]
impl DatasetSeed for NotesSeed {
    fn order(&self) -> i32 {
        410
    }

    fn entities(&self) -> Vec<RecordType> {
        vec![RecordType::NOTE]
    }

    async fn seed(&self, ctx: &Ctx<'_>, session: &mut DatasetSession) -> Result<(), AppError> {
        let mut random = session.random("notes");
        let mut planned = Vec::new();
        for (index, deal) in session
            .entries(DEAL_ENTITY)
            .iter()
            .enumerate()
            .filter(|(index, _)| index % 2 == 0)
        {
            planned.push((
                deal.record.clone(),
                random.pick(&NOTE_BODIES).to_string(),
                index % 8 == 0,
                deal.created_at + Duration::hours(random.between(2, 48)),
            ));
        }
        for company in session.entries(ORGANIZATION_ENTITY).iter().step_by(4) {
            planned.push((
                company.record.clone(),
                random.pick(&NOTE_BODIES).to_string(),
                false,
                company.created_at + Duration::days(1),
            ));
        }
        for (target, body, pinned, created_at) in planned {
            let id = crate::service::create(
                ctx,
                CreateNoteCommand {
                    record_type: target.record_type.as_str().to_owned(),
                    record_id: target.record_id.to_string(),
                    body,
                    visibility: Some("team".to_owned()),
                    pinned: Some(pinned),
                    ip: None,
                },
            )
            .await?
            .id;
            session.record(
                RecordRef::new(RecordType::NOTE, id),
                created_at,
                created_at,
                BTreeMap::new(),
            );
        }
        Ok(())
    }

    async fn backdate(&self, ctx: &Ctx<'_>, entry: &DatasetEntry) -> Result<bool, AppError> {
        if entry.record.record_type != RecordType::NOTE {
            return Ok(false);
        }
        let changed_at = entry.changed_at.max(entry.created_at);
        let mut tx = ctx.begin().await?;
        sqlx::query("UPDATE notes SET created_at = ?, updated_at = ? WHERE id = ?")
            .bind(mapper::naive(entry.created_at))
            .bind(mapper::naive(changed_at))
            .bind(entry.record.record_id.to_string())
            .execute(&mut *tx)
            .await?;
        aphel_sdk::records::backdate_dataset_history(
            &mut tx,
            &entry.record,
            entry.created_at,
            changed_at,
        )
        .await?;
        tx.commit().await?;
        Ok(true)
    }

    async fn cleanup(
        &self,
        tx: &mut MySqlTx,
        records: Option<&[RecordRef]>,
    ) -> Result<bool, AppError> {
        let Some(records) = records else {
            sqlx::query("DELETE FROM notes").execute(&mut **tx).await?;
            return Ok(true);
        };
        let mut delete = QueryBuilder::new("DELETE FROM notes WHERE ");
        push_record_tuples(&mut delete, records);
        let ids = ids_of(records, &RecordType::NOTE);
        if !ids.is_empty() {
            delete.push(" OR id IN (");
            let mut list = delete.separated(", ");
            for id in ids {
                list.push_bind(id);
            }
            delete.push(")");
        }
        delete.build().execute(&mut **tx).await?;
        Ok(true)
    }

    async fn counts(&self, tx: &mut MySqlTx) -> Result<Option<BTreeMap<String, u64>>, AppError> {
        let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM notes")
            .fetch_one(&mut **tx)
            .await?;
        Ok(Some(if total > 0 {
            BTreeMap::from([("notes".to_owned(), total as u64)])
        } else {
            BTreeMap::new()
        }))
    }
}
