use std::collections::BTreeMap;

use aphel_contract_crm::CrmRecordTypes;
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

use crate::domain::{ContactInput, SaveContactCommand};

const ORGANIZATION_ENTITY: &str = aphel_contract_crm::keys::COMPANY;
const CONTACTS_PER_COMPANY: usize = 3;
const DEFAULT_DOMAIN: &str = "example.com";
const FIRST_NAMES: [&str; 20] = [
    "Ana", "Bruno", "Camila", "Diego", "Elisa", "Felipe", "Gabriela", "Henrique", "Isabela",
    "João", "Larissa", "Marcos", "Natália", "Otávio", "Paula", "Rafael", "Sofia", "Thiago",
    "Vanessa", "Yuri",
];
const LAST_NAMES: [&str; 16] = [
    "Almeida", "Barbosa", "Cardoso", "Duarte", "Esteves", "Ferreira", "Gomes", "Lima", "Martins",
    "Nogueira", "Oliveira", "Pereira", "Ribeiro", "Souza", "Teixeira", "Vieira",
];
const JOB_TITLES: [&str; 8] = [
    "Diretora comercial",
    "Gerente de compras",
    "Coordenador financeiro",
    "Analista de operações",
    "CEO",
    "Gerente de TI",
    "Head de marketing",
    "Supervisor de vendas",
];
const SOURCES: [&str; 5] = [
    "Indicação",
    "Site",
    "Evento",
    "LinkedIn",
    "Prospecção ativa",
];
const CHANNELS: [&str; 3] = ["email", "phone", "whatsapp"];

pub(crate) struct PeopleSeed;

#[async_trait]
impl DatasetSeed for PeopleSeed {
    fn order(&self) -> i32 {
        200
    }

    fn entities(&self) -> Vec<RecordType> {
        vec![RecordType::CONTACT]
    }

    async fn seed(&self, ctx: &Ctx<'_>, session: &mut DatasetSession) -> Result<(), AppError> {
        let mut random = session.random("people");
        let fields = FieldUseCases::new(MySqlFieldPorts::new(
            ctx.pool().clone(),
            ctx.registry().clone(),
            ctx.events().clone(),
            ctx.audit().clone(),
        ));
        let channel_field = fields
            .create_field(
                ctx.scope,
                RecordType::CONTACT.as_str(),
                channel_field_command(),
            )
            .await?
            .definition
            .id;
        session.record(
            RecordRef::new(RecordType::from_static("field"), channel_field),
            session.now(),
            session.now(),
            BTreeMap::new(),
        );
        let companies: Vec<(String, String)> = session
            .entries(ORGANIZATION_ENTITY)
            .iter()
            .map(|entry| {
                (
                    entry.record.record_id.to_string(),
                    entry
                        .attributes
                        .get("domain")
                        .cloned()
                        .unwrap_or_else(|| DEFAULT_DOMAIN.to_owned()),
                )
            })
            .collect();
        let total = companies.len().max(1) * CONTACTS_PER_COMPANY;
        for position in 0..total {
            let company_index = position / CONTACTS_PER_COMPANY;
            let company = companies.get(company_index);
            let first_name = FIRST_NAMES[position % FIRST_NAMES.len()];
            let last_name = LAST_NAMES[(position * 7 + company_index) % LAST_NAMES.len()];
            let domain = company
                .map(|(_, domain)| domain.as_str())
                .unwrap_or(DEFAULT_DOMAIN);
            let email = format!("{}.{}@{domain}", slug(first_name), slug(last_name));
            let mobile = format!(
                "+55 11 9{:04}-{:04}",
                random.between(7000, 9999),
                random.between(1000, 9999)
            );
            let source = *random.pick(&SOURCES);
            let channel = *random.pick(&CHANNELS);
            let created_at = session.now()
                - Duration::days(random.between(60, 89))
                - Duration::minutes(random.between(0, 600));
            let id = crate::service::create(
                ctx,
                SaveContactCommand {
                    input: ContactInput {
                        first_name: text(first_name),
                        last_name: text(last_name),
                        email: text(&email),
                        mobile: text(&mobile),
                        job_title: text(JOB_TITLES[position % JOB_TITLES.len()]),
                        company_id: company.map(|(id, _)| Some(id.clone())),
                        source: text(source),
                        ..ContactInput::default()
                    },
                    custom_fields: custom_fields(&[("preferred_channel", json!(channel))]),
                    ip: None,
                    expected_updated_at: None,
                },
            )
            .await?
            .view
            .contact
            .id;
            session.record(
                RecordRef::new(RecordType::CONTACT, id),
                created_at,
                created_at,
                BTreeMap::from([("company_index".to_owned(), company_index.to_string())]),
            );
        }
        Ok(())
    }
}

fn channel_field_command() -> CreateFieldCommand {
    let label: LocalizedLabels = crate::labels("Preferred channel");
    CreateFieldCommand {
        key: "preferred_channel".to_owned(),
        label: label.default_text().to_owned(),
        field_type: "select".to_owned(),
        is_required: false,
        is_unique: false,
        is_searchable: false,
        is_personal: false,
        is_sensitive: false,
        config: Value::Null,
        options: [
            ("email", "E-mail"),
            ("phone", "Telefone"),
            ("whatsapp", "WhatsApp"),
        ]
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
