use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TenantStatus {
    Provisioning,
    Active,
    Suspended,
    Deleting,
}

impl TenantStatus {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "provisioning" => Some(Self::Provisioning),
            "active" => Some(Self::Active),
            "suspended" => Some(Self::Suspended),
            "deleting" => Some(Self::Deleting),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Provisioning => "provisioning",
            Self::Active => "active",
            Self::Suspended => "suspended",
            Self::Deleting => "deleting",
        }
    }

    pub fn serves_requests(self) -> bool {
        self == Self::Active
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tenant {
    pub id: Uuid,
    pub slug: String,
    pub name: String,
    pub database_name: String,
    pub status: TenantStatus,
}

impl Tenant {
    pub fn is_served_by(&self, other: &Tenant) -> bool {
        self.id == other.id
            && self.database_name == other.database_name
            && self.status.serves_requests()
            && other.status.serves_requests()
    }
}

pub fn normalize_host(raw: &str) -> Option<String> {
    let raw = raw.trim();
    let host = if raw.starts_with('[') {
        &raw[..=raw.find(']')?]
    } else {
        raw.split(':').next().unwrap_or_default()
    };
    let host = host.trim_end_matches('.').to_ascii_lowercase();
    let valid = !host.is_empty()
        && host.len() <= 253
        && host.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '-' | '.' | '[' | ']' | ':')
        });
    valid.then_some(host)
}
