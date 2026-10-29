use crate::domain::access::AccessScope;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TwoFactorPolicy {
    #[default]
    None,
    Admins,
    Everyone,
}

impl TwoFactorPolicy {
    pub const ALL: [Self; 3] = [Self::None, Self::Admins, Self::Everyone];

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Admins => "admins",
            Self::Everyone => "everyone",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|policy| policy.as_str() == value)
    }

    pub fn applies_to(&self, scope: &AccessScope) -> bool {
        self.applies_to_role(scope.role_key().is_admin())
    }

    pub fn applies_to_role(&self, is_admin: bool) -> bool {
        match self {
            Self::None => false,
            Self::Admins => is_admin,
            Self::Everyone => true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TwoFactorStatus {
    pub enabled: bool,
    pub required: bool,
}

impl TwoFactorStatus {
    pub fn blocks_organization(&self) -> bool {
        self.required && !self.enabled
    }
}
