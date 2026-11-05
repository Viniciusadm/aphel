use std::collections::BTreeMap;

use aphel_contract_crm::CrmRecordTypes;
use aphel_sdk::maintenance::{DatasetSeed, DatasetSession};
use aphel_sdk::shared::{RecordRef, RecordType};
use aphel_sdk::{AppError, Ctx};
use async_trait::async_trait;

use crate::domain::{ProductInput, SaveProductCommand};

type ProductSpec = (
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    Option<&'static str>,
);

const PRODUCTS: [ProductSpec; 4] = [
    (
        "license",
        "Licença Aphel Pro (usuário)",
        "Assinatura mensal por usuário",
        "149.00",
        "usuário",
        "monthly",
        Some("12"),
    ),
    (
        "support",
        "Suporte premium",
        "Atendimento prioritário com renovação anual",
        "4800.00",
        "contrato",
        "yearly",
        Some("12"),
    ),
    (
        "setup",
        "Implantação",
        "Configuração inicial e migração de dados",
        "3500.00",
        "projeto",
        "none",
        None,
    ),
    (
        "training",
        "Treinamento da equipe",
        "Turma de até 10 pessoas",
        "1200.00",
        "turma",
        "none",
        None,
    ),
];

pub(crate) struct CatalogItemsSeed;

fn text(value: &str) -> Option<Option<String>> {
    Some(Some(value.to_owned()))
}

#[async_trait]
impl DatasetSeed for CatalogItemsSeed {
    fn order(&self) -> i32 {
        150
    }

    fn entities(&self) -> Vec<RecordType> {
        vec![RecordType::PRODUCT]
    }

    async fn seed(&self, ctx: &Ctx<'_>, session: &mut DatasetSession) -> Result<(), AppError> {
        session.set_value(
            "catalog_items.record_type",
            RecordType::PRODUCT.as_str().to_owned(),
        );
        let now = session.now();
        for (kind, name, description, price, unit, recurrence, term) in PRODUCTS {
            let id = crate::service::create(
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
            session.record(
                RecordRef::new(RecordType::PRODUCT, id),
                now,
                now,
                BTreeMap::from([("kind".to_owned(), kind.to_owned())]),
            );
        }
        Ok(())
    }
}
