use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use reqwest::Url;
use reqwest::dns::{Addrs, Name, Resolve, Resolving};

use crate::application::webhooks::outgoing::{UrlRejection, WebhookUrlGuard};
use crate::config::WebhooksConfig;

const DNS_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WebhookNetworkPolicy {
    pub(crate) allow_private_networks: bool,
    pub(crate) require_https: bool,
}

impl WebhookNetworkPolicy {
    pub(crate) fn from_config(config: &WebhooksConfig) -> Self {
        Self {
            allow_private_networks: config.allow_private_networks,
            require_https: config.require_https,
        }
    }

    pub(crate) fn allows(&self, ip: IpAddr) -> bool {
        self.allow_private_networks || is_public_ip(ip)
    }
}

fn is_public_ipv4(ip: Ipv4Addr) -> bool {
    let [a, b, c, _] = ip.octets();
    !(ip.is_unspecified()
        || ip.is_loopback()
        || ip.is_private()
        || ip.is_link_local()
        || ip.is_multicast()
        || ip.is_broadcast()
        || ip.is_documentation()
        || a == 0
        || a >= 240
        || (a == 100 && (64..=127).contains(&b))
        || (a == 192 && b == 0 && c == 0)
        || (a == 198 && (18..=19).contains(&b)))
}

fn is_public_ipv6(ip: Ipv6Addr) -> bool {
    if let Some(mapped) = ip.to_ipv4_mapped() {
        return is_public_ipv4(mapped);
    }
    let segments = ip.segments();
    if segments[0] == 0x0064 && segments[1] == 0xff9b {
        let embedded = Ipv4Addr::new(
            (segments[6] >> 8) as u8,
            segments[6] as u8,
            (segments[7] >> 8) as u8,
            segments[7] as u8,
        );
        return is_public_ipv4(embedded);
    }
    !(ip.is_unspecified()
        || ip.is_loopback()
        || ip.is_multicast()
        || (segments[0] & 0xfe00) == 0xfc00
        || (segments[0] & 0xffc0) == 0xfe80
        || (segments[0] & 0xffc0) == 0xfec0
        || (segments[0] == 0x2001 && segments[1] == 0x0db8)
        || segments[..6].iter().all(|segment| *segment == 0))
}

pub fn is_public_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => is_public_ipv4(ip),
        IpAddr::V6(ip) => is_public_ipv6(ip),
    }
}

fn not_allowed() -> UrlRejection {
    UrlRejection::NotAllowed(
        "the host resolves to a private, loopback, link-local or reserved address".to_owned(),
    )
}

pub(crate) fn parse_webhook_url(
    raw: &str,
    policy: &WebhookNetworkPolicy,
) -> Result<Url, UrlRejection> {
    let url = Url::parse(raw)
        .map_err(|_| UrlRejection::Invalid("must be an absolute http(s) URL".to_owned()))?;
    match url.scheme() {
        "https" => {}
        "http" if !policy.require_https => {}
        "http" => {
            return Err(UrlRejection::NotAllowed(
                "only https URLs are allowed".to_owned(),
            ));
        }
        _ => {
            return Err(UrlRejection::Invalid(
                "must be an absolute http(s) URL".to_owned(),
            ));
        }
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err(UrlRejection::Invalid(
            "must not contain credentials".to_owned(),
        ));
    }
    if url.fragment().is_some() {
        return Err(UrlRejection::Invalid(
            "must not contain a fragment".to_owned(),
        ));
    }
    match target_host(&url) {
        None => Err(UrlRejection::Invalid("must have a host".to_owned())),
        Some(TargetHost::Ip(ip)) if !policy.allows(ip) => Err(not_allowed()),
        Some(TargetHost::Ip(_)) => Ok(url),
        Some(TargetHost::Domain(domain)) => {
            let domain = domain.trim_end_matches('.').to_ascii_lowercase();
            let local = domain == "localhost"
                || domain.ends_with(".localhost")
                || domain.ends_with(".local")
                || domain.ends_with(".internal");
            if local && !policy.allow_private_networks {
                Err(not_allowed())
            } else {
                Ok(url)
            }
        }
    }
}

pub(crate) enum TargetHost<'a> {
    Ip(IpAddr),
    Domain(&'a str),
}

pub(crate) fn target_host(url: &Url) -> Option<TargetHost<'_>> {
    let host = url.host_str().filter(|host| !host.is_empty())?;
    let unbracketed = host.trim_start_matches('[').trim_end_matches(']');
    Some(match unbracketed.parse::<IpAddr>() {
        Ok(ip) => TargetHost::Ip(ip),
        Err(_) => TargetHost::Domain(host),
    })
}

async fn lookup(host: &str, port: u16) -> Result<Vec<SocketAddr>, UrlRejection> {
    let unresolved = || UrlRejection::Invalid("the host could not be resolved".to_owned());
    let addresses = tokio::time::timeout(DNS_TIMEOUT, tokio::net::lookup_host((host, port)))
        .await
        .map_err(|_| unresolved())?
        .map_err(|_| unresolved())?
        .collect::<Vec<_>>();
    if addresses.is_empty() {
        return Err(unresolved());
    }
    Ok(addresses)
}

#[derive(Debug, Clone, Copy)]
pub struct NetworkWebhookUrlGuard {
    policy: WebhookNetworkPolicy,
}

impl NetworkWebhookUrlGuard {
    pub(crate) fn new(policy: WebhookNetworkPolicy) -> Self {
        Self { policy }
    }
}

#[async_trait]
impl WebhookUrlGuard for NetworkWebhookUrlGuard {
    async fn check(&self, raw: &str) -> Result<(), UrlRejection> {
        let url = parse_webhook_url(raw, &self.policy)?;
        let Some(TargetHost::Domain(domain)) = target_host(&url) else {
            return Ok(());
        };
        let port = url.port_or_known_default().unwrap_or(443);
        let addresses = lookup(domain, port).await?;
        if addresses
            .iter()
            .all(|address| self.policy.allows(address.ip()))
        {
            Ok(())
        } else {
            Err(not_allowed())
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct GuardedResolver {
    policy: WebhookNetworkPolicy,
}

impl GuardedResolver {
    pub(crate) fn shared(policy: WebhookNetworkPolicy) -> Arc<Self> {
        Arc::new(Self { policy })
    }
}

impl Resolve for GuardedResolver {
    fn resolve(&self, name: Name) -> Resolving {
        let policy = self.policy;
        Box::pin(async move {
            let addresses = lookup(name.as_str(), 0).await.map_err(
                |_| -> Box<dyn std::error::Error + Send + Sync> {
                    "the host could not be resolved".into()
                },
            )?;
            if !addresses.iter().all(|address| policy.allows(address.ip())) {
                return Err("the host resolves to an address that is not allowed".into());
            }
            let addrs: Addrs = Box::new(addresses.into_iter());
            Ok(addrs)
        })
    }
}
