use std::collections::BTreeMap;

use aphel_contract_crm::CrmRecordTypes;
use aphel_sdk::maintenance::{DatasetSeed, DatasetSession, ids_of, push_record_tuples};
use aphel_sdk::platform::database::transaction::MySqlTx;
use aphel_sdk::shared::{RecordRef, RecordType};
use aphel_sdk::tasks::CreateActivityCommand;
use aphel_sdk::{AppError, Ctx};
use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use sqlx::{QueryBuilder, Row};

const DEAL_ENTITY: &str = "deals.deal";
const CONTACT_ENTITY: &str = "people.person";
const TASK_TITLES: [&str; 8] = [
    "Ligar para alinhar a proposta",
    "Enviar proposta revisada",
    "Agendar demonstração",
    "Follow-up por e-mail",
    "Visitar o escritório do cliente",
    "Confirmar dados para o contrato",
    "Apresentar estudo de caso",
    "Revisar objeções levantadas",
];
const ACTIVITY_KEYS: [&str; 5] = ["call", "email", "meeting", "task", "visit"];

struct PlannedTask {
    title: String,
    type_key: String,
    target: RecordRef,
    due_at: DateTime<Utc>,
    completed_at: Option<DateTime<Utc>>,
    priority: &'static str,
    created_at: DateTime<Utc>,
}

pub(crate) struct TasksSeed;

#[async_trait]
impl DatasetSeed for TasksSeed {
    fn order(&self) -> i32 {
        400
    }

    fn entities(&self) -> Vec<RecordType> {
        vec![RecordType::ACTIVITY]
    }

    async fn seed(&self, ctx: &Ctx<'_>, session: &mut DatasetSession) -> Result<(), AppError> {
        let types: BTreeMap<String, String> = sqlx::query(
            "SELECT `key`, id FROM task_types WHERE is_active = TRUE ORDER BY position, id",
        )
        .fetch_all(ctx.pool())
        .await?
        .into_iter()
        .map(|row| Ok((row.try_get("key")?, row.try_get("id")?)))
        .collect::<Result<_, AppError>>()?;
        let Some(fallback_type) = types.values().next().cloned() else {
            return Err(AppError::Validation(
                "the organization has no activity types".to_owned(),
            ));
        };
        let now = session.now();
        let mut random = session.random("tasks");
        let mut planned = Vec::new();
        for (index, deal) in session.entries(DEAL_ENTITY).iter().enumerate() {
            let open = deal
                .attributes
                .get("open")
                .is_some_and(|open| open == "true");
            let completed_due = deal.created_at + Duration::days(random.between(1, 5));
            planned.push(PlannedTask {
                title: random.pick(&TASK_TITLES).to_string(),
                type_key: random.pick(&ACTIVITY_KEYS).to_string(),
                target: deal.record.clone(),
                due_at: completed_due.min(now),
                completed_at: Some(completed_due.min(now)),
                priority: "normal",
                created_at: deal.created_at,
            });
            if open {
                let due_at = match index % 4 {
                    0 => now - Duration::days(random.between(1, 10)),
                    1 => now + Duration::hours(random.between(2, 8)),
                    _ => now + Duration::days(random.between(1, 14)),
                };
                planned.push(PlannedTask {
                    title: random.pick(&TASK_TITLES).to_string(),
                    type_key: random.pick(&ACTIVITY_KEYS).to_string(),
                    target: deal.record.clone(),
                    due_at,
                    completed_at: None,
                    priority: if index % 3 == 0 { "high" } else { "normal" },
                    created_at: deal.changed_at,
                });
            }
        }
        for (index, contact) in session
            .entries(CONTACT_ENTITY)
            .iter()
            .enumerate()
            .step_by(6)
        {
            let due_at = if index % 12 == 0 {
                now - Duration::days(random.between(2, 7))
            } else {
                now + Duration::days(random.between(3, 20))
            };
            planned.push(PlannedTask {
                title: "Retomar contato".to_owned(),
                type_key: "call".to_owned(),
                target: contact.record.clone(),
                due_at,
                completed_at: None,
                priority: "low",
                created_at: contact.created_at,
            });
        }
        for task in planned {
            let activity_type_id = types
                .get(&task.type_key)
                .cloned()
                .unwrap_or_else(|| fallback_type.clone());
            let id = crate::service::create(
                ctx,
                CreateActivityCommand {
                    title: task.title,
                    activity_type_id,
                    status: Some(
                        if task.completed_at.is_some() {
                            "done"
                        } else {
                            "open"
                        }
                        .to_owned(),
                    ),
                    priority: Some(task.priority.to_owned()),
                    due_at: Some(task.due_at.to_rfc3339()),
                    completed_at: task.completed_at.map(|value| value.to_rfc3339()),
                    record_type: Some(task.target.record_type.as_str().to_owned()),
                    record_id: Some(task.target.record_id.to_string()),
                    ..CreateActivityCommand::default()
                },
            )
            .await?
            .activity
            .id;
            let changed_at = task.completed_at.unwrap_or(task.created_at);
            session.record(
                RecordRef::new(RecordType::ACTIVITY, id),
                task.created_at,
                changed_at,
                BTreeMap::new(),
            );
        }
        Ok(())
    }

    async fn cleanup(
        &self,
        tx: &mut MySqlTx,
        records: Option<&[RecordRef]>,
    ) -> Result<bool, AppError> {
        let Some(records) = records else {
            sqlx::query("DELETE FROM task_links")
                .execute(&mut **tx)
                .await?;
            sqlx::query("DELETE FROM tasks").execute(&mut **tx).await?;
            return Ok(true);
        };
        let mut links = QueryBuilder::new("DELETE FROM task_links WHERE ");
        push_record_tuples(&mut links, records);
        links.build().execute(&mut **tx).await?;
        let mut detach =
            QueryBuilder::new("UPDATE tasks SET record_type = NULL, record_id = NULL WHERE ");
        push_record_tuples(&mut detach, records);
        detach.build().execute(&mut **tx).await?;
        let ids = ids_of(records, &RecordType::ACTIVITY);
        if !ids.is_empty() {
            let mut delete = QueryBuilder::new("DELETE FROM tasks WHERE id IN (");
            let mut list = delete.separated(", ");
            for id in ids {
                list.push_bind(id);
            }
            delete.push(")");
            delete.build().execute(&mut **tx).await?;
        }
        Ok(true)
    }

    async fn counts(&self, tx: &mut MySqlTx) -> Result<Option<BTreeMap<String, u64>>, AppError> {
        let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tasks")
            .fetch_one(&mut **tx)
            .await?;
        Ok(Some(if total > 0 {
            BTreeMap::from([("tasks".to_owned(), total as u64)])
        } else {
            BTreeMap::new()
        }))
    }
}
