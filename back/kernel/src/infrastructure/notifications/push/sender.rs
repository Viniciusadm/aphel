use std::time::Duration;

use async_trait::async_trait;
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use chrono::Utc;
use reqwest::StatusCode;
use serde_json::json;

use crate::application::notifications::push::{
    PushMessage, PushOutcome, PushSender, PushTarget, is_known_push_service,
};
use crate::errors::AppError;
use crate::infrastructure::notifications::push::VaultVapidKeys;
use crate::infrastructure::notifications::push::crypto::encrypt_aes128gcm;

pub(crate) const PUSH_TTL_SECONDS: u32 = 24 * 60 * 60;
pub(crate) const VAPID_TOKEN_LIFETIME_SECONDS: i64 = 12 * 60 * 60;
pub(crate) const PUSH_REQUEST_TIMEOUT: Duration = Duration::from_secs(15);
pub(crate) const DEFAULT_VAPID_SUBJECT: &str = "mailto:webpush@aphel.invalid";

pub(crate) fn vapid_subject(app_base_url: &str) -> String {
    let base = app_base_url.trim().trim_end_matches('/');
    if base.starts_with("https://") {
        base.to_owned()
    } else {
        DEFAULT_VAPID_SUBJECT.to_owned()
    }
}

#[derive(Clone)]
pub(crate) struct WebPushSender {
    keys: VaultVapidKeys,
    client: reqwest::Client,
    subject: String,
}

impl WebPushSender {
    pub(crate) fn new(keys: VaultVapidKeys, app_base_url: &str) -> Result<Self, AppError> {
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(PUSH_REQUEST_TIMEOUT)
            .build()
            .map_err(|error| AppError::Internal(format!("web push client: {error}")))?;
        Ok(Self {
            keys,
            client,
            subject: vapid_subject(app_base_url),
        })
    }

    async fn authorization(&self, endpoint: &str) -> Result<String, AppError> {
        let audience = reqwest::Url::parse(endpoint)
            .map_err(|_| AppError::Internal("invalid push endpoint".to_owned()))?
            .origin()
            .ascii_serialization();
        let pair = self.keys.key_pair().await?;
        let header = URL_SAFE_NO_PAD.encode(br#"{"typ":"JWT","alg":"ES256"}"#);
        let claims = URL_SAFE_NO_PAD.encode(
            json!({
                "aud": audience,
                "exp": Utc::now().timestamp() + VAPID_TOKEN_LIFETIME_SECONDS,
                "sub": self.subject,
            })
            .to_string(),
        );
        let signing_input = format!("{header}.{claims}");
        let signature = URL_SAFE_NO_PAD.encode(pair.sign(signing_input.as_bytes())?);
        Ok(format!(
            "vapid t={signing_input}.{signature}, k={}",
            pair.public_key()
        ))
    }
}

fn decode_key(value: &str) -> Result<Vec<u8>, AppError> {
    URL_SAFE_NO_PAD
        .decode(value.trim_end_matches('='))
        .map_err(|_| AppError::Internal("invalid push subscription key".to_owned()))
}

fn payload(message: &PushMessage) -> Vec<u8> {
    json!({
        "title": message.title,
        "body": message.body,
        "url": message.url,
        "tag": message.tag,
    })
    .to_string()
    .into_bytes()
}

#[async_trait]
impl PushSender for WebPushSender {
    async fn send(
        &self,
        target: &PushTarget,
        message: &PushMessage,
    ) -> Result<PushOutcome, AppError> {
        if !is_known_push_service(&target.endpoint) {
            return Ok(PushOutcome::Gone);
        }
        let body = encrypt_aes128gcm(
            &decode_key(&target.p256dh)?,
            &decode_key(&target.auth)?,
            &payload(message),
        )?;
        let authorization = self.authorization(&target.endpoint).await?;
        let response = self
            .client
            .post(&target.endpoint)
            .header("TTL", PUSH_TTL_SECONDS.to_string())
            .header("Urgency", "normal")
            .header("Content-Encoding", "aes128gcm")
            .header("Content-Type", "application/octet-stream")
            .header("Authorization", authorization)
            .body(body)
            .send()
            .await;
        let response = match response {
            Ok(response) => response,
            Err(error) => return Ok(PushOutcome::Failed(error.without_url().to_string())),
        };
        let status = response.status();
        if status.is_success() {
            return Ok(PushOutcome::Delivered);
        }
        if status == StatusCode::NOT_FOUND || status == StatusCode::GONE {
            return Ok(PushOutcome::Gone);
        }
        Ok(PushOutcome::Failed(format!(
            "push service answered {}",
            status.as_u16()
        )))
    }
}
