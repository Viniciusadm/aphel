use aphel_sdk::approvals::{
    ApprovalMoment, ApprovalMomentDefinition, ApprovalMomentSource, MomentTarget,
};

use crate::service::QUOTE_RECORD_TYPE;

fn built_in() -> Vec<ApprovalMomentDefinition> {
    vec![
        ApprovalMomentDefinition {
            moment: ApprovalMoment::STAGE_CHANGE,
            label: "Stage change".to_owned(),
            target: MomentTarget::Policy,
            record_types: None,
        },
        ApprovalMomentDefinition {
            moment: ApprovalMoment::QUOTE_SEND,
            label: "Quote send".to_owned(),
            target: MomentTarget::None,
            record_types: Some(vec![QUOTE_RECORD_TYPE.to_owned()]),
        },
        ApprovalMomentDefinition {
            moment: ApprovalMoment::ON_DEMAND,
            label: "On demand".to_owned(),
            target: MomentTarget::None,
            record_types: None,
        },
    ]
}

pub(crate) fn all() -> Vec<ApprovalMomentDefinition> {
    let mut moments = built_in();
    if let Some(app) = aphel_sdk::platform::modules::try_app() {
        for source in app.contributions::<dyn ApprovalMomentSource>() {
            for definition in source.moments() {
                if !moments
                    .iter()
                    .any(|known| known.moment == definition.moment)
                {
                    moments.push(definition);
                }
            }
        }
    }
    moments
}

pub(crate) fn definition(moment: &ApprovalMoment) -> Option<ApprovalMomentDefinition> {
    all()
        .into_iter()
        .find(|definition| &definition.moment == moment)
}

pub(crate) fn target(moment: &ApprovalMoment) -> MomentTarget {
    definition(moment).map_or(MomentTarget::None, |definition| definition.target)
}
