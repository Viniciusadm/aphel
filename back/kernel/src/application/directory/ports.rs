use async_trait::async_trait;

use crate::application::organization_admin::{ListWindow, Listing};
use crate::domain::UserId;
use crate::errors::AppError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectoryUser {
    pub id: UserId,
    pub display_name: String,
    pub email: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DirectoryQuery {
    pub search: Option<String>,
}

#[async_trait]
pub trait UserDirectory: Send + Sync {
    async fn list_active(
        &self,
        query: &DirectoryQuery,
        window: ListWindow,
    ) -> Result<Listing<DirectoryUser>, AppError>;
}
