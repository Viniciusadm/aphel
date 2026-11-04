use aphel_kernel::domain::access::Permission;

pub trait CrmPermissions {
    const CONTACTS_READ: Permission;
    const CONTACTS_CREATE: Permission;
    const CONTACTS_UPDATE: Permission;
    const CONTACTS_DELETE: Permission;
    const CONTACTS_EXPORT: Permission;
    const CONTACTS_ARCHIVE: Permission;
    const COMPANIES_READ: Permission;
    const COMPANIES_CREATE: Permission;
    const COMPANIES_UPDATE: Permission;
    const COMPANIES_DELETE: Permission;
    const COMPANIES_EXPORT: Permission;
    const COMPANIES_ARCHIVE: Permission;
    const DEALS_READ: Permission;
    const DEALS_CREATE: Permission;
    const DEALS_UPDATE: Permission;
    const DEALS_DELETE: Permission;
    const DEALS_EXPORT: Permission;
    const DEALS_ARCHIVE: Permission;
    const TASKS_READ: Permission;
    const TASKS_CREATE: Permission;
    const TASKS_UPDATE: Permission;
    const TASKS_DELETE: Permission;
    const TASKS_EXPORT: Permission;
    const NOTES_READ: Permission;
    const NOTES_CREATE: Permission;
    const NOTES_UPDATE: Permission;
    const NOTES_DELETE: Permission;
    const EMAILS_READ: Permission;
    const EMAILS_SEND: Permission;
    const SEQUENCES_READ: Permission;
    const SEQUENCES_ENROLL: Permission;
    const BOOKING_MANAGE: Permission;
    const GOALS_MANAGE: Permission;
    const PRODUCTS_READ: Permission;
    const PRODUCTS_CREATE: Permission;
    const PRODUCTS_UPDATE: Permission;
    const PRODUCTS_DELETE: Permission;
    const QUOTES_READ: Permission;
    const QUOTES_CREATE: Permission;
    const QUOTES_UPDATE: Permission;
    const QUOTES_DELETE: Permission;
    const QUOTES_SEND: Permission;
    const AUTOMATIONS_MANAGE: Permission;
    const SEQUENCES_MANAGE: Permission;
    const APPROVALS_MANAGE: Permission;
    const LIFECYCLE_MANAGE: Permission;
    const SCORING_MANAGE: Permission;
    const ORGANIZATION_EXPORT: Permission;
}

impl CrmPermissions for Permission {
    const CONTACTS_READ: Permission = Permission::from_static(aphel_contract_people::READ);
    const CONTACTS_CREATE: Permission = Permission::from_static(aphel_contract_people::CREATE);
    const CONTACTS_UPDATE: Permission = Permission::from_static(aphel_contract_people::UPDATE);
    const CONTACTS_DELETE: Permission = Permission::from_static(aphel_contract_people::DELETE);
    const CONTACTS_EXPORT: Permission = Permission::from_static(aphel_contract_people::EXPORT);
    const CONTACTS_ARCHIVE: Permission = Permission::from_static(aphel_contract_people::ARCHIVE);
    const COMPANIES_READ: Permission = Permission::from_static(aphel_contract_organizations::READ);
    const COMPANIES_CREATE: Permission =
        Permission::from_static(aphel_contract_organizations::CREATE);
    const COMPANIES_UPDATE: Permission =
        Permission::from_static(aphel_contract_organizations::UPDATE);
    const COMPANIES_DELETE: Permission =
        Permission::from_static(aphel_contract_organizations::DELETE);
    const COMPANIES_EXPORT: Permission =
        Permission::from_static(aphel_contract_organizations::EXPORT);
    const COMPANIES_ARCHIVE: Permission =
        Permission::from_static(aphel_contract_organizations::ARCHIVE);
    const DEALS_READ: Permission = Permission::from_static("deals.read");
    const DEALS_CREATE: Permission = Permission::from_static("deals.create");
    const DEALS_UPDATE: Permission = Permission::from_static("deals.update");
    const DEALS_DELETE: Permission = Permission::from_static("deals.delete");
    const DEALS_EXPORT: Permission = Permission::from_static("deals.export");
    const DEALS_ARCHIVE: Permission = Permission::from_static("deals.archive");
    const TASKS_READ: Permission = Permission::from_static("tasks.read");
    const TASKS_CREATE: Permission = Permission::from_static("tasks.create");
    const TASKS_UPDATE: Permission = Permission::from_static("tasks.update");
    const TASKS_DELETE: Permission = Permission::from_static("tasks.delete");
    const TASKS_EXPORT: Permission = Permission::from_static("tasks.export");
    const NOTES_READ: Permission = Permission::from_static("notes.read");
    const NOTES_CREATE: Permission = Permission::from_static("notes.create");
    const NOTES_UPDATE: Permission = Permission::from_static("notes.update");
    const NOTES_DELETE: Permission = Permission::from_static("notes.delete");
    const EMAILS_READ: Permission = Permission::from_static("emails.read");
    const EMAILS_SEND: Permission = Permission::from_static("emails.send");
    const SEQUENCES_READ: Permission = Permission::from_static("sequences.read");
    const SEQUENCES_ENROLL: Permission = Permission::from_static("sequences.enroll");
    const BOOKING_MANAGE: Permission = Permission::from_static("booking.manage");
    const GOALS_MANAGE: Permission = Permission::from_static("goals.manage");
    const PRODUCTS_READ: Permission = Permission::from_static(aphel_contract_catalog_items::READ);
    const PRODUCTS_CREATE: Permission =
        Permission::from_static(aphel_contract_catalog_items::CREATE);
    const PRODUCTS_UPDATE: Permission =
        Permission::from_static(aphel_contract_catalog_items::UPDATE);
    const PRODUCTS_DELETE: Permission =
        Permission::from_static(aphel_contract_catalog_items::DELETE);
    const QUOTES_READ: Permission = Permission::from_static("quotes.read");
    const QUOTES_CREATE: Permission = Permission::from_static("quotes.create");
    const QUOTES_UPDATE: Permission = Permission::from_static("quotes.update");
    const QUOTES_DELETE: Permission = Permission::from_static("quotes.delete");
    const QUOTES_SEND: Permission = Permission::from_static("quotes.send");
    const AUTOMATIONS_MANAGE: Permission = Permission::from_static("automations.manage");
    const SEQUENCES_MANAGE: Permission = Permission::from_static("sequences.manage");
    const APPROVALS_MANAGE: Permission = Permission::from_static("approvals.manage");
    const LIFECYCLE_MANAGE: Permission = Permission::from_static("lifecycle.manage");
    const SCORING_MANAGE: Permission = Permission::from_static("scoring.manage");
    const ORGANIZATION_EXPORT: Permission = Permission::from_static("organization.export");
}
