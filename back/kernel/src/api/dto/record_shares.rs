use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::api::dto::common::timestamp as instant;
use crate::api::dto::files::UserReferenceResponse;
use crate::application::auth::ClientMetadata;
use crate::application::record_sharing::{
    CreateRecordShareCommand, RecordShare, RecordShares, UpdateRecordShareCommand,
};
use crate::domain::UserId;

#[derive(Debug, Serialize, ToSchema)]
pub struct ShareGranteeResponse {
    #[schema(example = "user")]
    pub r#type: String,
    pub id: String,
    pub name: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RecordShareResponse {
    pub id: String,
    #[schema(example = "deal")]
    pub record_type: String,
    pub record_id: String,
    pub grantee: ShareGranteeResponse,
    #[schema(example = "read")]
    pub access: String,
    pub shared_by: Option<UserReferenceResponse>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<RecordShare> for RecordShareResponse {
    fn from(share: RecordShare) -> Self {
        Self {
            id: share.id.to_string(),
            record_type: share.record.record_type.as_str().to_owned(),
            record_id: share.record.record_id.to_string(),
            grantee: ShareGranteeResponse {
                r#type: share.grantee.kind.as_str().to_owned(),
                id: share.grantee.id,
                name: share.grantee.name,
            },
            access: share.access.as_str().to_owned(),
            shared_by: share.shared_by.map(Into::into),
            created_at: instant(share.created_at),
            updated_at: instant(share.updated_at),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RecordSharesResponse {
    pub record_type: String,
    pub record_id: String,
    pub owner_id: Option<String>,
    pub can_manage: bool,
    pub shares: Vec<RecordShareResponse>,
}

impl From<RecordShares> for RecordSharesResponse {
    fn from(shares: RecordShares) -> Self {
        Self {
            record_type: shares.record.record_type.as_str().to_owned(),
            record_id: shares.record.record_id.to_string(),
            owner_id: shares.owner_id.map(UserId::into_inner),
            can_manage: shares.can_manage,
            shares: shares.shares.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateRecordShareRequest {
    #[schema(example = "user")]
    pub grantee_type: String,
    pub grantee_id: String,
    #[schema(example = "read")]
    pub access: String,
}

impl CreateRecordShareRequest {
    pub fn into_command(self, client: ClientMetadata) -> CreateRecordShareCommand {
        CreateRecordShareCommand {
            grantee_type: self.grantee_type,
            grantee_id: self.grantee_id,
            access: self.access,
            client,
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateRecordShareRequest {
    #[schema(example = "edit")]
    pub access: String,
}

impl UpdateRecordShareRequest {
    pub fn into_command(
        self,
        client: ClientMetadata,
        expected_updated_at: Option<DateTime<Utc>>,
    ) -> UpdateRecordShareCommand {
        UpdateRecordShareCommand {
            access: self.access,
            expected_updated_at,
            client,
        }
    }
}
