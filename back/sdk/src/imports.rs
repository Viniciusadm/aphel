use crate::{AppError, fields::EntityTraits};
pub use aphel_contract_imports::*;

pub trait ImportTraits {
    fn import_policy(self, policy: ImportFieldPolicy) -> Self;
    fn import_policy_data(&self) -> Result<ImportFieldPolicy, AppError>;
}
impl ImportTraits for EntityTraits {
    fn import_policy(self, policy: ImportFieldPolicy) -> Self {
        self.with_trait_data(
            FIELD_POLICY,
            serde_json::to_value(policy).expect("import field metadata"),
        )
    }
    fn import_policy_data(&self) -> Result<ImportFieldPolicy, AppError> {
        self.trait_data(FIELD_POLICY)
            .map(|value| {
                serde_json::from_value(value.clone()).map_err(|error| {
                    AppError::Configuration(format!("invalid import field policy: {error}"))
                })
            })
            .transpose()
            .map(Option::unwrap_or_default)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextEncoding {
    Utf8,
    Windows1252,
}

impl TextEncoding {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Utf8 => "utf-8",
            Self::Windows1252 => "windows-1252",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "utf-8" => Some(Self::Utf8),
            "windows-1252" => Some(Self::Windows1252),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceRow {
    pub row_index: u32,
    pub cells: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedSource {
    pub encoding: TextEncoding,
    pub delimiter: u8,
    pub headers: Vec<String>,
    pub rows: Vec<SourceRow>,
}

#[async_trait::async_trait]
pub trait ImportSources: Send + Sync {
    fn parse(&self, bytes: &[u8]) -> Result<ParsedSource, AppError>;
    async fn record_by_label(
        &self,
        ctx: &crate::Ctx<'_>,
        record_type: &str,
        label: &str,
    ) -> Result<Option<uuid::Uuid>, AppError>;
    async fn member_by_email(
        &self,
        ctx: &crate::Ctx<'_>,
        email: &str,
    ) -> Result<Option<String>, AppError>;
}

pub fn sources() -> Result<std::sync::Arc<dyn ImportSources>, AppError> {
    aphel_kernel::modules::app()
        .services()
        .get::<dyn ImportSources>()
        .ok_or_else(|| {
            AppError::unavailable_code(
                "import_sources_unavailable",
                "import sources are not enabled",
            )
        })
}
