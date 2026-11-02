use aphel_kernel::application::shared::RecordType;

pub trait CrmRecordTypes {
    const CONTACT: RecordType;
    const COMPANY: RecordType;
    const DEAL: RecordType;
    const ACTIVITY: RecordType;
    const PRODUCT: RecordType;
    const NOTE: RecordType;
    const PIPELINE: RecordType;
    const PIPELINE_STAGE: RecordType;
    const LOST_REASON: RecordType;
    const ACTIVITY_TYPE: RecordType;
}

impl CrmRecordTypes for RecordType {
    const CONTACT: RecordType = RecordType::from_static("contact");
    const COMPANY: RecordType = RecordType::from_static("company");
    const DEAL: RecordType = RecordType::from_static("deal");
    const ACTIVITY: RecordType = RecordType::from_static("activity");
    const PRODUCT: RecordType = RecordType::from_static("product");
    const NOTE: RecordType = RecordType::from_static("note");
    const PIPELINE: RecordType = RecordType::from_static("pipeline");
    const PIPELINE_STAGE: RecordType = RecordType::from_static("pipeline_stage");
    const LOST_REASON: RecordType = RecordType::from_static("lost_reason");
    const ACTIVITY_TYPE: RecordType = RecordType::from_static("activity_type");
}
