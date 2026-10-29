#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum RecordScope {
    Own,
    Team,
    Organization,
}

impl RecordScope {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Own => "own",
            Self::Team => "team",
            Self::Organization => "organization",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "own" => Some(Self::Own),
            "team" => Some(Self::Team),
            "organization" => Some(Self::Organization),
            _ => None,
        }
    }
}
