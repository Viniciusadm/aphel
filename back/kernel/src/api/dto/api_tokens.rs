use chrono::{DateTime, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::application::api_tokens::{
    ApiToken, ApiTokenOwnerFilter, CreateApiTokenCommand, CreatedApiToken,
};
use crate::domain::api_tokens::ApiTokenScope;

fn timestamp(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(SecondsFormat::Secs, true)
}

#[derive(Debug, Default, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct ApiTokenListParams {
    #[param(example = "mine")]
    pub owner: Option<String>,
}

impl ApiTokenListParams {
    pub fn owner(&self) -> Option<ApiTokenOwnerFilter> {
        match self.owner.as_deref().map(str::trim) {
            None | Some("") | Some("mine") => Some(ApiTokenOwnerFilter::Mine),
            Some("all") => Some(ApiTokenOwnerFilter::All),
            Some(_) => None,
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CreateApiTokenRequest {
    #[schema(example = "Website integration")]
    pub name: String,
    #[schema(example = json!(["contacts:read", "contacts:write", "deals:write"]))]
    pub scopes: Vec<String>,
    #[schema(value_type = Option<String>, format = DateTime, example = "2027-01-31T23:59:59Z")]
    pub expires_at: Option<DateTime<Utc>>,
}

impl CreateApiTokenRequest {
    pub fn into_command(self, ip: Option<String>) -> CreateApiTokenCommand {
        CreateApiTokenCommand {
            name: self.name,
            scopes: self.scopes,
            expires_at: self.expires_at,
            ip,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ApiTokenResponse {
    #[schema(example = "0192a1b2-0000-7000-8000-000000000001")]
    pub id: String,
    #[schema(example = "Website integration")]
    pub name: String,
    #[schema(example = "aphel_3f9a1c2b")]
    pub token_prefix: String,
    #[schema(example = json!(["contacts:read", "contacts:write"]))]
    pub scopes: Vec<String>,
    pub created_by: String,
    pub created_by_name: Option<String>,
    #[schema(example = "2027-01-31T23:59:59Z")]
    pub expires_at: Option<String>,
    pub last_used_at: Option<String>,
    #[schema(example = "203.0.113.10")]
    pub last_used_ip: Option<String>,
    pub revoked_at: Option<String>,
    #[schema(example = "2026-10-20T13:45:00Z")]
    pub created_at: String,
}

impl From<ApiToken> for ApiTokenResponse {
    fn from(token: ApiToken) -> Self {
        Self {
            id: token.id.to_string(),
            name: token.name,
            token_prefix: token.token_prefix,
            scopes: token.scopes.to_strings(),
            created_by: token.created_by.as_str().to_owned(),
            created_by_name: token.created_by_name,
            expires_at: token.expires_at.map(timestamp),
            last_used_at: token.last_used_at.map(timestamp),
            last_used_ip: token.last_used_ip,
            revoked_at: token.revoked_at.map(timestamp),
            created_at: timestamp(token.created_at),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CreatedApiTokenResponse {
    #[serde(flatten)]
    pub token: ApiTokenResponse,
    #[schema(
        example = "aphel_3f9a1c2b_9c1e0d7a4b2f6e8d0a1b3c5d7e9f1a2b4c6d8e0f1a3b5c7d9e1f2a4b6c8d0e2f4a"
    )]
    pub secret: String,
}

impl From<CreatedApiToken> for CreatedApiTokenResponse {
    fn from(created: CreatedApiToken) -> Self {
        Self {
            secret: created.secret.expose().to_owned(),
            token: created.token.into(),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ApiTokenScopeResponse {
    #[schema(example = "contacts:read")]
    pub scope: String,
    #[schema(example = "contacts")]
    pub resource: String,
    #[schema(example = "read")]
    pub access: String,
}

impl From<ApiTokenScope> for ApiTokenScopeResponse {
    fn from(scope: ApiTokenScope) -> Self {
        Self {
            scope: scope.to_string(),
            resource: scope.resource().to_owned(),
            access: scope.access().as_str().to_owned(),
        }
    }
}
