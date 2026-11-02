use aphel_kernel::application::config_packages::ConfigReferenceKind;

pub const PIPELINE_ITEM: ConfigReferenceKind = ConfigReferenceKind::new("workflow.pipeline");
pub const PIPELINE_STAGE_ITEM: ConfigReferenceKind = ConfigReferenceKind::new("workflow.stage");
pub const LOST_REASON_ITEM: ConfigReferenceKind = ConfigReferenceKind::new("workflow.lost_reason");
pub const TAG_ITEM: ConfigReferenceKind = ConfigReferenceKind::new("tagging.tag");
