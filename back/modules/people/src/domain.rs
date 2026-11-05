use chrono::{DateTime, Utc};
use serde_json::{Map, Value};
use uuid::Uuid;

use aphel_kernel::domain::UserId;
use aphel_kernel::errors::AppError;
use aphel_kernel::records::{
    Address, AddressInput, InputValidator, MAX_LONG_TEXT_LENGTH, Patch, insert_address,
    insert_text, value_patch,
};
pub(crate) use aphel_sdk::email::EmailEngagement;
use aphel_sdk::platform::application::fields::permissions::provided_key;
use aphel_sdk::platform::application::files::UserReference;

pub(crate) const CONTACT_NOT_FOUND: &str = "contact_not_found";
pub(crate) const LINK_NOT_FOUND: &str = "link_not_found";
pub(crate) const ALREADY_LINKED: &str = "already_linked";

pub(crate) const CONTACT_CREATED: &str = "contact.created";
pub const CONTACT_UPDATED: &str = "contact.updated";
pub(crate) const CONTACT_DELETED: &str = "contact.deleted";
pub(crate) const CONTACT_ARCHIVED: &str = "contact.archived";
pub(crate) const CONTACT_UNARCHIVED: &str = "contact.unarchived";
pub(crate) const CONTACT_COMPANY_LINKED: &str = "contact.company_linked";
pub(crate) const CONTACT_COMPANY_UNLINKED: &str = "contact.company_unlinked";

pub(crate) const INVALID_CONTACT: &str = "invalid contact";

pub fn contact_not_found() -> AppError {
    AppError::not_found_code(CONTACT_NOT_FOUND, "contact was not found")
}

pub(crate) fn link_not_found() -> AppError {
    AppError::not_found_code(LINK_NOT_FOUND, "the contact is not linked to this company")
}

pub(crate) fn already_linked() -> AppError {
    AppError::conflict_code(
        ALREADY_LINKED,
        "the contact is already linked to this company",
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ContactCompanyRole {
    DecisionMaker,
    Buyer,
    Influencer,
    User,
    Other,
}

impl ContactCompanyRole {
    pub(crate) const ALL: [Self; 5] = [
        Self::DecisionMaker,
        Self::Buyer,
        Self::Influencer,
        Self::User,
        Self::Other,
    ];

    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::DecisionMaker => "decision_maker",
            Self::Buyer => "buyer",
            Self::Influencer => "influencer",
            Self::User => "user",
            Self::Other => "other",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|role| role.as_str() == value)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ContactData {
    pub first_name: String,
    pub last_name: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub mobile: Option<String>,
    pub job_title: Option<String>,
    pub company_id: Option<Uuid>,
    pub owner_id: Option<UserId>,
    pub address: Address,
    pub source: Option<String>,
    pub notes: Option<String>,
}

impl ContactData {
    pub(crate) fn full_name(&self) -> String {
        [Some(self.first_name.as_str()), self.last_name.as_deref()]
            .into_iter()
            .flatten()
            .map(str::trim)
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>()
            .join(" ")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Contact {
    pub id: Uuid,
    pub data: ContactData,
    pub(crate) engagement: EmailEngagement,
    pub(crate) created_by: Option<UserId>,
    pub(crate) archived_at: Option<DateTime<Utc>>,
    pub(crate) created_at: DateTime<Utc>,
    pub(crate) updated_at: DateTime<Utc>,
}

impl Contact {
    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    pub fn full_name(&self) -> String {
        self.data.full_name()
    }

    pub(crate) fn is_archived(&self) -> bool {
        self.archived_at.is_some()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CompanyReference {
    pub(crate) id: Uuid,
    pub(crate) name: String,
    pub(crate) domain: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ContactView {
    pub contact: Contact,
    pub(crate) owner: Option<UserReference>,
    pub(crate) company: Option<CompanyReference>,
    pub contributed: Map<String, Value>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContactCompanyLink {
    pub(crate) link_id: Option<Uuid>,
    pub(crate) company: CompanyReference,
    pub(crate) role: Option<ContactCompanyRole>,
    pub(crate) is_primary: bool,
    pub(crate) created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ContactSummary {
    pub(crate) id: Uuid,
    pub(crate) name: String,
    pub(crate) email: Option<String>,
    pub(crate) phone: Option<String>,
    pub(crate) mobile: Option<String>,
    pub(crate) job_title: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CompanyContactLink {
    pub(crate) link_id: Option<Uuid>,
    pub(crate) contact: ContactSummary,
    pub(crate) role: Option<ContactCompanyRole>,
    pub(crate) is_primary: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewContactCompanyLink {
    pub id: Uuid,
    pub contact_id: Uuid,
    pub company_id: Uuid,
    pub role: Option<ContactCompanyRole>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ContactInput {
    pub first_name: Patch<String>,
    pub last_name: Patch<String>,
    pub email: Patch<String>,
    pub phone: Patch<String>,
    pub mobile: Patch<String>,
    pub job_title: Patch<String>,
    pub company_id: Patch<String>,
    pub owner_id: Patch<String>,
    pub address: AddressInput,
    pub source: Patch<String>,
    pub notes: Patch<String>,
    pub contributed: Map<String, Value>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct SaveContactCommand {
    pub input: ContactInput,
    pub custom_fields: Map<String, Value>,
    pub ip: Option<String>,
    pub expected_updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ContactDetails {
    pub view: ContactView,
    pub(crate) custom_fields: Map<String, Value>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct LinkCompanyCommand {
    pub(crate) company_id: String,
    pub(crate) role: Option<ContactCompanyRole>,
    pub(crate) ip: Option<String>,
}

impl ContactInput {
    pub fn from_values(values: &Map<String, Value>) -> Self {
        Self {
            first_name: value_patch(values, "first_name"),
            last_name: value_patch(values, "last_name"),
            email: value_patch(values, "email"),
            phone: value_patch(values, "phone"),
            mobile: value_patch(values, "mobile"),
            job_title: value_patch(values, "job_title"),
            company_id: value_patch(values, "company_id"),
            owner_id: value_patch(values, "owner_id"),
            address: AddressInput::from_values(values),
            source: value_patch(values, "source"),
            notes: value_patch(values, "notes"),
            contributed: Map::new(),
        }
    }

    pub fn contributed(mut self, key: &str, value: Value) -> Self {
        self.contributed.insert(key.to_owned(), value);
        self
    }

    pub(crate) fn provided_keys(&self) -> Vec<&str> {
        let mut keys = Vec::new();
        provided_key(&mut keys, "first_name", self.first_name.is_some());
        provided_key(&mut keys, "last_name", self.last_name.is_some());
        provided_key(&mut keys, "email", self.email.is_some());
        provided_key(&mut keys, "phone", self.phone.is_some());
        provided_key(&mut keys, "mobile", self.mobile.is_some());
        provided_key(&mut keys, "job_title", self.job_title.is_some());
        provided_key(&mut keys, "company_id", self.company_id.is_some());
        provided_key(&mut keys, "owner_id", self.owner_id.is_some());
        self.address.provided_keys(&mut keys);
        provided_key(&mut keys, "source", self.source.is_some());
        provided_key(&mut keys, "notes", self.notes.is_some());
        let mut keys: Vec<&str> = keys;
        keys.extend(self.contributed.keys().map(String::as_str));
        keys
    }
}

pub fn contact_snapshot(data: &ContactData) -> Value {
    let mut map = Map::new();
    insert_text(&mut map, "first_name", Some(&data.first_name));
    insert_text(&mut map, "last_name", data.last_name.as_deref());
    insert_text(&mut map, "email", data.email.as_deref());
    insert_text(&mut map, "phone", data.phone.as_deref());
    insert_text(&mut map, "mobile", data.mobile.as_deref());
    insert_text(&mut map, "job_title", data.job_title.as_deref());
    insert_text(
        &mut map,
        "company_id",
        data.company_id.map(|id| id.to_string()).as_deref(),
    );
    insert_text(
        &mut map,
        "owner_id",
        data.owner_id.as_ref().map(UserId::as_str),
    );
    insert_address(&mut map, &data.address);
    insert_text(&mut map, "source", data.source.as_deref());
    insert_text(&mut map, "notes", data.notes.as_deref());
    Value::Object(map)
}

pub fn apply_contact_input(
    base: &ContactData,
    input: ContactInput,
    validator: &mut InputValidator,
) -> ContactData {
    let mut data = base.clone();
    if let Some(value) = input.first_name {
        data.first_name = validator.required_text("first_name", value, 140);
    }
    if let Some(value) = input.last_name {
        data.last_name = validator.text("last_name", value, 140);
    }
    if let Some(value) = input.email {
        data.email = validator.email("email", value);
    }
    if let Some(value) = input.phone {
        data.phone = validator.phone("phone", value);
    }
    if let Some(value) = input.mobile {
        data.mobile = validator.phone("mobile", value);
    }
    if let Some(value) = input.job_title {
        data.job_title = validator.text("job_title", value, 140);
    }
    if let Some(value) = input.company_id {
        data.company_id = validator.record_id("company_id", value);
    }
    if let Some(value) = input.owner_id {
        data.owner_id = validator.user("owner_id", value);
    }
    validator.address(&mut data.address, input.address);
    if let Some(value) = input.source {
        data.source = validator.text("source", value, 140);
    }
    if let Some(value) = input.notes {
        data.notes = validator.text("notes", value, MAX_LONG_TEXT_LENGTH);
    }
    data
}
