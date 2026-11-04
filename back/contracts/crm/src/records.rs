use aphel_kernel::application::shared::RecordType;

pub trait CrmRecordTypes {
    const CONTACT: RecordType;
    const COMPANY: RecordType;
    const DEAL: RecordType;
    const ACTIVITY: RecordType;
    const PRODUCT: RecordType;
    const NOTE: RecordType;
    const ACTIVITY_TYPE: RecordType;
}

impl CrmRecordTypes for RecordType {
    const CONTACT: RecordType = RecordType::from_static(aphel_contract_people::RECORD_TYPE);
    const COMPANY: RecordType = RecordType::from_static(aphel_contract_organizations::RECORD_TYPE);
    const DEAL: RecordType = RecordType::from_static(aphel_contract_deals::RECORD_TYPE);
    const ACTIVITY: RecordType = RecordType::from_static("tasks.task");
    const PRODUCT: RecordType = RecordType::from_static(aphel_contract_catalog_items::RECORD_TYPE);
    const NOTE: RecordType = RecordType::from_static("notes.note");
    const ACTIVITY_TYPE: RecordType = RecordType::from_static("tasks.type");
}
