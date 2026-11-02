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
    const CONTACT: RecordType = RecordType::from_static("contacts.contact");
    const COMPANY: RecordType = RecordType::from_static("companies.company");
    const DEAL: RecordType = RecordType::from_static("deals.deal");
    const ACTIVITY: RecordType = RecordType::from_static("activities.activity");
    const PRODUCT: RecordType = RecordType::from_static("products.product");
    const NOTE: RecordType = RecordType::from_static("notes.note");
    const PIPELINE: RecordType = RecordType::from_static("pipelines.pipeline");
    const PIPELINE_STAGE: RecordType = RecordType::from_static("pipelines.pipeline_stage");
    const LOST_REASON: RecordType = RecordType::from_static("pipelines.lost_reason");
    const ACTIVITY_TYPE: RecordType = RecordType::from_static("activities.activity_type");
}
