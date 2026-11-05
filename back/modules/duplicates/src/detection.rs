use std::collections::{BTreeSet, VecDeque};

use uuid::Uuid;

use crate::domain::{
    DuplicateFamily, DuplicateRule, normalize_company_name, normalize_name,
    normalized_names_similar, ordered_pair,
};
use crate::{DuplicateType, RecordFingerprint, RuleSetting, SCAN_WINDOW};
use aphel_sdk::platform::application::fields::LabelNormalization;

pub(crate) fn enabled_rules(kind: &DuplicateType, settings: &[RuleSetting]) -> Vec<DuplicateRule> {
    kind.rules()
        .into_iter()
        .filter(|rule| {
            settings
                .iter()
                .find(|setting| setting.rule == *rule)
                .is_none_or(|setting| setting.enabled)
        })
        .collect()
}

pub(crate) fn rule_settings(kind: &DuplicateType, stored: &[RuleSetting]) -> Vec<RuleSetting> {
    kind.rules()
        .into_iter()
        .map(|rule| RuleSetting {
            rule,
            enabled: stored
                .iter()
                .find(|setting| setting.rule == rule)
                .is_none_or(|setting| setting.enabled),
        })
        .collect()
}

fn normalized_email(value: &Option<String>) -> Option<String> {
    value
        .as_deref()
        .map(|email| email.trim().to_lowercase())
        .filter(|email| !email.is_empty())
}

fn normalized_domain(value: &Option<String>) -> Option<String> {
    value
        .as_deref()
        .map(|domain| domain.trim().trim_start_matches("www.").to_lowercase())
        .filter(|domain| !domain.is_empty())
}

fn phones(fingerprint: &RecordFingerprint) -> BTreeSet<String> {
    fingerprint
        .phones
        .iter()
        .map(|phone| {
            phone
                .chars()
                .filter(|character| character.is_ascii_digit())
                .collect::<String>()
        })
        .filter(|phone| phone.len() >= 8)
        .collect()
}

fn comparable_name(family: DuplicateFamily, name: &str) -> String {
    match family {
        DuplicateFamily::Company => normalize_company_name(name),
        _ => normalize_name(name),
    }
}

pub(crate) fn matched_rules(
    kind: &DuplicateType,
    left: &RecordFingerprint,
    right: &RecordFingerprint,
    rules: &[DuplicateRule],
) -> Vec<DuplicateRule> {
    let supported = kind.rules();
    rules
        .iter()
        .copied()
        .filter(|rule| supported.contains(rule))
        .filter(|rule| match rule {
            DuplicateRule::Email => normalized_email(&left.email)
                .is_some_and(|email| normalized_email(&right.email) == Some(email)),
            DuplicateRule::Phone => !phones(left).is_disjoint(&phones(right)),
            DuplicateRule::NameAndCompany => {
                left.company_id.is_some()
                    && left.company_id == right.company_id
                    && normalized_names_similar(
                        &comparable_name(kind.family, &left.name),
                        &comparable_name(kind.family, &right.name),
                    )
            }
            DuplicateRule::Domain => normalized_domain(&left.domain)
                .is_some_and(|domain| normalized_domain(&right.domain) == Some(domain)),
            DuplicateRule::Name => normalized_names_similar(
                &comparable_name(kind.family, &left.name),
                &comparable_name(kind.family, &right.name),
            ),
            DuplicateRule::Identifier => left
                .identifiers
                .iter()
                .any(|identifier| right.identifiers.contains(identifier)),
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ScanKey {
    Columns(Vec<String>),
    Lowercase(Vec<String>),
    CompanyAndName,
    Title,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ScanPass {
    pub(crate) key: ScanKey,
    pub(crate) anchor: bool,
}

impl ScanPass {
    pub(crate) fn group<'a>(&self, key: &'a [u8]) -> &'a [u8] {
        match self.key {
            ScanKey::CompanyAndName => key.split(|byte| *byte == b'|').next().unwrap_or(key),
            ScanKey::Title => &[],
            _ => key,
        }
    }
}

pub(crate) fn scan_passes(kind: &DuplicateType, rules: &[DuplicateRule]) -> Vec<ScanPass> {
    let mut passes = Vec::new();
    let exact = |key: ScanKey| ScanPass { key, anchor: true };
    for rule in rules {
        match rule {
            DuplicateRule::Email => passes.push(exact(ScanKey::Lowercase(vec!["email".into()]))),
            DuplicateRule::Phone => {
                let columns = match kind.family {
                    DuplicateFamily::Contact => vec!["phone".into(), "mobile".into()],
                    _ => vec!["phone".into()],
                };
                passes.push(exact(ScanKey::Columns(columns)));
            }
            DuplicateRule::Domain => passes.push(exact(ScanKey::Lowercase(vec!["domain".into()]))),
            DuplicateRule::NameAndCompany => passes.push(exact(ScanKey::CompanyAndName)),
            DuplicateRule::Name => passes.push(ScanPass {
                key: ScanKey::Title,
                anchor: false,
            }),
            DuplicateRule::Identifier => {
                for alias in kind.identifiers() {
                    let column = vec![alias.column.clone()];
                    passes.push(exact(match alias.normalization {
                        LabelNormalization::Exact => ScanKey::Columns(column),
                        _ => ScanKey::Lowercase(column),
                    }));
                }
            }
        }
    }
    passes
}

pub(crate) struct PairWindow {
    group: Option<Vec<u8>>,
    anchor: Option<RecordFingerprint>,
    recent: VecDeque<RecordFingerprint>,
    use_anchor: bool,
}

impl PairWindow {
    pub(crate) fn new(pass: &ScanPass) -> Self {
        Self {
            group: None,
            anchor: None,
            recent: VecDeque::with_capacity(SCAN_WINDOW),
            use_anchor: pass.anchor,
        }
    }

    pub(crate) fn push(
        &mut self,
        kind: &DuplicateType,
        rules: &[DuplicateRule],
        group: &[u8],
        fingerprint: RecordFingerprint,
    ) -> Vec<((Uuid, Uuid), Vec<DuplicateRule>)> {
        if self.group.as_deref() != Some(group) {
            self.group = Some(group.to_vec());
            self.anchor = None;
            self.recent.clear();
        }
        let mut found = Vec::new();
        let mut compared = BTreeSet::new();
        let others = self.anchor.iter().chain(self.recent.iter());
        for other in others {
            if other.id == fingerprint.id || !compared.insert(other.id) {
                continue;
            }
            let matched = matched_rules(kind, other, &fingerprint, rules);
            if !matched.is_empty() {
                found.push((ordered_pair(other.id, fingerprint.id), matched));
            }
        }
        if self.use_anchor && self.anchor.is_none() {
            self.anchor = Some(fingerprint);
            return found;
        }
        if self.recent.len() == SCAN_WINDOW {
            self.recent.pop_front();
        }
        self.recent.push_back(fingerprint);
        found
    }
}
