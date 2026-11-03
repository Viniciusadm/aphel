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
