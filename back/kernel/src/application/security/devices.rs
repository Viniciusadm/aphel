use std::net::IpAddr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceKind {
    Desktop,
    Mobile,
    Tablet,
    Bot,
    Unknown,
}

impl DeviceKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Desktop => "desktop",
            Self::Mobile => "mobile",
            Self::Tablet => "tablet",
            Self::Bot => "bot",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeviceInfo {
    pub browser: Option<&'static str>,
    pub os: Option<&'static str>,
    pub kind: DeviceKind,
}

impl DeviceInfo {
    pub fn label(&self) -> String {
        match (self.browser, self.os) {
            (Some(browser), Some(os)) => format!("{browser} ({os})"),
            (Some(browser), None) => browser.to_owned(),
            (None, Some(os)) => os.to_owned(),
            (None, None) => "Dispositivo desconhecido".to_owned(),
        }
    }
}

const BROWSERS: &[(&str, &str)] = &[
    ("Edg/", "Edge"),
    ("EdgA/", "Edge"),
    ("EdgiOS/", "Edge"),
    ("Edge/", "Edge"),
    ("OPR/", "Opera"),
    ("Opera", "Opera"),
    ("SamsungBrowser/", "Samsung Internet"),
    ("YaBrowser/", "Yandex"),
    ("Vivaldi/", "Vivaldi"),
    ("FxiOS/", "Firefox"),
    ("Firefox/", "Firefox"),
    ("CriOS/", "Chrome"),
    ("Chromium/", "Chromium"),
    ("Chrome/", "Chrome"),
    ("Safari/", "Safari"),
    ("curl/", "curl"),
    ("PostmanRuntime/", "Postman"),
    ("python-requests/", "Python"),
    ("okhttp/", "OkHttp"),
];

const SYSTEMS: &[(&str, &str)] = &[
    ("Windows", "Windows"),
    ("iPhone", "iOS"),
    ("iPad", "iPadOS"),
    ("iPod", "iOS"),
    ("Android", "Android"),
    ("CrOS", "ChromeOS"),
    ("Macintosh", "macOS"),
    ("Mac OS X", "macOS"),
    ("Linux", "Linux"),
];

const BOT_MARKERS: &[&str] = &["bot", "crawler", "spider", "headless"];

pub fn describe_device(user_agent: Option<&str>) -> DeviceInfo {
    let Some(agent) = user_agent.map(str::trim).filter(|agent| !agent.is_empty()) else {
        return DeviceInfo {
            browser: None,
            os: None,
            kind: DeviceKind::Unknown,
        };
    };
    let browser = BROWSERS
        .iter()
        .find(|(marker, _)| agent.contains(marker))
        .map(|(_, name)| *name);
    let os = SYSTEMS
        .iter()
        .find(|(marker, _)| agent.contains(marker))
        .map(|(_, name)| *name);
    let lowered = agent.to_ascii_lowercase();
    let kind = if BOT_MARKERS.iter().any(|marker| lowered.contains(marker)) {
        DeviceKind::Bot
    } else if agent.contains("iPad")
        || agent.contains("Tablet")
        || (agent.contains("Android") && !agent.contains("Mobile"))
    {
        DeviceKind::Tablet
    } else if agent.contains("Mobi") || agent.contains("iPhone") || agent.contains("iPod") {
        DeviceKind::Mobile
    } else if os.is_some() {
        DeviceKind::Desktop
    } else {
        DeviceKind::Unknown
    };
    DeviceInfo { browser, os, kind }
}

pub fn network_of(ip: Option<&str>) -> String {
    match ip.and_then(|ip| ip.trim().parse::<IpAddr>().ok()) {
        Some(IpAddr::V4(address)) => {
            let [a, b, c, _] = address.octets();
            format!("{a}.{b}.{c}.0/24")
        }
        Some(IpAddr::V6(address)) => match address.to_ipv4_mapped() {
            Some(mapped) => network_of(Some(&mapped.to_string())),
            None => {
                let segments = address.segments();
                format!("{:x}:{:x}:{:x}::/48", segments[0], segments[1], segments[2])
            }
        },
        None => "unknown".to_owned(),
    }
}

pub fn device_key(user_agent: Option<&str>, ip: Option<&str>) -> String {
    let device = describe_device(user_agent);
    format!(
        "{}|{}|{}|{}",
        device.browser.unwrap_or("unknown"),
        device.os.unwrap_or("unknown"),
        device.kind.as_str(),
        network_of(ip)
    )
}
