pub(crate) mod automation_source;
pub(crate) mod followers;
pub(crate) mod points;

pub mod imap;
pub(crate) mod links;
pub mod mailboxes;
pub(crate) mod messages;
pub(crate) mod sending;
pub(crate) mod settings;
pub(crate) mod timeline;

pub(crate) use links::*;
pub(crate) use mailboxes::*;
pub(crate) use messages::*;

use std::sync::{Arc, OnceLock};

use crate::MailboxProviderRegistry;

static MAIL_PROVIDERS: OnceLock<Arc<MailboxProviderRegistry>> = OnceLock::new();

pub fn mail_providers() -> &'static Arc<MailboxProviderRegistry> {
    MAIL_PROVIDERS.get_or_init(mail_provider_registry)
}

fn mail_provider_registry() -> Arc<MailboxProviderRegistry> {
    let mut registry = MailboxProviderRegistry::new();
    registry.register(crate::KEY, Arc::new(imap::ImapMailboxProvider::from_env()));
    for (owner, provider) in
        aphel_sdk::platform::modules::app().all_contributions::<dyn crate::MailboxProvider>()
    {
        registry.register(owner, provider);
    }
    Arc::new(registry)
}
