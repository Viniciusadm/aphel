use std::sync::OnceLock;
use std::time::Duration;

use async_trait::async_trait;
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use hmac::{Hmac, KeyInit, Mac};
use reqwest::header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE};
use reqwest::{Client, Method, StatusCode};
use serde_json::{Value, json};
use sha2::Sha256;

use aphel_kernel::errors::AppError;

use crate::provider::{
    EnvelopeRequest, NoticeKind, SentEnvelope, SignatureProvider, WebhookNotice, rejected,
    unauthorized, unavailable,
};

pub(crate) const PROVIDER_KEY: &str = "clicksign";
pub(crate) const SANDBOX_URL: &str = "https://sandbox.clicksign.com";
pub(crate) const PRODUCTION_URL: &str = "https://app.clicksign.com";
pub(crate) const SIGNATURE_HEADER: &str = "content-hmac";

const JSON_API: &str = "application/vnd.api+json";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(60);
const MAX_DETAIL_CHARS: usize = 300;

fn api_client() -> Result<&'static Client, AppError> {
    static CLIENT: OnceLock<Client> = OnceLock::new();
    if let Some(client) = CLIENT.get() {
        return Ok(client);
    }
    let client = Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .connect_timeout(CONNECT_TIMEOUT)
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|_| AppError::Internal("could not build the Clicksign HTTP client".to_owned()))?;
    Ok(CLIENT.get_or_init(|| client))
}

fn download_client() -> Result<&'static Client, AppError> {
    static CLIENT: OnceLock<Client> = OnceLock::new();
    if let Some(client) = CLIENT.get() {
        return Ok(client);
    }
    let client = Client::builder()
        .timeout(DOWNLOAD_TIMEOUT)
        .connect_timeout(CONNECT_TIMEOUT)
        .redirect(reqwest::redirect::Policy::limited(3))
        .build()
        .map_err(|_| AppError::Internal("could not build the download HTTP client".to_owned()))?;
    Ok(CLIENT.get_or_init(|| client))
}

pub(crate) struct Clicksign {
    base_url: &'static str,
    access_token: String,
}

impl Clicksign {
    pub(crate) fn new(production: bool, access_token: String) -> Self {
        Self {
            base_url: if production {
                PRODUCTION_URL
            } else {
                SANDBOX_URL
            },
            access_token,
        }
    }

    async fn call(
        &self,
        method: Method,
        path: &str,
        body: Option<Value>,
    ) -> Result<Value, AppError> {
        let url = format!("{}/api/v3{path}", self.base_url);
        let mut request = api_client()?
            .request(method, url)
            .header(AUTHORIZATION, self.access_token.as_str())
            .header(ACCEPT, JSON_API);
        if let Some(body) = body {
            request = request.header(CONTENT_TYPE, JSON_API).body(
                serde_json::to_vec(&body).map_err(|error| AppError::Internal(error.to_string()))?,
            );
        }
        let response = request.send().await.map_err(|error| {
            if error.is_timeout() {
                unavailable("o provedor de assinatura não respondeu a tempo")
            } else {
                unavailable("não foi possível falar com o provedor de assinatura")
            }
        })?;
        let status = response.status();
        let bytes = response.bytes().await.unwrap_or_default();
        let payload: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
        if status.is_success() {
            return Ok(payload);
        }
        Err(match status {
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => unauthorized(),
            StatusCode::BAD_REQUEST
            | StatusCode::NOT_FOUND
            | StatusCode::CONFLICT
            | StatusCode::UNPROCESSABLE_ENTITY => rejected(error_detail(&payload, status)),
            StatusCode::TOO_MANY_REQUESTS => {
                unavailable("o provedor de assinatura limitou as requisições; tente novamente")
            }
            _ => unavailable(format!(
                "o provedor de assinatura respondeu com erro HTTP {}",
                status.as_u16()
            )),
        })
    }

    async fn create(&self, path: &str, kind: &str, attributes: Value) -> Result<String, AppError> {
        let payload = self
            .call(
                Method::POST,
                path,
                Some(json!({ "data": { "type": kind, "attributes": attributes } })),
            )
            .await?;
        resource_id(&payload)
    }

    async fn requirement(
        &self,
        envelope_id: &str,
        document_id: &str,
        signer_id: &str,
        attributes: Value,
    ) -> Result<(), AppError> {
        self.call(
            Method::POST,
            &format!("/envelopes/{envelope_id}/requirements"),
            Some(json!({
                "data": {
                    "type": "requirements",
                    "attributes": attributes,
                    "relationships": {
                        "document": { "data": { "type": "documents", "id": document_id } },
                        "signer": { "data": { "type": "signers", "id": signer_id } }
                    }
                }
            })),
        )
        .await
        .map(|_| ())
    }

    async fn compose(
        &self,
        envelope_id: &str,
        request: &EnvelopeRequest,
    ) -> Result<SentEnvelope, AppError> {
        let content = format!(
            "data:application/pdf;base64,{}",
            STANDARD.encode(&request.content)
        );
        let document_id = self
            .create(
                &format!("/envelopes/{envelope_id}/documents"),
                "documents",
                json!({ "filename": request.file_name, "content_base64": content }),
            )
            .await?;
        let mut signer_ids = Vec::with_capacity(request.signers.len());
        for signer in &request.signers {
            let signer_id = self
                .create(
                    &format!("/envelopes/{envelope_id}/signers"),
                    "signers",
                    json!({
                        "name": signer.name,
                        "email": signer.email,
                        "has_documentation": false,
                        "refusable": true,
                        "communicate_events": {
                            "signature_request": "email",
                            "signature_reminder": "email",
                            "document_signed": "email"
                        }
                    }),
                )
                .await?;
            self.requirement(
                envelope_id,
                &document_id,
                &signer_id,
                json!({ "action": "agree", "role": "sign" }),
            )
            .await?;
            self.requirement(
                envelope_id,
                &document_id,
                &signer_id,
                json!({ "action": "provide_evidence", "auth": "email" }),
            )
            .await?;
            signer_ids.push(signer_id);
        }
        self.call(
            Method::PATCH,
            &format!("/envelopes/{envelope_id}"),
            Some(json!({
                "data": { "id": envelope_id, "type": "envelopes", "attributes": { "status": "running" } }
            })),
        )
        .await?;
        let mut notification = json!({});
        if let Some(message) = &request.message {
            notification = json!({ "message": message });
        }
        self.create(
            &format!("/envelopes/{envelope_id}/notifications"),
            "notifications",
            notification,
        )
        .await
        .or_else(|error| {
            if matches!(
                &error,
                AppError::Coded { code, .. }
                    if *code == crate::provider::PROVIDER_REJECTED
            ) {
                Ok(String::new())
            } else {
                Err(error)
            }
        })?;
        Ok(SentEnvelope {
            envelope_id: envelope_id.to_owned(),
            document_id,
            signer_ids,
        })
    }
}

#[async_trait]
impl SignatureProvider for Clicksign {
    fn key(&self) -> &'static str {
        PROVIDER_KEY
    }

    async fn send(&self, request: &EnvelopeRequest) -> Result<SentEnvelope, AppError> {
        let envelope_id = self
            .create(
                "/envelopes",
                "envelopes",
                json!({
                    "name": request.name,
                    "locale": "pt-BR",
                    "auto_close": true,
                    "remind_interval": 3,
                    "block_after_refusal": true
                }),
            )
            .await?;
        match self.compose(&envelope_id, request).await {
            Ok(sent) => Ok(sent),
            Err(error) => {
                let _ = self
                    .call(Method::DELETE, &format!("/envelopes/{envelope_id}"), None)
                    .await;
                Err(error)
            }
        }
    }

    async fn cancel(&self, envelope_id: &str) -> Result<(), AppError> {
        self.call(
            Method::PATCH,
            &format!("/envelopes/{envelope_id}"),
            Some(json!({
                "data": { "id": envelope_id, "type": "envelopes", "attributes": { "status": "canceled" } }
            })),
        )
        .await
        .map(|_| ())
    }

    async fn signed_document(
        &self,
        envelope_id: &str,
        document_id: &str,
    ) -> Result<Option<Vec<u8>>, AppError> {
        let payload = self
            .call(
                Method::GET,
                &format!("/envelopes/{envelope_id}/documents/{document_id}"),
                None,
            )
            .await?;
        let Some(url) = payload
            .pointer("/data/links/files/signed")
            .and_then(Value::as_str)
            .filter(|url| url.starts_with("https://"))
        else {
            return Ok(None);
        };
        let response = download_client()?
            .get(url)
            .send()
            .await
            .map_err(|_| unavailable("não foi possível baixar o documento assinado"))?;
        if !response.status().is_success() {
            return Ok(None);
        }
        let bytes = response
            .bytes()
            .await
            .map_err(|_| unavailable("não foi possível baixar o documento assinado"))?;
        Ok(Some(bytes.to_vec()))
    }
}

fn resource_id(payload: &Value) -> Result<String, AppError> {
    payload
        .pointer("/data/id")
        .and_then(|id| match id {
            Value::String(text) => Some(text.clone()),
            Value::Number(number) => Some(number.to_string()),
            _ => None,
        })
        .filter(|id| !id.is_empty() && id.len() <= 128)
        .ok_or_else(|| unavailable("resposta inesperada do provedor de assinatura"))
}

fn error_detail(payload: &Value, status: StatusCode) -> String {
    let detail = payload
        .get("errors")
        .and_then(Value::as_array)
        .map(|errors| {
            errors
                .iter()
                .filter_map(|error| {
                    error
                        .get("detail")
                        .or_else(|| error.get("title"))
                        .and_then(Value::as_str)
                })
                .collect::<Vec<_>>()
                .join("; ")
        })
        .filter(|detail| !detail.is_empty())
        .unwrap_or_else(|| format!("HTTP {}", status.as_u16()));
    detail.chars().take(MAX_DETAIL_CHARS).collect()
}

pub(crate) fn verify_signature(secret: &str, header: Option<&str>, body: &[u8]) -> bool {
    let Some(header) = header.map(str::trim) else {
        return false;
    };
    let hex = header
        .strip_prefix("sha256=")
        .or_else(|| header.strip_prefix("SHA256="))
        .unwrap_or(header);
    let Some(expected) = decode_hex(hex) else {
        return false;
    };
    let Ok(mut mac) = Hmac::<Sha256>::new_from_slice(secret.as_bytes()) else {
        return false;
    };
    mac.update(body);
    mac.verify_slice(&expected).is_ok()
}

fn decode_hex(value: &str) -> Option<Vec<u8>> {
    if value.len() != 64 {
        return None;
    }
    (0..value.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(value.get(index..index + 2)?, 16).ok())
        .collect()
}

fn text(value: Option<&Value>) -> Option<String> {
    match value? {
        Value::String(text) if !text.trim().is_empty() => Some(text.trim().to_owned()),
        Value::Number(number) => Some(number.to_string()),
        _ => None,
    }
}

pub(crate) fn parse_webhook(body: &Value) -> Option<WebhookNotice> {
    let event_name = text(body.pointer("/event/name"))?;
    let kind = match event_name.as_str() {
        "sign" => Some(NoticeKind::SignerSigned),
        "refusal" => Some(NoticeKind::SignerRefused),
        "close" | "auto_close" | "document_closed" => Some(NoticeKind::Completed),
        "cancel" => Some(NoticeKind::Cancelled),
        "deadline" => Some(NoticeKind::Expired),
        _ => None,
    };
    let references: Vec<String> = [
        "/envelope/id",
        "/envelope/key",
        "/event/data/envelope/id",
        "/event/data/envelope/key",
        "/document/envelope/id",
        "/document/envelope/key",
        "/document/envelope_key",
        "/document/key",
        "/document/id",
        "/event/data/document/key",
        "/event/data/document/id",
    ]
    .iter()
    .filter_map(|pointer| text(body.pointer(pointer)))
    .filter(|reference| reference.len() <= 128)
    .collect();
    let signer_email = [
        "/event/data/signer/email",
        "/event/data/user/email",
        "/event/data/signers/0/email",
    ]
    .iter()
    .find_map(|pointer| text(body.pointer(pointer)))
    .map(|email| email.to_lowercase());
    Some(WebhookNotice {
        event_name: event_name.chars().take(64).collect(),
        kind,
        references,
        signer_email,
    })
}
