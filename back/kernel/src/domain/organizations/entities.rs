use crate::domain::{NonEmptyString, UtcDateTime};

pub const DEFAULT_LOCALE: &str = "pt-BR";
pub const DEFAULT_TIMEZONE: &str = "America/Sao_Paulo";
pub const DEFAULT_CURRENCY: &str = "BRL";
pub const ORGANIZATION_RECORD_ID: &str = "default";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Organization {
    pub name: NonEmptyString,
    pub logo_file_id: Option<String>,
    pub primary_color: Option<String>,
    pub locale: String,
    pub timezone: String,
    pub currency: String,
    pub installed_at: Option<UtcDateTime>,
    pub created_at: UtcDateTime,
    pub updated_at: UtcDateTime,
}
