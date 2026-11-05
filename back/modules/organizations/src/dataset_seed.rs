use std::collections::BTreeMap;

use aphel_contract_crm::CrmRecordTypes;
use aphel_kernel::records::AddressInput;
use aphel_sdk::maintenance::{DatasetSeed, DatasetSession};
use aphel_sdk::platform::application::fields::{
    CreateFieldCommand, FieldOptionInput, FieldUseCases, LocalizedLabels,
};
use aphel_sdk::platform::application::shared::RecordRef;
use aphel_sdk::platform::infrastructure::fields::MySqlFieldPorts;
use aphel_sdk::shared::RecordType;
use aphel_sdk::{AppError, Ctx};
use async_trait::async_trait;
use chrono::Duration;
use serde_json::{Map, Value, json};

use crate::domain::{CompanyInput, SaveCompanyCommand};

pub(crate) const SEED_ORDER: i32 = 100;
const COMPANY_NAMES: [&str; 20] = [
    "Aurora Logística",
    "Bússola Engenharia",
    "Cerrado Alimentos",
    "Delta Saúde Integrada",
    "Estrela Educação",
    "Farol Contabilidade",
    "Galáxia Software",
    "Horizonte Imóveis",
    "Ipê Construtora",
    "Jangada Turismo",
    "Lume Energia Solar",
    "Maré Pescados",
    "Nuvem Telecom",
    "Orquídea Cosméticos",
    "Pampa Agronegócio",
    "Quasar Consultoria",
    "Rio Claro Transportes",
    "Serra Verde Móveis",
    "Trilha Esportes",
    "Vértice Marketing",
];
const INDUSTRIES: [&str; 10] = [
    "Logística",
    "Engenharia",
    "Alimentos",
    "Saúde",
    "Educação",
    "Serviços financeiros",
    "Tecnologia",
    "Imobiliário",
    "Construção",
    "Turismo",
];
const CITIES: [(&str, &str); 8] = [
    ("São Paulo", "SP"),
    ("Rio de Janeiro", "RJ"),
    ("Belo Horizonte", "MG"),
    ("Curitiba", "PR"),
    ("Porto Alegre", "RS"),
    ("Recife", "PE"),
    ("Salvador", "BA"),
    ("Goiânia", "GO"),
];
const SIZE_RANGES: [&str; 4] = ["size_1_10", "size_11_50", "size_51_200", "size_201_500"];
const SOURCES: [&str; 5] = [
    "Indicação",
    "Site",
    "Evento",
    "LinkedIn",
    "Prospecção ativa",
];
const TIERS: [&str; 3] = ["gold", "silver", "bronze"];

pub(crate) struct OrganizationsSeed;

#[async_trait]
impl DatasetSeed for OrganizationsSeed {
    fn order(&self) -> i32 {
        SEED_ORDER
    }

    fn entities(&self) -> Vec<RecordType> {
        vec![RecordType::COMPANY]
    }

    async fn seed(&self, ctx: &Ctx<'_>, session: &mut DatasetSession) -> Result<(), AppError> {
        let mut random = session.random("organizations");
        let fields = FieldUseCases::new(MySqlFieldPorts::new(
            ctx.pool().clone(),
            ctx.registry().clone(),
            ctx.events().clone(),
            ctx.audit().clone(),
        ));
        let tier_field = fields
            .create_field(
                ctx.scope,
                RecordType::COMPANY.as_str(),
                tier_field_command(),
            )
            .await?
            .definition
            .id;
        session.record(
            RecordRef::new(RecordType::from_static("field"), tier_field),
            session.now(),
            session.now(),
            BTreeMap::new(),
        );
        for (index, name) in COMPANY_NAMES.iter().enumerate() {
            let (city, state) = CITIES[index % CITIES.len()];
            let domain = format!("{}.example.com", slug(name));
            let industry = INDUSTRIES[index % INDUSTRIES.len()];
            let size_range = *random.pick(&SIZE_RANGES);
            let phone = format!(
                "+55 11 3{:03}-{:04}",
                random.between(100, 999),
                random.between(1000, 9999)
            );
            let source = *random.pick(&SOURCES);
            let tier = *random.pick(&TIERS);
            let created_at = session.now()
                - Duration::days(random.between(90, 180))
                - Duration::minutes(random.between(0, 600));
            let id = crate::service::create(
                ctx,
                SaveCompanyCommand {
                    input: CompanyInput {
                        name: text(name),
                        domain: text(&domain),
                        website: text(&format!("https://{domain}")),
                        industry: text(industry),
                        size_range: text(size_range),
                        phone: text(&phone),
                        email: text(&format!("contato@{domain}")),
                        address: AddressInput {
                            city: text(city),
                            state: text(state),
                            country: text("BR"),
                            ..AddressInput::default()
                        },
                        source: text(source),
                        ..CompanyInput::default()
                    },
                    custom_fields: custom_fields(&[("customer_tier", json!(tier))]),
                    ip: None,
                    expected_updated_at: None,
                },
            )
            .await?
            .view
            .company
            .id;
            session.record(
                RecordRef::new(RecordType::COMPANY, id),
                created_at,
                created_at,
                BTreeMap::from([
                    ("name".to_owned(), (*name).to_owned()),
                    ("domain".to_owned(), domain),
                ]),
            );
        }
        Ok(())
    }
}

fn tier_field_command() -> CreateFieldCommand {
    let label: LocalizedLabels = crate::labels("Customer category");
    CreateFieldCommand {
        key: "customer_tier".to_owned(),
        label: label.default_text().to_owned(),
        field_type: "select".to_owned(),
        is_required: false,
        is_unique: false,
        is_searchable: false,
        is_personal: false,
        is_sensitive: false,
        config: Value::Null,
        options: [("gold", "Ouro"), ("silver", "Prata"), ("bronze", "Bronze")]
            .iter()
            .map(|(value, label)| FieldOptionInput {
                value: Some((*value).to_owned()),
                label: (*label).to_owned(),
                color: None,
            })
            .collect(),
        ip: None,
    }
}

fn text(value: &str) -> Option<Option<String>> {
    Some(Some(value.to_owned()))
}

fn custom_fields(values: &[(&str, Value)]) -> Map<String, Value> {
    values
        .iter()
        .map(|(key, value)| ((*key).to_owned(), value.clone()))
        .collect()
}

fn slug(text: &str) -> String {
    let mut slug = String::new();
    for character in text.chars() {
        let folded = match character {
            'á' | 'à' | 'â' | 'ã' | 'Á' | 'Â' | 'Ã' => 'a',
            'é' | 'ê' | 'É' | 'Ê' => 'e',
            'í' | 'Í' => 'i',
            'ó' | 'ô' | 'õ' | 'Ó' | 'Ô' | 'Õ' => 'o',
            'ú' | 'ü' | 'Ú' => 'u',
            'ç' | 'Ç' => 'c',
            other => other,
        };
        if folded.is_ascii_alphanumeric() {
            slug.push(folded.to_ascii_lowercase());
        } else if !slug.is_empty() && !slug.ends_with('-') {
            slug.push('-');
        }
    }
    slug.trim_matches('-').to_owned()
}
