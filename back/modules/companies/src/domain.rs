use chrono::{DateTime, Utc};
use serde_json::{Map, Value};
use uuid::Uuid;

use aphel_kernel::application::fields::permissions::provided_key;
use aphel_kernel::application::files::UserReference;
use aphel_kernel::domain::UserId;
use aphel_kernel::errors::AppError;
use aphel_kernel::records::{
    Address, AddressInput, InputValidator, MAX_LONG_TEXT_LENGTH, Patch, insert_address,
    insert_text, normalize_domain, value_patch,
};

pub(crate) const COMPANY_NOT_FOUND: &str = "company_not_found";

pub(crate) const COMPANY_CREATED: &str = "company.created";
pub(crate) const COMPANY_UPDATED: &str = "company.updated";
pub(crate) const COMPANY_DELETED: &str = "company.deleted";
pub(crate) const COMPANY_ARCHIVED: &str = "company.archived";
pub(crate) const COMPANY_UNARCHIVED: &str = "company.unarchived";
pub(crate) const COMPANY_RESTORED: &str = "company.restored";

pub(crate) const INVALID_COMPANY: &str = "invalid company";

pub(crate) const COMPANY_SIZE_RANGES: &[&str] = &[
    "size_1_10",
    "size_11_50",
    "size_51_200",
    "size_201_500",
    "size_501_1000",
    "size_1001_plus",
];

pub(crate) fn is_company_size_range(value: &str) -> bool {
    COMPANY_SIZE_RANGES.contains(&value)
}

pub fn domain_from_website(website: &str) -> Option<String> {
    normalize_domain(website)
}

pub fn company_not_found() -> AppError {
    AppError::not_found_code(COMPANY_NOT_FOUND, "company was not found")
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CompanyData {
    pub name: String,
    pub domain: Option<String>,
    pub website: Option<String>,
    pub industry: Option<String>,
    pub size_range: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub owner_id: Option<UserId>,
    pub address: Address,
    pub description: Option<String>,
    pub source: Option<String>,
    pub parent_company_id: Option<Uuid>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EmailEngagement {
    pub last_contacted_at: Option<DateTime<Utc>>,
    pub last_email_received_at: Option<DateTime<Utc>>,
    pub last_email_sent_at: Option<DateTime<Utc>>,
    pub interaction_count: i64,
    pub last_contacted_by: Option<UserId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Company {
    pub id: Uuid,
    pub data: CompanyData,
    pub(crate) engagement: EmailEngagement,
    pub(crate) created_by: Option<UserId>,
    pub(crate) archived_at: Option<DateTime<Utc>>,
    pub(crate) created_at: DateTime<Utc>,
    pub(crate) updated_at: DateTime<Utc>,
}

impl Company {
    pub(crate) fn is_archived(&self) -> bool {
        self.archived_at.is_some()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompanyView {
    pub company: Company,
    pub(crate) owner: Option<UserReference>,
    pub(crate) contributed: Map<String, Value>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct CompanyInput {
    pub name: Patch<String>,
    pub domain: Patch<String>,
    pub website: Patch<String>,
    pub industry: Patch<String>,
    pub size_range: Patch<String>,
    pub phone: Patch<String>,
    pub email: Patch<String>,
    pub owner_id: Patch<String>,
    pub address: AddressInput,
    pub description: Patch<String>,
    pub source: Patch<String>,
    pub parent_company_id: Patch<String>,
    pub contributed: Map<String, Value>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct SaveCompanyCommand {
    pub input: CompanyInput,
    pub custom_fields: Map<String, Value>,
    pub ip: Option<String>,
    pub expected_updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompanyDetails {
    pub view: CompanyView,
    pub(crate) custom_fields: Map<String, Value>,
}

impl CompanyInput {
    pub(crate) fn from_values(values: &Map<String, Value>) -> Self {
        Self {
            name: value_patch(values, "name"),
            domain: value_patch(values, "domain"),
            website: value_patch(values, "website"),
            industry: value_patch(values, "industry"),
            size_range: value_patch(values, "size_range"),
            phone: value_patch(values, "phone"),
            email: value_patch(values, "email"),
            owner_id: value_patch(values, "owner_id"),
            address: AddressInput::from_values(values),
            description: value_patch(values, "description"),
            source: value_patch(values, "source"),
            parent_company_id: value_patch(values, "parent_company_id"),
            contributed: Map::new(),
        }
    }

    pub(crate) fn provided_keys(&self) -> Vec<&str> {
        let mut keys = Vec::new();
        provided_key(&mut keys, "name", self.name.is_some());
        provided_key(&mut keys, "domain", self.domain.is_some());
        provided_key(&mut keys, "website", self.website.is_some());
        provided_key(&mut keys, "industry", self.industry.is_some());
        provided_key(&mut keys, "size_range", self.size_range.is_some());
        provided_key(&mut keys, "phone", self.phone.is_some());
        provided_key(&mut keys, "email", self.email.is_some());
        provided_key(&mut keys, "owner_id", self.owner_id.is_some());
        self.address.provided_keys(&mut keys);
        provided_key(&mut keys, "description", self.description.is_some());
        provided_key(&mut keys, "source", self.source.is_some());
        provided_key(
            &mut keys,
            "parent_company_id",
            self.parent_company_id.is_some(),
        );
        let mut keys: Vec<&str> = keys;
        keys.extend(self.contributed.keys().map(String::as_str));
        keys
    }
}

pub fn company_snapshot(data: &CompanyData) -> Value {
    let mut map = Map::new();
    insert_text(&mut map, "name", Some(&data.name));
    insert_text(&mut map, "domain", data.domain.as_deref());
    insert_text(&mut map, "website", data.website.as_deref());
    insert_text(&mut map, "industry", data.industry.as_deref());
    insert_text(&mut map, "size_range", data.size_range.as_deref());
    insert_text(&mut map, "phone", data.phone.as_deref());
    insert_text(&mut map, "email", data.email.as_deref());
    insert_text(
        &mut map,
        "owner_id",
        data.owner_id.as_ref().map(UserId::as_str),
    );
    insert_address(&mut map, &data.address);
    insert_text(&mut map, "description", data.description.as_deref());
    insert_text(&mut map, "source", data.source.as_deref());
    insert_text(
        &mut map,
        "parent_company_id",
        data.parent_company_id.map(|id| id.to_string()).as_deref(),
    );
    Value::Object(map)
}

pub(crate) fn apply_company_input(
    base: &CompanyData,
    input: CompanyInput,
    validator: &mut InputValidator,
) -> CompanyData {
    let mut data = base.clone();
    if let Some(value) = input.name {
        data.name = validator.required_text("name", value, 255);
    }
    if let Some(value) = input.website.clone() {
        data.website = validator.url("website", value);
    }
    match input.domain {
        Some(value) => data.domain = validator.domain("domain", value),
        None if data.domain.is_none() && input.website.is_some() => {
            data.domain = data.website.as_deref().and_then(domain_from_website);
        }
        None => {}
    }
    if let Some(value) = input.industry {
        data.industry = validator.text("industry", value, 140);
    }
    if let Some(value) = input.size_range {
        data.size_range = validator.text("size_range", value, 64);
        if data
            .size_range
            .as_deref()
            .is_some_and(|size| !is_company_size_range(size))
        {
            validator.violation("size_range", "is not a valid option");
            data.size_range = None;
        }
    }
    if let Some(value) = input.phone {
        data.phone = validator.phone("phone", value);
    }
    if let Some(value) = input.email {
        data.email = validator.email("email", value);
    }
    if let Some(value) = input.owner_id {
        data.owner_id = validator.user("owner_id", value);
    }
    validator.address(&mut data.address, input.address);
    if let Some(value) = input.description {
        data.description = validator.text("description", value, MAX_LONG_TEXT_LENGTH);
    }
    if let Some(value) = input.source {
        data.source = validator.text("source", value, 140);
    }
    if let Some(value) = input.parent_company_id {
        data.parent_company_id = validator.record_id("parent_company_id", value);
    }
    data
}
