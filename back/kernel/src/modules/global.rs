use std::sync::{Arc, OnceLock};

use crate::modules::builder::App;

static APP: OnceLock<Arc<App>> = OnceLock::new();

pub fn install_app(app: App) -> &'static Arc<App> {
    APP.get_or_init(|| Arc::new(app))
}

pub fn app() -> &'static Arc<App> {
    APP.get()
        .expect("the modules of this build must be installed with install_app before use")
}

pub fn try_app() -> Option<&'static Arc<App>> {
    APP.get()
}
