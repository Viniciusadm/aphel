use std::collections::BTreeMap;

use aphel_contract_crm::CrmRecordTypes;
use aphel_sdk::maintenance::{DatasetEntry, DatasetSeed, DatasetSession};
use aphel_sdk::platform::application::fields::{
    CreateFieldCommand, FieldUseCases, LocalizedLabels,
};
use aphel_sdk::platform::application::shared::RecordRef;
use aphel_sdk::platform::infrastructure::fields::MySqlFieldPorts;
use aphel_sdk::platform::infrastructure::repositories::mapper;
use aphel_sdk::shared::RecordType;
use aphel_sdk::{AppError, Ctx};
use async_trait::async_trait;
use chrono::Duration;
use serde_json::{Map, Value, json};

use crate::domain::{DealInput, LoseDealCommand, MoveDealCommand};
use crate::team::domain::AddDealMemberCommand;
use aphel_kernel::errors::CodedKind;

const ORGANIZATION_ENTITY: &str = aphel_contract_crm::keys::COMPANY;
const CONTACT_ENTITY: &str = aphel_contract_crm::keys::CONTACT;
const FOREIGN_CURRENCY_VALUE: &str = "money.foreign_currency";
const FOREIGN_CURRENCY_EVERY: usize = 5;
const DEAL_COUNT: usize = 40;
const DEALS_WITH_TEAM: usize = 8;
const STAGE_OPEN: &str = "open";
const PRODUCTS: [&str; 8] = [
    "Plano anual",
    "Implantação",
    "Consultoria",
    "Licenças adicionais",
    "Treinamento da equipe",
    "Renovação de contrato",
    "Projeto piloto",
    "Suporte premium",
];
const CONTRACT_MONTHS: [i64; 4] = [6, 12, 24, 36];

enum Outcome {
    Open {
        stage_index: usize,
    },
    Won {
        reached_stage: usize,
    },
    Lost {
        reached_stage: usize,
        reason_index: usize,
    },
}

pub(crate) struct DealsSeed;

#[async_trait]
impl DatasetSeed for DealsSeed {
    fn order(&self) -> i32 {
        300
    }

    fn entities(&self) -> Vec<RecordType> {
        vec![RecordType::DEAL]
    }

    async fn seed(&self, ctx: &Ctx<'_>, session: &mut DatasetSession) -> Result<(), AppError> {
        let open_stages: Vec<String> = session
            .catalog()
            .stages
            .iter()
            .filter(|(_, kind)| kind == STAGE_OPEN)
            .map(|(id, _)| id.clone())
            .collect();
        let lost_reasons = session.catalog().lost_reason_ids.clone();
        let companies = session.entries(ORGANIZATION_ENTITY).to_vec();
        let contacts = session.entries(CONTACT_ENTITY).to_vec();
        let mut contacts_by_company = vec![Vec::new(); companies.len()];
        for contact in &contacts {
            let index = contact
                .attributes
                .get("company_index")
                .and_then(|value| value.parse::<usize>().ok());
            if let Some(bucket) = index.and_then(|index| contacts_by_company.get_mut(index)) {
                bucket.push(contact.record.record_id);
            }
        }
        if open_stages.is_empty()
            || companies.is_empty()
            || contacts_by_company.iter().any(Vec::is_empty)
        {
            return Ok(());
        }
        let field_id = FieldUseCases::new(MySqlFieldPorts::new(
            ctx.pool().clone(),
            ctx.registry().clone(),
            ctx.events().clone(),
            ctx.audit().clone(),
        ))
        .create_field(
            ctx.scope,
            RecordType::DEAL.as_str(),
            contract_months_field(),
        )
        .await?
        .definition
        .id;
        session.set_value("deals.record_type", RecordType::DEAL.as_str().to_owned());
        let now = session.now();
        session.record(
            RecordRef::new(RecordType::from_static("field"), field_id),
            now,
            now,
            BTreeMap::new(),
        );
        let foreign_currency = session.value(FOREIGN_CURRENCY_VALUE).map(str::to_owned);
        let mut random = session.random("deals");
        let today = now.date_naive();
        let stage_count = open_stages.len();
        let reason_count = lost_reasons.len().max(1);
        for index in 0..DEAL_COUNT {
            let company_index = index % companies.len();
            let company = &companies[company_index];
            let bucket = &contacts_by_company[company_index];
            let contact_id = bucket[index % bucket.len()];
            let outcome = match index % 10 {
                0 | 1 => Outcome::Won {
                    reached_stage: stage_count - 1,
                },
                2 => Outcome::Won {
                    reached_stage: random.below(stage_count),
                },
                3 | 4 => Outcome::Lost {
                    reached_stage: random.below(stage_count),
                    reason_index: random.below(reason_count),
                },
                slot => Outcome::Open {
                    stage_index: (slot - 5 + index / 10) % stage_count,
                },
            };
            let created_at = now
                - Duration::days(random.between(10, 55))
                - Duration::minutes(random.between(0, 600));
            let last_change_at = (created_at
                + Duration::hours(
                    random.between(24, ((now - created_at).num_hours() - 1).max(25)),
                ))
            .min(now);
            let expected_close_in_days = match outcome {
                Outcome::Open { .. } => random.between(-5, 60),
                _ => -(now - last_change_at).num_days(),
            };
            let title = format!(
                "{} - {}",
                random.pick(&PRODUCTS),
                company
                    .attributes
                    .get("name")
                    .map(String::as_str)
                    .unwrap_or_default()
            );
            let amount = random.between(5, 250) * 1000 + random.between(0, 9) * 100;
            let contract_months = *random.pick(&CONTRACT_MONTHS);
            let currency = foreign_currency
                .clone()
                .filter(|_| index % FOREIGN_CURRENCY_EVERY == FOREIGN_CURRENCY_EVERY - 1);
            let id = crate::service::create(
                ctx,
                DealInput {
                    title: Some(title),
                    amount: Some(Some(format!("{amount}.00"))),
                    stage_id: Some(open_stages[0].clone()),
                    expected_close_date: Some(Some(
                        (today + Duration::days(expected_close_in_days))
                            .format("%Y-%m-%d")
                            .to_string(),
                    )),
                    contact_id: Some(Some(contact_id.to_string())),
                    company_id: Some(Some(company.record.record_id.to_string())),
                    source: Some(Some("Demonstração".to_owned())),
                    currency,
                    custom_fields: custom_fields(&[("contract_months", json!(contract_months))]),
                    ..DealInput::default()
                },
            )
            .await?
            .record
            .deal
            .id;
            let deal_id = id.to_string();
            let reached = match outcome {
                Outcome::Open { stage_index } => stage_index,
                Outcome::Won { reached_stage } | Outcome::Lost { reached_stage, .. } => {
                    reached_stage
                }
            };
            for stage_id in open_stages.iter().take(reached + 1).skip(1) {
                crate::service::move_to_stage(
                    ctx,
                    &deal_id,
                    MoveDealCommand {
                        stage_id: stage_id.clone(),
                        ..MoveDealCommand::default()
                    },
                )
                .await?;
            }
            let open = matches!(outcome, Outcome::Open { .. });
            match outcome {
                Outcome::Won { .. } => {
                    crate::service::win(ctx, &deal_id, None).await?;
                }
                Outcome::Lost { reason_index, .. } => {
                    crate::service::lose(
                        ctx,
                        &deal_id,
                        LoseDealCommand {
                            lost_reason_id: lost_reasons.get(reason_index).cloned(),
                            lost_reason_note: Some(
                                "Registrado pelos dados de demonstração".to_owned(),
                            ),
                            ip: None,
                        },
                    )
                    .await?;
                }
                Outcome::Open { .. } => {}
            }
            session.record(
                RecordRef::new(RecordType::DEAL, id),
                created_at,
                last_change_at,
                BTreeMap::from([("open".to_owned(), open.to_string())]),
            );
        }
        let deal_ids = session.ids("deals.deal");
        let added = seed_team(ctx, &deal_ids).await?;
        session.add_count("deal_members", added);
        Ok(())
    }

    async fn backdate(&self, ctx: &Ctx<'_>, entry: &DatasetEntry) -> Result<bool, AppError> {
        if entry.record.record_type != RecordType::DEAL {
            return Ok(false);
        }
        let changed_at = entry.changed_at.max(entry.created_at);
        sqlx::query(
            "UPDATE deals SET created_at = ?, stage_entered_at = ?, closed_at = IF(closed_at IS NULL, NULL, ?) WHERE id = ?",
        )
        .bind(mapper::naive(entry.created_at))
        .bind(mapper::naive(changed_at))
        .bind(mapper::naive(changed_at))
        .bind(entry.record.record_id.to_string())
        .execute(ctx.pool())
        .await?;
        Ok(true)
    }
}

fn contract_months_field() -> CreateFieldCommand {
    let label: LocalizedLabels = crate::labels("Contract duration (months)");
    CreateFieldCommand {
        key: "contract_months".to_owned(),
        label: label.default_text().to_owned(),
        field_type: "number".to_owned(),
        is_required: false,
        is_unique: false,
        is_searchable: false,
        is_personal: false,
        is_sensitive: false,
        config: Value::Null,
        options: Vec::new(),
        ip: None,
    }
}

fn custom_fields(values: &[(&str, Value)]) -> Map<String, Value> {
    values
        .iter()
        .map(|(key, value)| ((*key).to_owned(), value.clone()))
        .collect()
}

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

async fn seed_team(ctx: &Ctx<'_>, deals: &[uuid::Uuid]) -> Result<u64, AppError> {
    let members: Vec<String> = sqlx::query_scalar(
        "SELECT u.id FROM users u JOIN roles r ON r.id = u.role_id WHERE u.status = 'active' \
         ORDER BY r.`key` = 'admin' DESC, u.joined_at, u.id",
    )
    .fetch_all(ctx.pool())
    .await?;
    if members.len() < 2 {
        return Ok(0);
    }
    let roles = crate::team::service::list_roles(ctx, false).await?;
    let mut added = 0;
    for (index, deal_id) in deals.iter().take(DEALS_WITH_TEAM).enumerate() {
        let member = &members[1 + index % (members.len() - 1)];
        let role = roles.get(index % roles.len().max(1));
        match crate::team::service::add_member(
            ctx,
            &deal_id.to_string(),
            AddDealMemberCommand {
                user_id: member.clone(),
                role_id: role.map(|role| role.id.to_string()),
                credit_percent: Some("30".to_owned()),
                ip: None,
            },
        )
        .await
        {
            Ok(_) => added += 1,
            Err(error) if is_conflict(&error) => {}
            Err(error) => return Err(error),
        }
    }
    Ok(added)
}
