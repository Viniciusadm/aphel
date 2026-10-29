use std::net::{IpAddr, SocketAddr};
use std::sync::OnceLock;

use axum::extract::ConnectInfo;
use axum::http::HeaderMap;
use axum::http::request::Parts;

pub const TRUSTED_PROXIES_VAR: &str = "HTTP_TRUSTED_PROXIES";
const FORWARDED_FOR: &str = "x-forwarded-for";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IpNetwork {
    address: IpAddr,
    prefix: u8,
}

impl IpNetwork {
    pub fn parse(value: &str) -> Option<Self> {
        let value = value.trim();
        let (address, prefix) = match value.split_once('/') {
            Some((address, prefix)) => (address.parse::<IpAddr>().ok()?, prefix.parse().ok()?),
            None => {
                let address = value.parse::<IpAddr>().ok()?;
                let prefix = if address.is_ipv4() { 32 } else { 128 };
                (address, prefix)
            }
        };
        let limit = if address.is_ipv4() { 32 } else { 128 };
        (prefix <= limit).then_some(Self { address, prefix })
    }

    pub fn contains(&self, candidate: IpAddr) -> bool {
        match (self.address, canonical(candidate)) {
            (IpAddr::V4(network), IpAddr::V4(candidate)) => {
                let mask = u32::MAX
                    .checked_shl(32 - u32::from(self.prefix))
                    .unwrap_or(0);
                u32::from(network) & mask == u32::from(candidate) & mask
            }
            (IpAddr::V6(network), IpAddr::V6(candidate)) => {
                let mask = u128::MAX
                    .checked_shl(128 - u32::from(self.prefix))
                    .unwrap_or(0);
                u128::from(network) & mask == u128::from(candidate) & mask
            }
            _ => false,
        }
    }
}

fn canonical(address: IpAddr) -> IpAddr {
    match address {
        IpAddr::V6(v6) => v6.to_ipv4_mapped().map_or(address, IpAddr::V4),
        other => other,
    }
}

pub fn parse_trusted_proxies(value: &str) -> Vec<IpNetwork> {
    value
        .split(',')
        .filter(|entry| !entry.trim().is_empty())
        .filter_map(|entry| {
            let parsed = IpNetwork::parse(entry);
            if parsed.is_none() {
                tracing::warn!(
                    entry = entry.trim(),
                    "ignored invalid {TRUSTED_PROXIES_VAR} entry"
                );
            }
            parsed
        })
        .collect()
}

fn trusted_proxies() -> &'static [IpNetwork] {
    static PROXIES: OnceLock<Vec<IpNetwork>> = OnceLock::new();
    PROXIES.get_or_init(|| {
        std::env::var(TRUSTED_PROXIES_VAR)
            .map(|value| parse_trusted_proxies(&value))
            .unwrap_or_default()
    })
}

pub fn resolve_client_ip(peer: IpAddr, headers: &HeaderMap, trusted: &[IpNetwork]) -> IpAddr {
    let peer = canonical(peer);
    let is_trusted = |address: IpAddr| trusted.iter().any(|network| network.contains(address));
    if !is_trusted(peer) {
        return peer;
    }
    let forwarded: Vec<IpAddr> = headers
        .get_all(FORWARDED_FOR)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(','))
        .filter_map(|entry| entry.trim().parse::<IpAddr>().ok())
        .map(canonical)
        .collect();
    forwarded
        .iter()
        .rev()
        .copied()
        .find(|address| !is_trusted(*address))
        .or_else(|| forwarded.first().copied())
        .unwrap_or(peer)
}

pub fn client_ip(parts: &Parts) -> Option<String> {
    let ConnectInfo(address) = parts.extensions.get::<ConnectInfo<SocketAddr>>()?;
    Some(resolve_client_ip(address.ip(), &parts.headers, trusted_proxies()).to_string())
}
