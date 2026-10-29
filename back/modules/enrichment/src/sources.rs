use std::collections::BTreeMap;
use std::sync::OnceLock;
use std::time::Duration;

use async_trait::async_trait;
use reqwest::header::{ACCEPT, USER_AGENT};
use reqwest::{Client, StatusCode};
use serde_json::Value;

use crate::cnpj;
use aphel_kernel::errors::AppError;

pub(crate) const BRASILAPI: &str = "brasilapi";
pub(crate) const RECEITAWS: &str = "receitaws";
pub(crate) const BRASILAPI_THEN_RECEITAWS: &str = "brasilapi_then_receitaws";

pub(crate) const CNPJ_NOT_FOUND: &str = "enrichment_cnpj_not_found";
pub(crate) const SOURCE_RATE_LIMITED: &str = "enrichment_source_rate_limited";
pub(crate) const SOURCE_UNAVAILABLE: &str = "enrichment_source_unavailable";

const BRASILAPI_URL: &str = "https://brasilapi.com.br/api/cnpj/v1";
const RECEITAWS_URL: &str = "https://receitaws.com.br/v1/cnpj";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const MAX_VALUE_CHARS: usize = 500;

pub(crate) type CompanyData = BTreeMap<String, String>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Fetched {
    pub(crate) source: &'static str,
    pub(crate) data: CompanyData,
}

#[async_trait]
pub(crate) trait CompanyDataSource: Send + Sync {
    fn key(&self) -> &'static str;
    async fn fetch(&self, cnpj: &str) -> Result<CompanyData, AppError>;
}

fn client() -> Result<&'static Client, AppError> {
    static CLIENT: OnceLock<Client> = OnceLock::new();
    if let Some(client) = CLIENT.get() {
        return Ok(client);
    }
    let client = Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .connect_timeout(CONNECT_TIMEOUT)
        .redirect(reqwest::redirect::Policy::limited(2))
        .build()
        .map_err(|_| AppError::Internal("could not build the enrichment HTTP client".to_owned()))?;
    Ok(CLIENT.get_or_init(|| client))
}

fn unavailable(message: impl Into<String>) -> AppError {
    AppError::unavailable_code(SOURCE_UNAVAILABLE, message)
}

fn not_found() -> AppError {
    AppError::not_found_code(CNPJ_NOT_FOUND, "CNPJ não encontrado na fonte de dados")
}

fn rate_limited() -> AppError {
    AppError::too_many_requests_code(
        SOURCE_RATE_LIMITED,
        "a fonte de dados limitou as consultas; tente novamente em alguns minutos",
    )
}

async fn get_json(url: String) -> Result<Value, AppError> {
    let response = client()?
        .get(url)
        .header(ACCEPT, "application/json")
        .header(USER_AGENT, "Aphel-CRM-Enrichment/1.0")
        .send()
        .await
        .map_err(|error| {
            if error.is_timeout() {
                unavailable("a fonte de dados não respondeu a tempo")
            } else {
                unavailable("não foi possível consultar a fonte de dados")
            }
        })?;
    match response.status() {
        status if status.is_success() => response
            .json::<Value>()
            .await
            .map_err(|_| unavailable("resposta inválida da fonte de dados")),
        StatusCode::NOT_FOUND | StatusCode::BAD_REQUEST => Err(not_found()),
        StatusCode::TOO_MANY_REQUESTS => Err(rate_limited()),
        status => Err(unavailable(format!(
            "a fonte de dados respondeu com erro HTTP {}",
            status.as_u16()
        ))),
    }
}

fn text(value: Option<&Value>) -> Option<String> {
    let raw = match value? {
        Value::String(text) => text.trim().to_owned(),
        Value::Number(number) => number.to_string(),
        _ => return None,
    };
    (!raw.is_empty()).then(|| raw.chars().take(MAX_VALUE_CHARS).collect())
}

fn put(data: &mut CompanyData, key: &str, value: Option<String>) {
    if let Some(value) = value {
        data.insert(key.to_owned(), value);
    }
}

fn digits(value: &str) -> String {
    value.chars().filter(char::is_ascii_digit).collect()
}

fn format_postal_code(value: &str) -> Option<String> {
    let digits = digits(value);
    (digits.len() == 8).then(|| format!("{}-{}", &digits[..5], &digits[5..]))
}

fn format_phone(value: &str) -> Option<String> {
    let digits = digits(value);
    match digits.len() {
        10 => Some(format!(
            "({}) {}-{}",
            &digits[..2],
            &digits[2..6],
            &digits[6..]
        )),
        11 => Some(format!(
            "({}) {}-{}",
            &digits[..2],
            &digits[2..7],
            &digits[7..]
        )),
        _ => None,
    }
}

fn join_address(data: &CompanyData) -> Option<String> {
    let street = [data.get("address_street"), data.get("address_number")]
        .into_iter()
        .flatten()
        .cloned()
        .collect::<Vec<_>>()
        .join(", ");
    let city = [data.get("address_city"), data.get("address_state")]
        .into_iter()
        .flatten()
        .cloned()
        .collect::<Vec<_>>()
        .join("/");
    let parts: Vec<String> = [
        (!street.is_empty()).then_some(street),
        data.get("address_complement").cloned(),
        data.get("address_district").cloned(),
        (!city.is_empty()).then_some(city),
        data.get("address_postal_code").cloned(),
    ]
    .into_iter()
    .flatten()
    .collect();
    (!parts.is_empty()).then(|| parts.join(" - "))
}

pub(crate) struct BrasilApi;

#[async_trait]
impl CompanyDataSource for BrasilApi {
    fn key(&self) -> &'static str {
        BRASILAPI
    }

    async fn fetch(&self, cnpj: &str) -> Result<CompanyData, AppError> {
        let body = get_json(format!("{BRASILAPI_URL}/{cnpj}")).await?;
        let mut data = CompanyData::new();
        put(&mut data, "legal_name", text(body.get("razao_social")));
        put(&mut data, "trade_name", text(body.get("nome_fantasia")));
        let activity = match (
            text(body.get("cnae_fiscal")),
            text(body.get("cnae_fiscal_descricao")),
        ) {
            (Some(code), Some(description)) => Some(format!("{code} - {description}")),
            (code, description) => description.or(code),
        };
        put(&mut data, "activity", activity);
        put(
            &mut data,
            "size",
            text(body.get("porte")).or_else(|| text(body.get("descricao_porte"))),
        );
        put(
            &mut data,
            "registration_status",
            text(body.get("descricao_situacao_cadastral")),
        );
        put(
            &mut data,
            "opened_on",
            text(body.get("data_inicio_atividade")),
        );
        let street = match (
            text(body.get("descricao_tipo_de_logradouro")),
            text(body.get("logradouro")),
        ) {
            (Some(kind), Some(street))
                if !street.to_uppercase().starts_with(&kind.to_uppercase()) =>
            {
                Some(format!("{kind} {street}"))
            }
            (_, street) => street,
        };
        put(&mut data, "address_street", street);
        put(&mut data, "address_number", text(body.get("numero")));
        put(
            &mut data,
            "address_complement",
            text(body.get("complemento")),
        );
        put(&mut data, "address_district", text(body.get("bairro")));
        put(&mut data, "address_city", text(body.get("municipio")));
        put(&mut data, "address_state", text(body.get("uf")));
        put(
            &mut data,
            "address_postal_code",
            text(body.get("cep")).and_then(|cep| format_postal_code(&cep)),
        );
        put(
            &mut data,
            "phone",
            text(body.get("ddd_telefone_1")).and_then(|phone| format_phone(&phone)),
        );
        put(
            &mut data,
            "email",
            text(body.get("email")).map(|email| email.to_lowercase()),
        );
        Ok(finish(cnpj, data))
    }
}

pub(crate) struct ReceitaWs;

fn brazilian_date(value: &str) -> Option<String> {
    let mut parts = value.split('/');
    let (day, month, year) = (parts.next()?, parts.next()?, parts.next()?);
    (day.len() == 2 && month.len() == 2 && year.len() == 4).then(|| format!("{year}-{month}-{day}"))
}

#[async_trait]
impl CompanyDataSource for ReceitaWs {
    fn key(&self) -> &'static str {
        RECEITAWS
    }

    async fn fetch(&self, cnpj: &str) -> Result<CompanyData, AppError> {
        let body = get_json(format!("{RECEITAWS_URL}/{cnpj}")).await?;
        if body.get("status").and_then(Value::as_str) == Some("ERROR") {
            return Err(not_found());
        }
        let mut data = CompanyData::new();
        put(&mut data, "legal_name", text(body.get("nome")));
        put(&mut data, "trade_name", text(body.get("fantasia")));
        let activity = body
            .get("atividade_principal")
            .and_then(Value::as_array)
            .and_then(|items| items.first())
            .and_then(
                |item| match (text(item.get("code")), text(item.get("text"))) {
                    (Some(code), Some(description)) => Some(format!("{code} - {description}")),
                    (code, description) => description.or(code),
                },
            );
        put(&mut data, "activity", activity);
        put(&mut data, "size", text(body.get("porte")));
        put(&mut data, "registration_status", text(body.get("situacao")));
        put(
            &mut data,
            "opened_on",
            text(body.get("abertura")).and_then(|date| brazilian_date(&date)),
        );
        put(&mut data, "address_street", text(body.get("logradouro")));
        put(&mut data, "address_number", text(body.get("numero")));
        put(
            &mut data,
            "address_complement",
            text(body.get("complemento")),
        );
        put(&mut data, "address_district", text(body.get("bairro")));
        put(&mut data, "address_city", text(body.get("municipio")));
        put(&mut data, "address_state", text(body.get("uf")));
        put(
            &mut data,
            "address_postal_code",
            text(body.get("cep")).and_then(|cep| format_postal_code(&cep)),
        );
        put(
            &mut data,
            "phone",
            text(body.get("telefone"))
                .and_then(|phone| phone.split('/').next().map(str::trim).map(str::to_owned))
                .map(|phone| format_phone(&phone).unwrap_or(phone)),
        );
        put(
            &mut data,
            "email",
            text(body.get("email")).map(|email| email.to_lowercase()),
        );
        Ok(finish(cnpj, data))
    }
}

fn finish(cnpj_value: &str, mut data: CompanyData) -> CompanyData {
    if let Some(address) = join_address(&data) {
        data.insert("address".to_owned(), address);
    }
    data.insert("cnpj".to_owned(), cnpj::format(cnpj_value));
    data
}

pub(crate) fn chain(mode: &str) -> Vec<Box<dyn CompanyDataSource>> {
    match mode {
        BRASILAPI => vec![Box::new(BrasilApi)],
        RECEITAWS => vec![Box::new(ReceitaWs)],
        _ => vec![Box::new(BrasilApi), Box::new(ReceitaWs)],
    }
}
