use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::application::organization_profile::experiences::check_sections;
use crate::application::organization_profile::{
    ExperienceProfile, ExperienceSection, OrganizationExperiences,
};
use crate::errors::FieldViolation;

use super::{NavigationPreferences, UpdateUserPreferencesCommand, UserPreferences};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct RecordDetailPreferences {
    pub record_type: String,
    pub order: Vec<ExperienceSection>,
    pub hidden: Vec<ExperienceSection>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ExperiencePreferences {
    pub profile: Option<String>,
    pub record_detail: Option<RecordDetailPreferences>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct UpdateExperiencePreferences {
    pub profile: Option<Option<String>>,
    pub record_detail: Option<Option<RecordDetailPreferences>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
pub struct ExperiencePreferenceDiagnostic {
    pub path: String,
    pub code: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserPreferencesView {
    pub effective: UserPreferences,
    pub stored_experience: ExperiencePreferences,
    pub stored_navigation: NavigationPreferences,
    pub experience_diagnostics: Vec<ExperiencePreferenceDiagnostic>,
    pub experience_omittable_sections: Vec<ExperienceSection>,
}

fn allowed_profile<'a>(
    config: &'a OrganizationExperiences,
    key: &str,
) -> Option<&'a ExperienceProfile> {
    config
        .profiles
        .iter()
        .find(|entry| {
            entry.enabled
                && entry.profile.key == key
                && config
                    .policy
                    .allowed_profiles
                    .iter()
                    .any(|allowed| allowed == key)
        })
        .map(|entry| &entry.profile)
}

fn selected_profile<'a>(
    config: &'a OrganizationExperiences,
    preferences: &ExperiencePreferences,
) -> Option<&'a ExperienceProfile> {
    preferences
        .profile
        .as_deref()
        .filter(|_| config.policy.customization.profile_selection)
        .and_then(|key| allowed_profile(config, key))
        .or_else(|| {
            config
                .default_profile
                .as_deref()
                .and_then(|key| allowed_profile(config, key))
        })
}

pub fn validate_personal_experience_update(
    command: &UpdateUserPreferencesCommand,
    next: &UserPreferences,
    config: Option<&OrganizationExperiences>,
    omittable: &[ExperienceSection],
) -> Vec<FieldViolation> {
    let mut errors = Vec::new();
    if command.navigation.is_some()
        && config.is_some_and(|config| !config.policy.customization.navigation)
        && next.navigation != NavigationPreferences::default()
    {
        errors.push(FieldViolation::new(
            "navigation",
            "personal navigation is blocked by organization policy; only a reset is allowed",
        ));
    }
    let Some(Some(change)) = &command.experience else {
        return errors;
    };
    if let Some(Some(key)) = &change.profile
        && !config.is_some_and(|config| {
            config.policy.customization.profile_selection && allowed_profile(config, key).is_some()
        })
    {
        errors.push(FieldViolation::new(
            "experience.profile",
            "must be an enabled, allowed profile and personal selection must be permitted",
        ));
    }
    let Some(Some(detail)) = &change.record_detail else {
        return errors;
    };
    check_sections(&detail.order, "experience.record_detail.order", &mut errors);
    check_sections(
        &detail.hidden,
        "experience.record_detail.hidden",
        &mut errors,
    );
    let Some((config, profile)) = config.and_then(|config| {
        selected_profile(config, &next.experience).map(|profile| (config, profile))
    }) else {
        errors.push(FieldViolation::new(
            "experience.record_detail",
            "requires an allowed selected or default profile",
        ));
        return errors;
    };
    if detail.record_type != profile.record_detail.record_type {
        errors.push(FieldViolation::new(
            "experience.record_detail.record_type",
            "must match the selected profile's record type",
        ));
    }
    let main = profile
        .record_detail
        .regions
        .as_ref()
        .map(|regions| regions.main.as_slice())
        .unwrap_or_default();
    if !detail.order.is_empty() && !config.policy.customization.section_order {
        errors.push(FieldViolation::new(
            "experience.record_detail.order",
            "personal section order is blocked by organization policy",
        ));
    }
    for (index, source) in detail.order.iter().enumerate() {
        if !main.contains(source) {
            errors.push(FieldViolation::new(
                format!("experience.record_detail.order.{index}"),
                "must reference a main section of the selected profile",
            ));
        }
    }
    for (index, source) in detail.hidden.iter().enumerate() {
        if !main.contains(source)
            || !config.policy.customization.section_hidden.contains(source)
            || !omittable.contains(source)
        {
            errors.push(FieldViolation::new(format!("experience.record_detail.hidden.{index}"), "must be an organization-permitted, owner-declared optional section of the selected profile"));
        }
    }
    errors
}

pub fn personal_preferences_view(
    stored: UserPreferences,
    config: Option<&OrganizationExperiences>,
    omittable: &[ExperienceSection],
) -> UserPreferencesView {
    let mut effective = stored.clone();
    let mut diagnostics = Vec::new();
    let mut diagnostic = |path: &str, code: &str| {
        diagnostics.push(ExperiencePreferenceDiagnostic {
            path: path.to_owned(),
            code: code.to_owned(),
        })
    };
    effective.experience = ExperiencePreferences::default();
    if let Some(config) = config {
        if !config.policy.customization.navigation {
            effective.navigation = NavigationPreferences::default();
            if stored.navigation != effective.navigation {
                diagnostic("navigation", "navigation_blocked");
            }
        }
        let selected = selected_profile(config, &stored.experience);
        effective.experience.profile = selected.map(|profile| profile.key.clone());
        if let Some(key) = &stored.experience.profile {
            if !config.policy.customization.profile_selection {
                diagnostic("experience.profile", "profile_selection_blocked");
            } else if allowed_profile(config, key).is_none() {
                diagnostic("experience.profile", "profile_unavailable_or_not_allowed");
            }
        }
        if let Some(detail) = &stored.experience.record_detail {
            if let Some(profile) =
                selected.filter(|profile| profile.record_detail.record_type == detail.record_type)
            {
                let main = profile
                    .record_detail
                    .regions
                    .as_ref()
                    .map(|regions| regions.main.as_slice())
                    .unwrap_or_default();
                let unique =
                    |items: &[ExperienceSection], allow: &dyn Fn(&ExperienceSection) -> bool| {
                        let mut seen = BTreeSet::new();
                        items
                            .iter()
                            .filter(|source| allow(source) && seen.insert((*source).clone()))
                            .cloned()
                            .collect::<Vec<_>>()
                    };
                let order = unique(&detail.order, &|source| {
                    config.policy.customization.section_order && main.contains(source)
                });
                let hidden = unique(&detail.hidden, &|source| {
                    main.contains(source)
                        && config.policy.customization.section_hidden.contains(source)
                        && omittable.contains(source)
                });
                if order != detail.order {
                    diagnostic(
                        "experience.record_detail.order",
                        "section_order_blocked_or_incompatible",
                    );
                }
                if hidden != detail.hidden {
                    diagnostic(
                        "experience.record_detail.hidden",
                        "section_omission_blocked_or_incompatible",
                    );
                }
                if !order.is_empty() || !hidden.is_empty() {
                    effective.experience.record_detail = Some(RecordDetailPreferences {
                        record_type: detail.record_type.clone(),
                        order,
                        hidden,
                    });
                }
            } else {
                diagnostic(
                    "experience.record_detail",
                    "record_surface_unavailable_or_incompatible",
                );
            }
        }
    } else if stored.experience != ExperiencePreferences::default() {
        diagnostic("experience", "experience_configuration_unavailable");
    }
    let experience_omittable_sections = config
        .and_then(|config| {
            selected_profile(config, &stored.experience).map(|profile| (config, profile))
        })
        .map(|(config, profile)| {
            profile
                .record_detail
                .regions
                .as_ref()
                .map(|regions| {
                    regions
                        .main
                        .iter()
                        .filter(|source| {
                            config.policy.customization.section_hidden.contains(source)
                                && omittable.contains(source)
                        })
                        .cloned()
                        .collect()
                })
                .unwrap_or_default()
        })
        .unwrap_or_default();
    UserPreferencesView {
        experience_omittable_sections,
        effective,
        stored_experience: stored.experience,
        stored_navigation: stored.navigation,
        experience_diagnostics: diagnostics,
    }
}

pub fn probe_personal_experience_preferences(failures: &mut Vec<String>) {
    use crate::application::organization_profile::{ExperienceSection, OrganizationExperiences};
    use crate::application::user_preferences::{
        ExperiencePreferences, NavigationPreferences, RecordDetailPreferences,
        UpdateExperiencePreferences, UpdateUserPreferencesCommand, UserPreferences,
        personal_preferences_view, validate_personal_experience_update,
    };
    use serde_json::json;

    let source = |key: &str| ExperienceSection {
        module: "probe_sections".into(),
        key: key.into(),
    };
    let optional = source("optional");
    let required = source("required");
    let config: OrganizationExperiences = serde_json::from_value(json!({
        "profiles": [{ "enabled": true, "profile": {
            "format_version": 1, "key": "probe.active", "label": "Probe",
            "frame": { "module": "kernel" },
            "record_detail": { "record_type": "probe.record", "presentation": { "module": "probe_presentation" },
                "regions": { "main": [optional, required] } }
        }}], "default_profile": "probe.active",
        "policy": { "allowed_profiles": ["probe.active"], "customization": {
            "profile_selection": true, "navigation": true, "section_order": true, "section_hidden": [optional, required]
        }}
    })).expect("probe experience catalog");
    let stored = UserPreferences {
        keyboard_shortcuts: false,
        locale: Some("en".into()),
        navigation: NavigationPreferences {
            home: Some("/probe".into()),
            ..Default::default()
        },
        experience: ExperiencePreferences {
            profile: Some("probe.active".into()),
            record_detail: Some(RecordDetailPreferences {
                record_type: "probe.record".into(),
                order: vec![required.clone(), optional.clone()],
                hidden: vec![optional.clone()],
            }),
        },
    };
    let command = UpdateUserPreferencesCommand {
        navigation: Some(stored.navigation.clone()),
        experience: Some(Some(UpdateExperiencePreferences {
            profile: Some(stored.experience.profile.clone()),
            record_detail: Some(stored.experience.record_detail.clone()),
        })),
        ..Default::default()
    };
    if !validate_personal_experience_update(
        &command,
        &stored,
        Some(&config),
        std::slice::from_ref(&optional),
    )
    .is_empty()
    {
        failures
            .push("preferences: permitted profile and surface adjustments must be accepted".into());
    }
    if validate_personal_experience_update(&command, &stored, Some(&config), &[]).is_empty() {
        failures.push(
            "preferences: an organization limit must not replace owner-declared optionality".into(),
        );
    }
    if personal_preferences_view(
        stored.clone(),
        Some(&config),
        std::slice::from_ref(&optional),
    )
    .experience_omittable_sections
        != vec![optional.clone()]
        || !personal_preferences_view(stored.clone(), Some(&config), &[])
            .experience_omittable_sections
            .is_empty()
    {
        failures.push("preferences: omission choices must intersect policy, profile sources and active owner declarations".into());
    }
    let mut mandatory = stored.clone();
    mandatory.experience.record_detail.as_mut().unwrap().hidden = vec![required];
    let mut mandatory_command = command.clone();
    mandatory_command
        .experience
        .as_mut()
        .unwrap()
        .as_mut()
        .unwrap()
        .record_detail = Some(mandatory.experience.record_detail.clone());
    if validate_personal_experience_update(
        &mandatory_command,
        &mandatory,
        Some(&config),
        std::slice::from_ref(&optional),
    )
    .is_empty()
    {
        failures.push("preferences: mandatory source omission must be rejected even if the organization allows its identity".into());
    }
    let mut locked = config.clone();
    locked.policy.customization.navigation = false;
    locked.policy.customization.profile_selection = false;
    locked.policy.customization.section_order = false;
    locked.policy.customization.section_hidden.clear();
    let errors = validate_personal_experience_update(
        &command,
        &stored,
        Some(&locked),
        std::slice::from_ref(&optional),
    );
    for field in [
        "navigation",
        "experience.profile",
        "experience.record_detail.order",
        "experience.record_detail.hidden.0",
    ] {
        if !errors.iter().any(|error| error.field == field) {
            failures.push(format!(
                "preferences: a blocked {field} write must be rejected"
            ));
        }
    }
    locked.profiles[0].enabled = false;
    locked.policy.allowed_profiles.clear();
    locked.default_profile = None;
    let view = personal_preferences_view(
        stored.clone(),
        Some(&locked),
        std::slice::from_ref(&optional),
    );
    if view.effective.experience != ExperiencePreferences::default()
        || view.effective.navigation != NavigationPreferences::default()
        || view.stored_experience != stored.experience
        || view.stored_navigation != stored.navigation
        || view.effective.locale != stored.locale
        || view.effective.keyboard_shortcuts != stored.keyboard_shortcuts
        || view.experience_diagnostics.is_empty()
        || !view.experience_omittable_sections.is_empty()
    {
        failures.push("preferences: revocation must retain saved references, ignore blocked choices and preserve unrelated values".into());
    }
    let old_patch = UpdateUserPreferencesCommand {
        locale: Some(Some("en".into())),
        ..Default::default()
    };
    if !validate_personal_experience_update(&old_patch, &stored, Some(&locked), &[]).is_empty() {
        failures.push("preferences: an old language-only patch must not resubmit or reject retained blocked choices".into());
    }
    let reset = UpdateUserPreferencesCommand {
        navigation: Some(NavigationPreferences::default()),
        experience: Some(None),
        ..Default::default()
    };
    if !validate_personal_experience_update(&reset, &UserPreferences::default(), Some(&locked), &[])
        .is_empty()
    {
        failures.push(
            "preferences: explicit personal resets must remain possible under a blocked policy"
                .into(),
        );
    }
    let legacy = UserPreferences::default();
    if personal_preferences_view(legacy.clone(), None, &[]).effective != legacy {
        failures.push(
            "preferences: unconfigured installations must preserve default preference behavior"
                .into(),
        );
    }
}
