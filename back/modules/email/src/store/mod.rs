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
    registry.register(Arc::new(imap::ImapMailboxProvider::from_env()));
    for (_, provider) in
        aphel_kernel::modules::app().all_contributions::<dyn crate::MailboxProvider>()
    {
        registry.register(provider);
    }
    Arc::new(registry)
}
