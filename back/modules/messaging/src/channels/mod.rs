pub(crate) mod whatsapp;
use crate::{KEY, store::StoredChannel};
use aphel_kernel::{
    Ctx,
    database::transaction::Tx,
    domain::{Secret, access::AccessScope},
    errors::AppError,
    modules::module_secret_key,
};
pub(crate) use aphel_sdk::messaging::*;
use std::collections::BTreeMap;
pub(crate) use whatsapp::{APP_SECRET, VERIFY_TOKEN, parse_webhook};
pub(crate) fn secret_name(channel_id: uuid::Uuid, name: &str) -> String {
    format!("channel:{channel_id}:{name}")
}

pub(crate) async fn load_credentials(
    ctx: &Ctx<'_>,
    channel: &StoredChannel,
) -> Result<ChannelCredentials, AppError> {
    let secrets = ctx.secrets()?;
    let system = AccessScope::system();
    let mut credentials = ChannelCredentials::default();
    for name in compiled_channel_provider(channel.kind.as_str())
        .map(|provider| provider.secret_names())
        .unwrap_or_default()
    {
        if let Some(value) = secrets
            .reveal(
                &system,
                &module_secret_key(KEY, &secret_name(channel.id, name)),
            )
            .await?
        {
            credentials.insert(name, value.expose().to_owned());
        }
    }
    Ok(credentials)
}

pub(crate) async fn configured_secrets(
    ctx: &Ctx<'_>,
    channel: &StoredChannel,
) -> Result<BTreeMap<String, bool>, AppError> {
    let credentials = load_credentials(ctx, channel).await?;
    Ok(compiled_channel_provider(channel.kind.as_str())
        .map(|provider| provider.secret_names())
        .unwrap_or_default()
        .iter()
        .map(|name| ((*name).to_owned(), credentials.get(name).is_some()))
        .collect())
}

pub(crate) async fn store_credential(
    ctx: &Ctx<'_>,
    tx: &mut Tx,
    channel_id: uuid::Uuid,
    name: &str,
    value: &str,
) -> Result<(), AppError> {
    ctx.secrets()?
        .store(
            tx,
            ctx.scope,
            &module_secret_key(KEY, &secret_name(channel_id, name)),
            &Secret::new(value.to_owned()),
        )
        .await
}

pub(crate) async fn remove_credential(
    ctx: &Ctx<'_>,
    tx: &mut Tx,
    channel_id: uuid::Uuid,
    name: &str,
) -> Result<(), AppError> {
    ctx.secrets()?
        .remove(
            tx,
            ctx.scope,
            &module_secret_key(KEY, &secret_name(channel_id, name)),
        )
        .await
}

pub(crate) async fn remove_credentials(
    ctx: &Ctx<'_>,
    tx: &mut Tx,
    channel: &StoredChannel,
) -> Result<(), AppError> {
    for name in compiled_channel_provider(channel.kind.as_str())
        .map(|provider| provider.secret_names())
        .unwrap_or_default()
    {
        remove_credential(ctx, tx, channel.id, name).await?;
    }
    Ok(())
}

pub(crate) fn http_error(error: &reqwest::Error) -> ChannelError {
    if error.is_timeout() {
        ChannelError::transient(
            "messaging_provider_timeout",
            "the provider did not answer in time",
        )
    } else if error.is_connect() {
        ChannelError::transient(
            "messaging_provider_unreachable",
            "could not connect to the provider",
        )
    } else {
        ChannelError::transient(
            "messaging_provider_error",
            "communication with the provider failed",
        )
    }
}
