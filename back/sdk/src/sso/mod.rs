pub use aphel_contract_sso::*;
pub mod oauth;
pub mod states;
pub use oauth::{ClientCredentials, IdTokenClaims, ProviderEndpoints};
pub use states::StoredState;
pub mod errors;
pub mod login;
pub mod settings;
pub mod status;
pub mod support;
pub use login::{
    CallbackQuery, ClientContext, IdentityProvider, StartQuery, callback_url, discover_options,
    handle_callback, start_login,
};
pub struct IdentityLogin<P: IdentityProvider>(pub P);
#[async_trait::async_trait]
impl<P: IdentityProvider + 'static> aphel_kernel::api::sso::LoginProvider for IdentityLogin<P> {
    fn key(&self) -> &'static str {
        self.0.key()
    }
    async fn discover(
        &self,
        state: &aphel_kernel::state::AppState,
        email: &str,
    ) -> Result<Vec<aphel_kernel::api::sso::LoginOption>, crate::AppError> {
        login::discover_options(state, &self.0, email).await
    }
}
