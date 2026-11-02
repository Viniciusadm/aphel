use crate::gateway_impl::{
    AutomationOperations, HandlerAutomationGateway, HandlerAutomationGatewayFactory,
    SharedAutomationOperations,
};
use aphel_contract_automation::application::gateway::USER;
use aphel_kernel::application::security::signature::sign_payload;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use uuid::Uuid;

use aphel_contract_automation::application::{
    AutomationGatewayFactory, Reference, WebhookCall, WebhookCallResult,
};
use aphel_kernel::api::record_services::relation_use_cases;
use aphel_kernel::api::state::AppState;
use aphel_kernel::application::access::AccessRepository;
use aphel_kernel::application::outbound_http::{
    OutboundRequest, OutboundSender, OutboundUrlGuard, UrlRejection,
};
use aphel_kernel::application::shared::RecordRef;
use aphel_kernel::domain::UserId;
use aphel_kernel::domain::access::AccessScope;
use aphel_kernel::errors::AppError;
use aphel_kernel::infrastructure::repositories::mysql_access::MySqlAccessRepository;

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
        let network = state.outbound_http_network()?;
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
        let mut headers = call
            .headers
            .into_iter()
            .filter(|(name, _)| !name.to_ascii_lowercase().starts_with("x-aphel-"))
            .collect::<Vec<_>>();
        headers.extend([
            ("X-Aphel-Delivery".to_owned(), call.delivery_id.to_string()),
            ("X-Aphel-Event".to_owned(), call.event_type),
            ("X-Aphel-Signature".to_owned(), signature),
        ]);
        let response = network
            .http_sender()
            .send(&OutboundRequest {
                url: call.url,
                body: call.body,
                method: call.method,
                headers,
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

pub trait AutomationContext {
    fn automation_gateways(
        &self,
    ) -> std::sync::Arc<dyn aphel_contract_automation::application::AutomationGatewayFactory>;
}
impl AutomationContext for aphel_kernel::Ctx<'_> {
    fn automation_gateways(
        &self,
    ) -> std::sync::Arc<dyn aphel_contract_automation::application::AutomationGatewayFactory> {
        automation_gateways(self.state())
    }
}
