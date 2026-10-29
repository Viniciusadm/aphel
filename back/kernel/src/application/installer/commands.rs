use crate::domain::{Secret, UserId};

#[derive(Clone, PartialEq, Eq)]
pub struct InstallCommand {
    pub organization_name: String,
    pub timezone: String,
    pub currency: String,
    pub locale: String,
    pub admin_name: String,
    pub admin_email: String,
    pub admin_password: Secret,
    pub public_base_url: Option<String>,
    pub ip: Option<String>,
    pub modules: Vec<String>,
    pub module_settings:
        std::collections::BTreeMap<String, serde_json::Map<String, serde_json::Value>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Installation {
    pub user_id: UserId,
    pub admin_email: String,
}
