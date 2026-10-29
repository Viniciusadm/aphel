use std::fmt::Debug;
use std::future::Future;
use std::net::SocketAddr;
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use async_imap::{Client, Session};
use futures::TryStreamExt;
use rustls::pki_types::ServerName;
use rustls::{ClientConfig, RootCertStore};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;

use crate::MailProviderError;
use crate::store::imap::config::{ImapSettings, NetworkPolicy, Security};

pub(crate) const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);
pub(crate) const COMMAND_TIMEOUT: Duration = Duration::from_secs(60);

pub(crate) trait MailStream: AsyncRead + AsyncWrite + Unpin + Send + Debug {}

impl<T> MailStream for T where T: AsyncRead + AsyncWrite + Unpin + Send + Debug {}

pub(crate) type ImapSession = Session<Box<dyn MailStream>>;

pub(crate) async fn timed<T, E>(
    future: impl Future<Output = Result<T, E>>,
    limit: Duration,
    failure: impl FnOnce(E) -> MailProviderError,
) -> Result<T, MailProviderError> {
    match tokio::time::timeout(limit, future).await {
        Ok(Ok(value)) => Ok(value),
        Ok(Err(error)) => Err(failure(error)),
        Err(_) => Err(MailProviderError::connection(
            "the mail server did not answer in time",
        )),
    }
}

pub(crate) fn imap_failure<E: std::fmt::Display>(error: E) -> MailProviderError {
    MailProviderError::connection(format!("the IMAP server returned an error: {error}"))
}

fn tls_config() -> Result<Arc<ClientConfig>, MailProviderError> {
    static CONFIG: OnceLock<Arc<ClientConfig>> = OnceLock::new();
    if let Some(config) = CONFIG.get() {
        return Ok(config.clone());
    }
    let roots = RootCertStore {
        roots: webpki_roots::TLS_SERVER_ROOTS.to_vec(),
    };
    let config =
        ClientConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
            .with_safe_default_protocol_versions()
            .map_err(|_| MailProviderError::connection("could not configure TLS"))?
            .with_root_certificates(roots)
            .with_no_client_auth();
    Ok(CONFIG.get_or_init(|| Arc::new(config)).clone())
}

pub(crate) async fn resolve(
    host: &str,
    port: u16,
    policy: &NetworkPolicy,
) -> Result<SocketAddr, MailProviderError> {
    let addresses: Vec<SocketAddr> = timed(
        tokio::net::lookup_host((host, port)),
        CONNECT_TIMEOUT,
        |_| MailProviderError::connection(format!("could not resolve the host {host}")),
    )
    .await?
    .collect();
    if addresses.is_empty() {
        return Err(MailProviderError::connection(format!(
            "could not resolve the host {host}"
        )));
    }
    if addresses.iter().any(|address| !policy.allows(address.ip())) {
        return Err(MailProviderError::rejected(format!(
            "the host {host} resolves to a private or reserved address"
        )));
    }
    Ok(addresses[0])
}

async fn upgrade(
    stream: TcpStream,
    host: &str,
) -> Result<tokio_rustls::client::TlsStream<TcpStream>, MailProviderError> {
    let name = ServerName::try_from(host.to_owned())
        .map_err(|_| MailProviderError::rejected("invalid server host name"))?;
    timed(
        TlsConnector::from(tls_config()?).connect(name, stream),
        CONNECT_TIMEOUT,
        |error| MailProviderError::connection(format!("the TLS handshake failed: {error}")),
    )
    .await
}

async fn greet(client: &mut Client<Box<dyn MailStream>>) -> Result<(), MailProviderError> {
    let greeting = timed(client.read_response(), COMMAND_TIMEOUT, imap_failure).await?;
    if greeting.is_none() {
        return Err(MailProviderError::connection(
            "the IMAP server closed the connection",
        ));
    }
    Ok(())
}

pub(crate) async fn open_session(
    settings: &ImapSettings,
    password: &str,
    policy: &NetworkPolicy,
) -> Result<ImapSession, MailProviderError> {
    let address = resolve(&settings.imap_host, settings.imap_port, policy).await?;
    let tcp = timed(TcpStream::connect(address), CONNECT_TIMEOUT, |_| {
        MailProviderError::connection("could not connect to the IMAP server")
    })
    .await?;
    let client = match settings.imap_security {
        Security::Tls => {
            let stream: Box<dyn MailStream> = Box::new(upgrade(tcp, &settings.imap_host).await?);
            let mut client = Client::new(stream);
            greet(&mut client).await?;
            client
        }
        Security::None => {
            let stream: Box<dyn MailStream> = Box::new(tcp);
            let mut client = Client::new(stream);
            greet(&mut client).await?;
            client
        }
        Security::StartTls => {
            let mut plain = Client::new(tcp);
            let greeting = timed(plain.read_response(), COMMAND_TIMEOUT, imap_failure).await?;
            if greeting.is_none() {
                return Err(MailProviderError::connection(
                    "the IMAP server closed the connection",
                ));
            }
            timed(
                plain.run_command_and_check_ok("STARTTLS", None),
                COMMAND_TIMEOUT,
                |_| MailProviderError::connection("the IMAP server refused STARTTLS"),
            )
            .await?;
            let stream: Box<dyn MailStream> =
                Box::new(upgrade(plain.into_inner(), &settings.imap_host).await?);
            Client::new(stream)
        }
    };
    timed(
        async {
            client
                .login(&settings.username, password)
                .await
                .map_err(|(error, _)| error)
        },
        COMMAND_TIMEOUT,
        |_| MailProviderError::authentication("the IMAP server rejected the user name or password"),
    )
    .await
}

pub(crate) async fn close(mut session: ImapSession) {
    let _ = tokio::time::timeout(COMMAND_TIMEOUT, session.logout()).await;
}

pub(crate) async fn find_sent_folder(
    session: &mut ImapSession,
) -> Result<Option<String>, MailProviderError> {
    let names = {
        let stream = timed(session.list(None, Some("*")), COMMAND_TIMEOUT, imap_failure).await?;
        timed(
            stream.try_collect::<Vec<_>>(),
            COMMAND_TIMEOUT,
            imap_failure,
        )
        .await?
    };
    let special = names.iter().find(|name| {
        name.attributes()
            .iter()
            .any(|attribute| matches!(attribute, async_imap::types::NameAttribute::Sent))
    });
    if let Some(name) = special {
        return Ok(Some(name.name().to_owned()));
    }
    const COMMON: [&str; 8] = [
        "Sent",
        "Sent Items",
        "Sent Messages",
        "Sent Mail",
        "INBOX.Sent",
        "INBOX/Sent",
        "Enviados",
        "Itens Enviados",
    ];
    Ok(COMMON.iter().find_map(|candidate| {
        names
            .iter()
            .find(|name| name.name().eq_ignore_ascii_case(candidate))
            .map(|name| name.name().to_owned())
    }))
}
