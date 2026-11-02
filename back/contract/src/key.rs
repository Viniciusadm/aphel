use std::fmt;
use std::marker::PhantomData;

pub const fn is_valid_key(key: &str) -> bool {
    let bytes = key.as_bytes();
    if bytes.is_empty() || bytes.len() > 100 {
        return false;
    }
    let mut segments = 1;
    let mut segment_start = true;
    let mut index = 0;
    while index < bytes.len() {
        let byte = bytes[index];
        if byte == b'.' {
            if segment_start {
                return false;
            }
            segments += 1;
            segment_start = true;
        } else if byte.is_ascii_lowercase()
            || (!segment_start && (byte.is_ascii_digit() || byte == b'_'))
        {
            segment_start = false;
        } else {
            return false;
        }
        index += 1;
    }
    !segment_start && segments >= 2
}

pub(crate) const fn is_valid_name(name: &str) -> bool {
    let bytes = name.as_bytes();
    if bytes.is_empty() {
        return false;
    }
    let mut index = 0;
    while index < bytes.len() {
        let byte = bytes[index];
        let valid = byte.is_ascii_lowercase()
            || (index > 0 && (byte.is_ascii_digit() || byte == b'_' || byte == b'-'));
        if !valid {
            return false;
        }
        index += 1;
    }
    true
}

const fn major_of(version: &str) -> u32 {
    let bytes = version.as_bytes();
    let mut major = 0u32;
    let mut index = 0;
    let mut seen = false;
    while index < bytes.len() && bytes[index] != b'.' {
        assert!(
            bytes[index].is_ascii_digit(),
            "contract version must start with a number"
        );
        major = major * 10 + (bytes[index] - b'0') as u32;
        seen = true;
        index += 1;
    }
    assert!(seen, "contract version must start with a number");
    major
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Contract {
    name: &'static str,
    version: &'static str,
    major: u32,
}

impl Contract {
    pub const fn new(name: &'static str, version: &'static str) -> Self {
        assert!(is_valid_name(name), "invalid contract name");
        Self {
            name,
            version,
            major: major_of(version),
        }
    }

    pub const fn name(&self) -> &'static str {
        self.name
    }

    pub const fn version(&self) -> &'static str {
        self.version
    }

    pub const fn major(&self) -> u32 {
        self.major
    }

    pub fn requirement(&self) -> String {
        format!("{}@{}", self.name, self.major)
    }

    pub fn owns(&self, key: &str) -> bool {
        key.strip_prefix(self.name)
            .is_some_and(|rest| rest.starts_with('.'))
    }
}

impl fmt::Display for Contract {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}@{}", self.name, self.version)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum HookKind {
    Veto,
    Filter,
    Action,
}

impl HookKind {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Veto => "veto",
            Self::Filter => "filter",
            Self::Action => "action",
        }
    }
}

macro_rules! typed_key {
    ($name:ident<$($param:ident),+>) => {
        pub struct $name<$($param),+> {
            key: &'static str,
            marker: PhantomData<fn() -> ($($param,)+)>,
        }

        impl<$($param),+> $name<$($param),+> {
            pub const fn key(&self) -> &'static str {
                self.key
            }
        }

        impl<$($param),+> Clone for $name<$($param),+> {
            fn clone(&self) -> Self {
                *self
            }
        }

        impl<$($param),+> Copy for $name<$($param),+> {}

        impl<$($param),+> PartialEq for $name<$($param),+> {
            fn eq(&self, other: &Self) -> bool {
                self.key == other.key
            }
        }

        impl<$($param),+> Eq for $name<$($param),+> {}

        impl<$($param),+> fmt::Debug for $name<$($param),+> {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.debug_tuple(stringify!($name)).field(&self.key).finish()
            }
        }

        impl<$($param),+> fmt::Display for $name<$($param),+> {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(self.key)
            }
        }
    };
}

typed_key!(OperationKey<Input, Output>);
typed_key!(EventKey<Payload>);
typed_key!(ExtensionPoint<Contribution>);

impl<Input, Output> OperationKey<Input, Output> {
    pub const fn new(key: &'static str) -> Self {
        assert!(is_valid_key(key), "invalid operation key");
        Self {
            key,
            marker: PhantomData,
        }
    }
}

impl<Payload> EventKey<Payload> {
    pub const fn new(key: &'static str) -> Self {
        assert!(is_valid_key(key), "invalid event key");
        Self {
            key,
            marker: PhantomData,
        }
    }
}

impl<Contribution> ExtensionPoint<Contribution> {
    pub const fn new(key: &'static str) -> Self {
        assert!(is_valid_key(key), "invalid extension point key");
        Self {
            key,
            marker: PhantomData,
        }
    }
}

pub struct HookKey<Args> {
    key: &'static str,
    kind: HookKind,
    marker: PhantomData<fn() -> Args>,
}

impl<Args> HookKey<Args> {
    pub const fn new(key: &'static str, kind: HookKind) -> Self {
        assert!(is_valid_key(key), "invalid hook key");
        Self {
            key,
            kind,
            marker: PhantomData,
        }
    }

    pub const fn key(&self) -> &'static str {
        self.key
    }

    pub const fn kind(&self) -> HookKind {
        self.kind
    }
}

impl<Args> Clone for HookKey<Args> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<Args> Copy for HookKey<Args> {}

impl<Args> PartialEq for HookKey<Args> {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key && self.kind == other.kind
    }
}

impl<Args> Eq for HookKey<Args> {}

impl<Args> fmt::Debug for HookKey<Args> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("HookKey")
            .field(&self.key)
            .field(&self.kind)
            .finish()
    }
}

impl<Args> fmt::Display for HookKey<Args> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.key)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ReferenceKind {
    key: &'static str,
}

impl ReferenceKind {
    pub const fn new(key: &'static str) -> Self {
        assert!(is_valid_key(key), "invalid reference kind");
        Self { key }
    }

    pub const fn key(&self) -> &'static str {
        self.key
    }
}

impl fmt::Display for ReferenceKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.key)
    }
}
