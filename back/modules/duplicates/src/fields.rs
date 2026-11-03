use aphel_kernel::records::Address;
use organizations::domain::CompanyData;
use people::domain::ContactData;

pub(crate) const CONTACT_MERGE_KEYS: &[&str] = &[
    "first_name",
    "last_name",
    "email",
    "phone",
    "mobile",
    "job_title",
    "company_id",
    "owner_id",
    "address_street",
    "address_number",
    "address_complement",
    "address_district",
    "address_city",
    "address_state",
    "address_postal_code",
    "address_country",
    "source",
    "notes",
];

pub(crate) const COMPANY_MERGE_KEYS: &[&str] = &[
    "name",
    "domain",
    "website",
    "industry",
    "size_range",
    "phone",
    "email",
    "owner_id",
    "address_street",
    "address_number",
    "address_complement",
    "address_district",
    "address_city",
    "address_state",
    "address_postal_code",
    "address_country",
    "description",
    "source",
];

fn address_slot<'a>(address: &'a mut Address, key: &str) -> Option<&'a mut Option<String>> {
    Some(match key {
        "address_street" => &mut address.street,
        "address_number" => &mut address.number,
        "address_complement" => &mut address.complement,
        "address_district" => &mut address.district,
        "address_city" => &mut address.city,
        "address_state" => &mut address.state,
        "address_postal_code" => &mut address.postal_code,
        "address_country" => &mut address.country,
        _ => return None,
    })
}

fn address_value<'a>(address: &'a Address, key: &str) -> Option<&'a Option<String>> {
    Some(match key {
        "address_street" => &address.street,
        "address_number" => &address.number,
        "address_complement" => &address.complement,
        "address_district" => &address.district,
        "address_city" => &address.city,
        "address_state" => &address.state,
        "address_postal_code" => &address.postal_code,
        "address_country" => &address.country,
        _ => return None,
    })
}

fn blank(value: &Option<String>) -> bool {
    value.as_deref().is_none_or(|text| text.trim().is_empty())
}

pub(crate) fn contact_field_is_empty(data: &ContactData, key: &str) -> bool {
    match key {
        "first_name" => data.first_name.trim().is_empty(),
        "last_name" => blank(&data.last_name),
        "email" => blank(&data.email),
        "phone" => blank(&data.phone),
        "mobile" => blank(&data.mobile),
        "job_title" => blank(&data.job_title),
        "company_id" => data.company_id.is_none(),
        "owner_id" => data.owner_id.is_none(),
        "source" => blank(&data.source),
        "notes" => blank(&data.notes),
        other => address_value(&data.address, other).is_none_or(blank),
    }
}

pub(crate) fn copy_contact_field(target: &mut ContactData, source: &ContactData, key: &str) {
    match key {
        "first_name" => target.first_name = source.first_name.clone(),
        "last_name" => target.last_name = source.last_name.clone(),
        "email" => target.email = source.email.clone(),
        "phone" => target.phone = source.phone.clone(),
        "mobile" => target.mobile = source.mobile.clone(),
        "job_title" => target.job_title = source.job_title.clone(),
        "company_id" => target.company_id = source.company_id,
        "owner_id" => target.owner_id = source.owner_id.clone(),
        "source" => target.source = source.source.clone(),
        "notes" => target.notes = source.notes.clone(),
        other => {
            if let (Some(slot), Some(value)) = (
                address_slot(&mut target.address, other),
                address_value(&source.address, other),
            ) {
                *slot = value.clone();
            }
        }
    }
}

pub(crate) fn company_field_is_empty(data: &CompanyData, key: &str) -> bool {
    match key {
        "name" => data.name.trim().is_empty(),
        "domain" => blank(&data.domain),
        "website" => blank(&data.website),
        "industry" => blank(&data.industry),
        "size_range" => blank(&data.size_range),
        "phone" => blank(&data.phone),
        "email" => blank(&data.email),
        "owner_id" => data.owner_id.is_none(),
        "description" => blank(&data.description),
        "source" => blank(&data.source),
        other => address_value(&data.address, other).is_none_or(blank),
    }
}

pub(crate) fn copy_company_field(target: &mut CompanyData, source: &CompanyData, key: &str) {
    match key {
        "name" => target.name = source.name.clone(),
        "domain" => target.domain = source.domain.clone(),
        "website" => target.website = source.website.clone(),
        "industry" => target.industry = source.industry.clone(),
        "size_range" => target.size_range = source.size_range.clone(),
        "phone" => target.phone = source.phone.clone(),
        "email" => target.email = source.email.clone(),
        "owner_id" => target.owner_id = source.owner_id.clone(),
        "description" => target.description = source.description.clone(),
        "source" => target.source = source.source.clone(),
        other => {
            if let (Some(slot), Some(value)) = (
                address_slot(&mut target.address, other),
                address_value(&source.address, other),
            ) {
                *slot = value.clone();
            }
        }
    }
}
