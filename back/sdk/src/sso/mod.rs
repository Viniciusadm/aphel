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

pub struct CallbackHints(pub &'static str);

#[async_trait::async_trait]
impl aphel_kernel::modules::ModuleSettingsTester for CallbackHints {
    fn module(&self) -> &'static str {
        self.0
    }

    fn can_test(&self) -> bool {
        false
    }

    fn has_hints(&self) -> bool {
        true
    }

    async fn hints(
        &self,
        ctx: &aphel_kernel::Ctx<'_>,
    ) -> Result<Vec<aphel_kernel::modules::SetupHint>, crate::AppError> {
        Ok(vec![aphel_kernel::modules::SetupHint {
            key: "callback_url",
            value: login::callback_url(ctx, self.0),
        }])
    }
}
