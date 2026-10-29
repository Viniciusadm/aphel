#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum DuplicateFamily {
    Contact,
    Company,
    Generic,
}

impl DuplicateFamily {
    pub(crate) fn of(record_type: &str) -> Self {
        match record_type {
            "contact" => Self::Contact,
            "company" => Self::Company,
            _ => Self::Generic,
        }
    }

    pub(crate) fn rules(self, has_identifiers: bool) -> Vec<DuplicateRule> {
        match self {
            Self::Contact => vec![
                DuplicateRule::Email,
                DuplicateRule::Phone,
                DuplicateRule::NameAndCompany,
            ],
            Self::Company => vec![DuplicateRule::Domain, DuplicateRule::Name],
            Self::Generic if has_identifiers => {
                vec![DuplicateRule::Identifier, DuplicateRule::Name]
            }
            Self::Generic => vec![DuplicateRule::Name],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) enum DuplicateRule {
    Email,
    Phone,
    NameAndCompany,
    Domain,
    Name,
    Identifier,
}

impl DuplicateRule {
    pub(crate) const ALL: [Self; 6] = [
        Self::Email,
        Self::Phone,
        Self::NameAndCompany,
        Self::Domain,
        Self::Name,
        Self::Identifier,
    ];

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Email => "email",
            Self::Phone => "phone",
            Self::NameAndCompany => "name_company",
            Self::Domain => "domain",
            Self::Name => "name",
            Self::Identifier => "identifier",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|rule| rule.as_str() == value.trim())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum CandidateStatus {
    Pending,
    Dismissed,
    Merged,
}

impl CandidateStatus {
    pub(crate) const ALL: [Self; 3] = [Self::Pending, Self::Dismissed, Self::Merged];

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Dismissed => "dismissed",
            Self::Merged => "merged",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|status| status.as_str() == value.trim())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum MergeSide {
    Primary,
    Duplicate,
}

impl MergeSide {
    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value.trim() {
            "primary" => Some(Self::Primary),
            "duplicate" => Some(Self::Duplicate),
            _ => None,
        }
    }
}

pub(crate) fn ordered_pair<T: Ord + Copy>(left: T, right: T) -> (T, T) {
    if left <= right {
        (left, right)
    } else {
        (right, left)
    }
}
