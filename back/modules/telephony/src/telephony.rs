use aphel_kernel::domain::access::AccessScope;
use aphel_sdk::telephony::{SharedTelephonyProvider, TelephonyProvider};
pub(crate) use aphel_sdk::telephony::{
    TelephonyRecordingRequest, TelephonyRouter, TelephonyStartRequest,
};
use std::sync::Arc;

struct RegistryTelephonyRouter;

impl TelephonyRouter for RegistryTelephonyRouter {
    fn providers(&self, scope: &AccessScope) -> Vec<SharedTelephonyProvider> {
        let app = aphel_kernel::modules::app();
        let mut providers: Vec<SharedTelephonyProvider> = Vec::new();
        for (owner, provider) in app.all_contributions::<dyn TelephonyProvider>() {
            if !app.is_enabled(owner) || !scope.module_enabled(owner) {
                continue;
            }
            if provider
                .module()
                .is_some_and(|module| !scope.module_enabled(module))
            {
                continue;
            }
            providers.retain(|existing| existing.key() != provider.key());
            providers.push(provider);
        }
        providers
    }
}

pub(crate) fn shared_router() -> Arc<dyn TelephonyRouter> {
    Arc::new(RegistryTelephonyRouter)
}
