use std::sync::Arc;

use serde::Serialize;
use serde_json::Value;

use crate::Ctx;
use crate::api::routes::v1::field_permissions::{redacted, redacted_items};
use crate::api::routes::v1::files::file_use_cases;
use crate::application::fields::FieldSet;
use crate::application::files::FileUseCases;
use crate::application::shared::RecordType;
use crate::domain::access::AccessScope;
use crate::errors::AppError;
use crate::infrastructure::outbound_http::OutboundHttpNetwork;
use crate::infrastructure::storage::MySqlFilePorts;
use crate::records::runtime::field_set;

impl Ctx<'_> {
    pub async fn field_set(
        &self,
        scope: &AccessScope,
        record_type: &str,
    ) -> Result<FieldSet, AppError> {
        field_set(&self.state(), scope, record_type).await
    }

    pub async fn redacted<T: Serialize>(
        &self,
        scope: &AccessScope,
        record_type: &RecordType,
        item: T,
    ) -> Result<Value, AppError> {
        redacted(&self.state(), scope, record_type, item).await
    }

    pub async fn redacted_items<T: Serialize>(
        &self,
        scope: &AccessScope,
        record_type: &RecordType,
        items: Vec<T>,
    ) -> Result<Vec<Value>, AppError> {
        redacted_items(&self.state(), scope, record_type, items).await
    }

    pub async fn files(&self) -> Result<FileUseCases<MySqlFilePorts>, AppError> {
        file_use_cases(&self.state()).await
    }

    pub fn outbound_http_network(&self) -> Result<Arc<OutboundHttpNetwork>, AppError> {
        self.state().outbound_http_network()
    }

    pub fn app_base_url(&self) -> String {
        self.state().auth_settings.app_base_url.get()
    }

    pub fn app_root_url(&self) -> String {
        self.state().auth_settings.app_base_url.app_root()
    }

    pub fn public_tools_url(&self) -> String {
        self.state()
            .auth_settings
            .app_base_url
            .tools_root(crate::modules::site_enabled(self.app()))
    }
}
