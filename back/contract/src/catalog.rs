use std::collections::BTreeMap;
use std::fmt;

use crate::key::{Contract, HookKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum KeyKind {
    Operation,
    Event,
    Hook(HookKind),
    ExtensionPoint,
    Reference,
}

impl KeyKind {
    pub const fn label(&self) -> &'static str {
        match self {
            Self::Operation => "operation",
            Self::Event => "event",
            Self::Hook(HookKind::Veto) => "veto hook",
            Self::Hook(HookKind::Filter) => "filter hook",
            Self::Hook(HookKind::Action) => "action hook",
            Self::ExtensionPoint => "extension point",
            Self::Reference => "reference kind",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyEntry {
    pub key: &'static str,
    pub kind: KeyKind,
    pub description: &'static str,
}

impl KeyEntry {
    pub const fn new(key: &'static str, kind: KeyKind, description: &'static str) -> Self {
        Self {
            key,
            kind,
            description,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ContractDescriptor {
    pub contract: Contract,
    pub entries: &'static [KeyEntry],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CatalogError {
    DuplicateContract {
        name: String,
    },
    ForeignKey {
        contract: String,
        key: String,
    },
    DuplicateKey {
        key: String,
        first: String,
        second: String,
    },
}

impl fmt::Display for CatalogError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateContract { name } => {
                write!(formatter, "contract `{name}` is registered twice")
            }
            Self::ForeignKey { contract, key } => {
                write!(
                    formatter,
                    "key `{key}` does not belong to contract `{contract}`"
                )
            }
            Self::DuplicateKey { key, first, second } => write!(
                formatter,
                "key `{key}` is declared by `{first}` and `{second}`"
            ),
        }
    }
}

impl std::error::Error for CatalogError {}

#[derive(Debug, Default)]
pub struct Catalog {
    contracts: BTreeMap<&'static str, ContractDescriptor>,
    owners: BTreeMap<&'static str, &'static str>,
}

impl Catalog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, descriptor: ContractDescriptor) -> Result<(), CatalogError> {
        let name = descriptor.contract.name();
        if self.contracts.contains_key(name) {
            return Err(CatalogError::DuplicateContract {
                name: name.to_string(),
            });
        }
        for entry in descriptor.entries {
            if !descriptor.contract.owns(entry.key) {
                return Err(CatalogError::ForeignKey {
                    contract: name.to_string(),
                    key: entry.key.to_string(),
                });
            }
            if let Some(first) = self.owners.get(entry.key) {
                return Err(CatalogError::DuplicateKey {
                    key: entry.key.to_string(),
                    first: first.to_string(),
                    second: name.to_string(),
                });
            }
        }
        for entry in descriptor.entries {
            self.owners.insert(entry.key, name);
        }
        self.contracts.insert(name, descriptor);
        Ok(())
    }

    pub fn contract(&self, name: &str) -> Option<&ContractDescriptor> {
        self.contracts.get(name)
    }

    pub fn contracts(&self) -> impl Iterator<Item = &ContractDescriptor> {
        self.contracts.values()
    }

    pub fn find(&self, key: &str) -> Option<(&Contract, &KeyEntry)> {
        let owner = self.owners.get(key)?;
        let descriptor = self.contracts.get(owner)?;
        let entry = descriptor.entries.iter().find(|entry| entry.key == key)?;
        Some((&descriptor.contract, entry))
    }

    pub fn entries_of(&self, kind: KeyKind) -> Vec<(&Contract, &KeyEntry)> {
        self.contracts
            .values()
            .flat_map(|descriptor| {
                descriptor
                    .entries
                    .iter()
                    .filter(move |entry| entry.kind == kind)
                    .map(move |entry| (&descriptor.contract, entry))
            })
            .collect()
    }

    pub fn provides(&self, requirement: &str) -> bool {
        let Some((name, major)) = requirement.split_once('@') else {
            return false;
        };
        self.contracts
            .get(name)
            .is_some_and(|descriptor| descriptor.contract.major().to_string() == major)
    }
}
