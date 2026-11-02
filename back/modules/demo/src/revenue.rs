use aphel_contract_crm::CrmRecordTypes;
use chrono::Duration;
use uuid::Uuid;

use crate::store::DemoStore;
use aphel_kernel::Ctx;
use aphel_kernel::application::fields::validation_rules::{
    CreateValidationRuleCommand, ValidationRuleUseCases,
};
use aphel_kernel::application::shared::RecordType;
use aphel_kernel::errors::{AppError, CodedKind};
use aphel_kernel::infrastructure::fields::validation_rules::MySqlValidationRulePorts;
use approvals::domain::{PolicyCommand, StepInput};
use currencies::domain::{CreateExchangeRateCommand, SetCurrenciesCommand};
use deal_items::domain::{DealItemInput, SaveDealItemCommand};
use deals::team::domain::AddDealMemberCommand;
use pipelines::domain::UpdateStageCommand;
use price_lists::domain::{PriceListItemInput, ReplacePriceListItemsCommand, SavePriceListCommand};
use products::domain::{ProductInput, SaveProductCommand};
use quotes::domain::{CreateQuoteCommand, QuoteInput};

pub(crate) const EXCHANGE_RATE_RECORD_TYPE: RecordType = RecordType::from_static("exchange_rate");
pub(crate) const PRICE_LIST_RECORD_TYPE: RecordType = RecordType::from_static("price_list");
pub(crate) const QUOTE_RECORD_TYPE: RecordType = RecordType::from_static("quote");
pub(crate) const APPROVAL_POLICY_RECORD_TYPE: RecordType =
    RecordType::from_static("approval_policy");
pub(crate) const VALIDATION_RULE_RECORD_TYPE: RecordType =
    RecordType::from_static("validation_rule");

pub(crate) const DEMO_PRICE_LIST_KEY: &str = "demo_partners";
pub(crate) const DEMO_APPROVAL_POLICY_KEY: &str = "demo_large_deal_win";
pub(crate) const DEMO_VALIDATION_RULE_KEY: &str = "demo_large_deal_source";
const FOREIGN_CURRENCIES: [(&str, &str); 2] = [("USD", "5.4500"), ("EUR", "5.9000")];
const ROTTING_DAYS: [i64; 4] = [7, 10, 14, 21];
const DEALS_WITH_ITEMS: usize = 6;
const DEALS_WITH_QUOTE: usize = 3;
const DEALS_WITH_TEAM: usize = 8;
const APPROVAL_THRESHOLD: &str = "amount > 100000";
const VALIDATION_EXPRESSION: &str = "amount > 10000 and is_blank(source)";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DemoProductKind {
    License,
    Support,
    Setup,
    Training,
}

pub(crate) struct DemoProduct {
    pub(crate) kind: DemoProductKind,
    pub(crate) id: Uuid,
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

fn text(value: &str) -> Option<Option<String>> {
    Some(Some(value.to_owned()))
}

pub(crate) fn enabled(ctx: &Ctx<'_>, module: &str) -> bool {
    ctx.app().is_enabled(module)
}

pub(crate) async fn seed_currency(
    ctx: &Ctx<'_>,
) -> Result<Option<(&'static str, &'static str)>, AppError> {
    if !enabled(ctx, currencies::KEY) {
        return Ok(None);
    }
    let settings = currencies::service::settings(ctx).await?;
    let Some((code, rate)) = FOREIGN_CURRENCIES
        .into_iter()
        .find(|(code, _)| *code != settings.base)
    else {
        return Ok(None);
    };
    let mut codes: Vec<String> = settings
        .currencies
        .iter()
        .filter(|currency| !currency.is_base)
        .map(|currency| currency.code.clone())
        .collect();
    if !codes.iter().any(|existing| existing == code) {
        codes.push(code.to_owned());
        currencies::service::set_currencies(
            ctx,
            SetCurrenciesCommand {
                currencies: codes,
                ip: None,
            },
        )
        .await?;
    }
    Ok(Some((code, rate)))
}

pub(crate) async fn seed_rates(
    ctx: &Ctx<'_>,
    (code, rate): (&'static str, &'static str),
) -> Result<Vec<Uuid>, AppError> {
    let today = ctx.now().date_naive();
    let mut created = Vec::new();
    for days in [90, 30, 0] {
        let effective_date = (today - Duration::days(days))
            .format("%Y-%m-%d")
            .to_string();
        match currencies::service::create_rate(
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
            Ok(rate) => created.push(rate.id),
            Err(error) if is_conflict(&error) => {}
            Err(error) => return Err(error),
        }
    }
    Ok(created)
}

pub(crate) async fn seed_rotting(ctx: &Ctx<'_>, open_stages: &[&str]) -> Result<(), AppError> {
    if !enabled(ctx, pipelines::KEY) {
        return Ok(());
    }
    for (index, stage_id) in open_stages.iter().enumerate() {
        let current = pipelines::service::get_stage(ctx, stage_id).await?;
        if current.rotting_days.is_some() {
            continue;
        }
        pipelines::service::update_stage(
            ctx,
            stage_id,
            UpdateStageCommand {
                rotting_days: Some(Some(ROTTING_DAYS[index.min(ROTTING_DAYS.len() - 1)])),
                ..UpdateStageCommand::default()
            },
        )
        .await?;
    }
    Ok(())
}

pub(crate) async fn seed_products(ctx: &Ctx<'_>) -> Result<Vec<DemoProduct>, AppError> {
    if !enabled(ctx, products::KEY) {
        return Ok(Vec::new());
    }
    let catalog = [
        (
            DemoProductKind::License,
            "Licença Aphel Pro (usuário)",
            "Assinatura mensal por usuário",
            "149.00",
            "usuário",
            "monthly",
            Some("12"),
        ),
        (
            DemoProductKind::Support,
            "Suporte premium",
            "Atendimento prioritário com renovação anual",
            "4800.00",
            "contrato",
            "yearly",
            Some("12"),
        ),
        (
            DemoProductKind::Setup,
            "Implantação",
            "Configuração inicial e migração de dados",
            "3500.00",
            "projeto",
            "none",
            None,
        ),
        (
            DemoProductKind::Training,
            "Treinamento da equipe",
            "Turma de até 10 pessoas",
            "1200.00",
            "turma",
            "none",
            None,
        ),
    ];
    let mut products = Vec::new();
    for (kind, name, description, price, unit, recurrence, term) in catalog {
        let id = products::service::create(
            ctx,
            SaveProductCommand {
                input: ProductInput {
                    name: text(name),
                    description: text(description),
                    price: text(price),
                    unit: text(unit),
                    recurrence: text(recurrence),
                    term_months: term.map(|term| Some(term.to_owned())),
                    is_active: Some(Some(true)),
                    ..ProductInput::default()
                },
                ..SaveProductCommand::default()
            },
        )
        .await?
        .product
        .id;
        products.push(DemoProduct { kind, id });
    }
    Ok(products)
}

pub(crate) async fn seed_price_list(
    ctx: &Ctx<'_>,
    store: &DemoStore,
    products: &[DemoProduct],
) -> Result<Option<Uuid>, AppError> {
    if products.is_empty()
        || !enabled(ctx, price_lists::KEY)
        || store.key_exists("price_lists", DEMO_PRICE_LIST_KEY).await?
    {
        return Ok(None);
    }
    let list = price_lists::service::create(
        ctx,
        SavePriceListCommand {
            key: Some(DEMO_PRICE_LIST_KEY.to_owned()),
            name: Some("Parceiros (demonstração)".to_owned()),
            description: text("Preços com desconto para revendas e parceiros"),
            applies_to: Some("all".to_owned()),
            is_active: Some(true),
            is_default: Some(false),
            ..SavePriceListCommand::default()
        },
    )
    .await?;
    let prices = [
        (DemoProductKind::License, "129.00", Some("10")),
        (DemoProductKind::Support, "4200.00", None),
        (DemoProductKind::Setup, "2900.00", None),
    ];
    let items = prices
        .iter()
        .filter_map(|(kind, price, min_quantity)| {
            products
                .iter()
                .find(|product| product.kind == *kind)
                .map(|product| PriceListItemInput {
                    product_id: product.id.to_string(),
                    unit_price: (*price).to_owned(),
                    min_quantity: min_quantity.map(str::to_owned),
                })
        })
        .collect();
    price_lists::service::replace_items(
        ctx,
        &list.id.to_string(),
        ReplacePriceListItemsCommand {
            items,
            ..ReplacePriceListItemsCommand::default()
        },
    )
    .await?;
    Ok(Some(list.id))
}

pub(crate) async fn seed_deal_items(
    ctx: &Ctx<'_>,
    open_deals: &[Uuid],
    products: &[DemoProduct],
) -> Result<Vec<Uuid>, AppError> {
    if products.is_empty() || !enabled(ctx, deal_items::KEY) {
        return Ok(Vec::new());
    }
    let mut with_items = Vec::new();
    for (index, deal_id) in open_deals.iter().take(DEALS_WITH_ITEMS).enumerate() {
        let seats = 5 + (index as i64 * 3);
        let lines: [(DemoProductKind, String); 3] = [
            (DemoProductKind::License, seats.to_string()),
            (DemoProductKind::Setup, "1".to_owned()),
            (
                if index % 2 == 0 {
                    DemoProductKind::Support
                } else {
                    DemoProductKind::Training
                },
                "1".to_owned(),
            ),
        ];
        for (kind, quantity) in lines {
            let Some(product) = products.iter().find(|product| product.kind == kind) else {
                continue;
            };
            deal_items::service::add(
                ctx,
                &deal_id.to_string(),
                SaveDealItemCommand {
                    input: DealItemInput {
                        product_id: Some(Some(product.id.to_string())),
                        quantity: Some(Some(quantity)),
                        ..DealItemInput::default()
                    },
                    ..SaveDealItemCommand::default()
                },
            )
            .await?;
        }
        with_items.push(*deal_id);
    }
    Ok(with_items)
}

pub(crate) async fn seed_quotes(ctx: &Ctx<'_>, deals: &[Uuid]) -> Result<Vec<Uuid>, AppError> {
    if !enabled(ctx, quotes::KEY) {
        return Ok(Vec::new());
    }
    let valid_until = (ctx.now().date_naive() + Duration::days(30))
        .format("%Y-%m-%d")
        .to_string();
    let mut created = Vec::new();
    for deal_id in deals.iter().take(DEALS_WITH_QUOTE) {
        let quote = quotes::service::create(
            ctx,
            CreateQuoteCommand {
                deal_id: deal_id.to_string(),
                from_deal_items: true,
                input: QuoteInput {
                    valid_until: Some(Some(valid_until.clone())),
                    terms: text("Pagamento em 30 dias. Proposta válida pelo prazo indicado."),
                    ..QuoteInput::default()
                },
                ..CreateQuoteCommand::default()
            },
        )
        .await?;
        created.push(quote.quote.id);
    }
    Ok(created)
}

pub(crate) async fn seed_deal_team(
    ctx: &Ctx<'_>,
    store: &DemoStore,
    deals: &[Uuid],
) -> Result<u64, AppError> {
    let members = store.active_users().await?;
    if members.len() < 2 {
        return Ok(0);
    }
    let roles = deals::team::service::list_roles(ctx, false).await?;
    let mut added = 0;
    for (index, deal_id) in deals.iter().take(DEALS_WITH_TEAM).enumerate() {
        let member = &members[1 + index % (members.len() - 1)];
        let role = roles.get(index % roles.len().max(1));
        match deals::team::service::add_member(
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

pub(crate) async fn seed_approval_policy(
    ctx: &Ctx<'_>,
    store: &DemoStore,
    won_stage_id: &str,
) -> Result<Option<Uuid>, AppError> {
    if !enabled(ctx, approvals::KEY)
        || store
            .key_exists("approval_policies", DEMO_APPROVAL_POLICY_KEY)
            .await?
    {
        return Ok(None);
    }
    let Some(role_id) = store.admin_role_id().await? else {
        return Ok(None);
    };
    let policy = approvals::service::create_policy(
        ctx,
        PolicyCommand {
            key: Some(DEMO_APPROVAL_POLICY_KEY.to_owned()),
            name: Some("Ganho de oportunidades acima de 100 mil".to_owned()),
            description: Some(Some(
                "Exemplo da demonstração: oportunidades acima de R$ 100 mil só podem ser marcadas como ganhas depois da aprovação de um administrador.".to_owned(),
            )),
            record_type: Some(RecordType::DEAL.as_str().to_owned()),
            moment: Some("stage_change".to_owned()),
            target_id: Some(Some(won_stage_id.to_owned())),
            condition_kind: Some("formula".to_owned()),
            condition_formula: Some(Some(APPROVAL_THRESHOLD.to_owned())),
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
    Ok(Some(policy.id))
}

pub(crate) async fn seed_validation_rule(
    ctx: &Ctx<'_>,
    store: &DemoStore,
) -> Result<Option<Uuid>, AppError> {
    if store
        .key_exists("validation_rules", DEMO_VALIDATION_RULE_KEY)
        .await?
    {
        return Ok(None);
    }
    let rules = ValidationRuleUseCases::new(MySqlValidationRulePorts::new(
        ctx.pool().clone(),
        ctx.registry().clone(),
        ctx.events().clone(),
        ctx.audit().clone(),
    ));
    let rule = rules
        .create(
            ctx.scope,
            CreateValidationRuleCommand {
                record_type: RecordType::DEAL.as_str().to_owned(),
                key: Some(DEMO_VALIDATION_RULE_KEY.to_owned()),
                name: "Origem obrigatória acima de 10 mil".to_owned(),
                expression: VALIDATION_EXPRESSION.to_owned(),
                message: "Informe a origem para oportunidades acima de 10 mil".to_owned(),
                field: Some("source".to_owned()),
                applies_on: None,
                is_active: Some(true),
                ip: None,
            },
        )
        .await?;
    Ok(Some(rule.id))
}
