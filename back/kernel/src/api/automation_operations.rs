use crate::application::automations::gateway::USER;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use uuid::Uuid;

use crate::api::routes::v1::custom_objects::relation_use_cases;
use crate::api::state::AppState;
use crate::application::access::AccessRepository;
use crate::application::automations::{
    AutomationGatewayFactory, AutomationOperations, HandlerAutomationGateway,
    HandlerAutomationGatewayFactory, Reference, SharedAutomationOperations, WebhookCall,
    WebhookCallResult,
};
use crate::application::shared::RecordRef;
use crate::application::webhooks::outgoing::{
    UrlRejection, WebhookRequest, WebhookSender, WebhookUrlGuard, sign_payload,
};
use crate::domain::UserId;
use crate::domain::access::AccessScope;
use crate::errors::AppError;
use crate::infrastructure::repositories::mysql_access::MySqlAccessRepository;

pub struct ApiAutomationOperations;

pub fn automation_operations() -> SharedAutomationOperations {
    Arc::new(ApiAutomationOperations)
}

pub fn automation_gateway(state: AppState) -> HandlerAutomationGateway {
    HandlerAutomationGateway::new(state, automation_operations())
}

pub fn automation_gateways(state: AppState) -> Arc<dyn AutomationGatewayFactory> {
    HandlerAutomationGatewayFactory::shared(state, automation_operations())
}

#[async_trait]
impl AutomationOperations for ApiAutomationOperations {
    async fn reference_exists(
        &self,
        state: &AppState,
        reference: Reference<'_>,
    ) -> Result<bool, AppError> {
        let found = if reference.kind == USER {
            match UserId::new(reference.id) {
                Ok(user) => MySqlAccessRepository::new(state.db()?)
                    .load_access(&user)
                    .await?
                    .is_some(),
                Err(_) => false,
            }
        } else {
            false
        };
        Ok(found)
    }

    async fn call_webhook(
        &self,
        state: &AppState,
        call: WebhookCall,
    ) -> Result<WebhookCallResult, AppError> {
        let network = state.webhook_network()?;
        if let Err(rejection) = network.guard().check(&call.url).await {
            let message = match rejection {
                UrlRejection::Invalid(message) | UrlRejection::NotAllowed(message) => message,
            };
            return Err(AppError::Validation(format!(
                "the webhook URL was refused: {message}"
            )));
        }
        let timestamp = Utc::now().timestamp();
        let signature = match &call.signing_secret {
            Some(secret) => sign_payload(secret, timestamp, &call.body)?,
            None => String::new(),
        };
        let response = network
            .http_sender()
            .send(&WebhookRequest {
                url: call.url,
                delivery_id: call.delivery_id,
                event_type: call.event_type,
                timestamp,
                signature,
                body: call.body,
                method: call.method,
                headers: call.headers,
                content_type: call.content_type,
            })
            .await;
        Ok(WebhookCallResult {
            status: response.status,
            duration_ms: response.duration_ms,
            error: response.error,
        })
    }

    async fn link(
        &self,
        state: &AppState,
        scope: &AccessScope,
        from: &RecordRef,
        relation: &str,
        target: Uuid,
    ) -> Result<(), AppError> {
        relation_use_cases(state)
            .await?
            .link(
                scope,
                from.record_type.as_str(),
                &from.record_id.to_string(),
                relation,
                &target.to_string(),
                None,
            )
            .await
            .map(|_| ())
    }
}
